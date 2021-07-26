use std::{convert::Infallible, net::SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use hyper::{Body, header, Request, Response, Server, StatusCode};
use routerify::{ext::RequestExt, Middleware, Router, RouterService};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use structopt::StructOpt;
use tokio::fs::File;
use tokio::io::ErrorKind;

use option::Options;

use crate::asset::Assets;
use crate::limit::Chain as LimiterChain;
use crate::limit::global::Global;
use crate::limit::ip::Ip as IpLimiter;
use crate::misc::generic_500;
use crate::storage::clean::Cleaner;
use crate::storage::dir::Dir;
use crate::upload::expiration::Determiner;
use crate::upload::origin::RealIp;

mod alias;
mod download;
mod upload;
mod update;
mod error;
mod storage;
mod query;
mod option;
mod limit;
mod asset;
mod misc;

async fn logger(req: Request<Body>) -> Result<Request<Body>, Infallible> {
    log::info!("{} {} {}", req.remote_addr(), req.method(), req.uri().path());
    Ok(req)
}

async fn remove_powered_header(mut res: Response<Body>) -> Result<Response<Body>, Infallible> {
    res.headers_mut().remove("x-powered-by");
    Ok(res)
}

async fn asset_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let assets = match req.data::<Assets>() {
        Some(assets) => assets,
        None => return Ok(generic_500()),
    };
    match assets.asset_for_path(req.uri().path()).await {
        Some((content, mime_type)) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime_type)
                .body(Body::from(content))
        }
        None => {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
        }
    }.or_else(|_| Ok(generic_500()))
}

fn router(
    uploads_dir: PathBuf,
    real_ip: RealIp,
    limiters: LimiterChain,
    determiner: Determiner,
    pool: SqlitePool,
    assets: Assets,
) -> Router<Body, Infallible> {
    Router::builder()
        .data(Dir::new(uploads_dir))
        .data(real_ip)
        .data(limiters)
        .data(determiner)
        .data(pool)
        .data(assets)
        .middleware(Middleware::pre(logger))
        .middleware(Middleware::post(remove_powered_header))
        .get("/", asset_handler)
        .get("/index.html", asset_handler)
        .get("/style.css", asset_handler)
        .get("/app.js", asset_handler)
        .get("/:alias", download::handler)
        .post("/", upload::handler)
        .post("/upload", upload::handler)
        .delete("/:alias", update::revoke::handler)
        .patch("/:alias/aliases", update::alias::handler)
        .patch("/:alias/expiration", update::expiration::handler)
        .build()
        .unwrap_or_else(|_| exit_error!("Cannot create HTTP router"))
}

async fn create_uploads_dir(path: &Path, should_create: bool) {
    match File::open(&path).await {
        Ok(fd) => {
            match fd.metadata().await {
                Ok(md) => if !md.is_dir() {
                    exit_error!("Uploads path is not a directory");
                },
                Err(_) => {
                    exit_error!("Cannot fetch uploads dir metadata");
                }
            }
        }
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                if should_create {
                    if let Err(err) = tokio::fs::create_dir_all(&path).await {
                        exit_error!("Cannot create uploads directory: {}", err);
                    }
                } else {
                    exit_error!("Uploads directory not found");
                }
            } else {
                exit_error!("Cannot open uploads directory");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let options = Options::from_args();
    env_logger::Builder::new().filter_level(options.log_level).init();

    let limiters = LimiterChain::new(vec![
        Box::new(IpLimiter::new(options.ip_size_sum, options.ip_file_count)),
        Box::new(Global::new(options.global_size_sum)),
    ]);
    let determiner = Determiner::new(options.thresholds)
        .unwrap_or_else(|| exit_error!("Invalid thresholds"));

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&options.database)
                .create_if_missing(!options.no_database_creation)
                .busy_timeout(Duration::from_secs(30))
        ).await
        .unwrap_or_else(|e| exit_error!("Cannot create database pool: {}", e));
    sqlx::query(include_query!("migration"))
        .execute(&pool).await
        .unwrap_or_else(|e| exit_error!("Cannot run migration query: {}", e));

    create_uploads_dir(&options.uploads_dir, !options.no_uploads_dir_creation).await;
    let cleaner = Cleaner::new(&options.uploads_dir, pool.clone());
    tokio::task::spawn(async move {
        cleaner.start().await;
    });

    let router = router(
        options.uploads_dir,
        RealIp::new(options.behind_proxy),
        limiters,
        determiner,
        pool,
        Assets::new(options.color),
    );

    let address = SocketAddr::new(options.address, options.port);
    let service = RouterService::new(router)
        .unwrap_or_else(|e| exit_error!("Cannot create HTTP service: {}", e));

    log::info!("App is running on: {}", address);
    Server::bind(&address)
        .serve(service).await
        .unwrap_or_else(|e| exit_error!("Server stopped: {}", e))
}