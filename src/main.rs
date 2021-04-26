mod alias;
mod download;
mod upload;
mod storage;
mod query;
mod option;
mod limit;
mod asset;

use hyper::{Body, Request, Response, Server, StatusCode, header};
use routerify::{Middleware, Router, RouterService, ext::RequestExt};
use std::{convert::Infallible, net::SocketAddr};
use tokio::fs::File;
use tokio::io::ErrorKind;
use sqlx::SqlitePool;
use std::time::Duration;
use crate::storage::clean::Cleaner;
use crate::limit::ip::Ip as IpLimiter;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use crate::storage::dir::Dir;
use std::path::PathBuf;
use option::Options;
use structopt::StructOpt;
use crate::upload::expiration::Determiner;
use crate::upload::origin::RealIp;
use crate::limit::Chain as LimiterChain;
use crate::limit::global::Global;
use crate::asset::Assets;

async fn logger(req: Request<Body>) -> Result<Request<Body>, Infallible> {
    println!("{} {} {}", req.remote_addr(), req.method(), req.uri().path());
    Ok(req)
}

async fn remove_powered_header(mut res: Response<Body>) -> Result<Response<Body>, Infallible> {
    res.headers_mut().remove("x-powered-by");
    Ok(res)
}

async fn asset_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let assets = req.data::<Assets>().unwrap();
    Ok (
        match assets.asset_for_path(req.uri().path()) {
            Some((content, mime_type)) => {
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, mime_type)
                    .body(Body::from(content))
                    .unwrap()
            }
            None => {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap()
            }
        }
    )
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
        .get("/:alias", download::file::handler)
        .post("/", upload::handler)
        .post("/upload", upload::handler)
        .build()
        .unwrap()
}

#[tokio::main]
async fn main() {
    let Options {
        uploads_dir,
        address,
        port,
        behind_proxy,
        thresholds,
        ip_size_sum, ip_file_count,
        global_size_sum,
        color,
    } = Options::from_args();

    let limiters = LimiterChain::new(vec![
        Box::new(IpLimiter::new(ip_size_sum, ip_file_count)),
        Box::new(Global::new(global_size_sum)),
    ]);
    let determiner = Determiner::new(thresholds).expect("invalid thresholds");

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename("database.db")
                .create_if_missing(true)
                .busy_timeout(Duration::from_secs(30))
        ).await.unwrap();
    sqlx::query(include_query!("migration")).execute(&pool).await.unwrap();

    if let Err(e) = File::open(&uploads_dir).await {
        if e.kind() == ErrorKind::NotFound {
            tokio::fs::create_dir_all(&uploads_dir).await.unwrap();
        }
    }
    let cleaner = Cleaner::new(&uploads_dir, pool.clone());
    tokio::task::spawn(async move {
        cleaner.start().await;
    });

    let router = router(
        uploads_dir,
        RealIp::new(behind_proxy),
        limiters,
        determiner,
        pool,
        Assets::new(&color),
    );

    let address = SocketAddr::new(address, port);
    let service = RouterService::new(router).unwrap();
    let server = Server::bind(&address).serve(service);

    println!("App is running on: {}", address);
    if let Err(err) = server.await {
        eprintln!("Server error: {}", err);
    }
}