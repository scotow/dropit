mod alias;
mod download;
mod upload;
mod storage;
mod query;

use hyper::{Body, Request, Response, Server, StatusCode};
use routerify::prelude::*;
use routerify::{Middleware, Router, RouterService, ext::RequestExt};
use std::{convert::Infallible, net::SocketAddr};
use futures::StreamExt;
use futures::TryStreamExt;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, ErrorKind};
use sqlx::{SqlitePool, Sqlite, Pool};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
use tokio_util::codec::{BytesCodec, FramedRead};
use async_tar::{Builder, Header, HeaderMode};
use tokio::io;
use tokio_util::io::ReaderStream;
use bytesize::ByteSize;
use std::time::Duration;
use crate::storage::clean::Cleaner;
use crate::upload::limit::IpLimiter;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::pool::PoolOptions;
use crate::storage::dir::Dir;

async fn logger(req: Request<Body>) -> Result<Request<Body>, Infallible> {
    println!("{} {} {}", req.remote_addr(), req.method(), req.uri().path());
    Ok(req)
}

async fn remove_powered_header(mut res: Response<Body>) -> Result<Response<Body>, Infallible> {
    res.headers_mut().remove("x-powered-by");
    Ok(res)
}

async fn index_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(
        Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(include_str!("public/index.html")))
            .unwrap()
    )
}

async fn router(pool: SqlitePool) -> Router<Body, Infallible> {
    Router::builder()
        .data(IpLimiter::new(512 * 1024 * 1024, 16))
        .data(Dir::new("uploads"))
        .data(pool)
        .middleware(Middleware::pre(logger))
        .middleware(Middleware::post(remove_powered_header))
        .get("/", index_handler)
        .get("/index.html", index_handler)
        .get("/:alias", download::file::download_handler)
        .post("/", upload::handler::upload)
        .post("/upload", upload::handler::upload)
        .build()
        .unwrap()
}

#[tokio::main]
async fn main() {
    let uploads_dir = "uploads";
    if let Err(e) = File::open(uploads_dir).await {
        if e.kind() == ErrorKind::NotFound {
            tokio::fs::create_dir_all(uploads_dir).await.unwrap();
        }
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename("database.db")
                .create_if_missing(true)
                .busy_timeout(Duration::from_secs(30))
        ).await.unwrap();
    sqlx::query(include_query!("migration")).execute(&pool).await.unwrap();

    let cleaner = Cleaner::new("uploads".into(), pool.clone());
    tokio::task::spawn(async move {
        cleaner.start().await;
    });

    let address = SocketAddr::from(([127, 0, 0, 1], 3001));
    let router = router(pool).await;
    let service = RouterService::new(router).unwrap();
    let server = Server::bind(&address).serve(service);

    println!("App is running on: {}", address);
    if let Err(err) = server.await {
        eprintln!("Server error: {}", err);
    }
}