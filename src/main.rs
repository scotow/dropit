mod alias;
mod download;
mod upload;
mod storage;
mod query;
mod option;

use hyper::{Body, Request, Response, Server, StatusCode, header};
use routerify::{Middleware, Router, RouterService, ext::RequestExt};
use std::{convert::Infallible, net::SocketAddr};
use tokio::fs::File;
use tokio::io::ErrorKind;
use sqlx::SqlitePool;
use std::time::Duration;
use crate::storage::clean::Cleaner;
use crate::upload::limit::IpLimiter;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use crate::storage::dir::Dir;
use std::path::PathBuf;
use option::Options;
use structopt::StructOpt;
use crate::upload::expiration::Determiner;
use crate::upload::origin::RealIp;

async fn logger(req: Request<Body>) -> Result<Request<Body>, Infallible> {
    println!("{} {} {}", req.remote_addr(), req.method(), req.uri().path());
    Ok(req)
}

async fn remove_powered_header(mut res: Response<Body>) -> Result<Response<Body>, Infallible> {
    res.headers_mut().remove("x-powered-by");
    Ok(res)
}

async fn asset_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let (content, mime) = match req.uri().path() {
        "/" | "/index.html" => (include_str!("public/index.html"), "text/html"),
        "/style.css" => (include_str!("public/style.css"), "text/css"),
        "/app.js" => (include_str!("public/app.js"), "application/javascript"),
        _ => unreachable!(),
    };
    Ok(
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .body(Body::from(content))
            .unwrap()
    )
}

fn router(
    uploads_dir: PathBuf,
    real_ip: RealIp,
    limiter: IpLimiter,
    determiner: Determiner,
    pool: SqlitePool
) -> Router<Body, Infallible> {
    Router::builder()
        .data(Dir::new(uploads_dir))
        .data(real_ip)
        .data(limiter)
        .data(determiner)
        .data(pool)
        .middleware(Middleware::pre(logger))
        .middleware(Middleware::post(remove_powered_header))
        .get("/", asset_handler)
        .get("/index.html", asset_handler)
        .get("/style.css", asset_handler)
        .get("/app.js", asset_handler)
        .get("/:alias", download::file::download_handler)
        .post("/", upload::handler::upload)
        .post("/upload", upload::handler::upload)
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
    } = Options::from_args();

    let limiter = IpLimiter::new(ip_size_sum, ip_file_count);
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
        limiter,
        determiner,
        pool
    );

    let address = SocketAddr::new(address, port);
    let service = RouterService::new(router).unwrap();
    let server = Server::bind(&address).serve(service);

    println!("App is running on: {}", address);
    if let Err(err) = server.await {
        eprintln!("Server error: {}", err);
    }
}