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
        .data(IpLimiter::new(512 * 1024 * 1024, 16, pool.clone()))
        .data(pool)
        .middleware(Middleware::pre(logger))
        .middleware(Middleware::post(remove_powered_header))
        .get("/", index_handler)
        .get("/index.html", index_handler)
        .get("/:alias", download::file::download_handler)
        .post("/", upload::upload_handler)
        .post("/upload", upload::upload_handler)
        .build()
        .unwrap()
}

#[tokio::main]
async fn main() {
    for _ in 1..10 {
        println!("{} {}", alias::short::random().unwrap(), alias::long::random().unwrap());
    }
    println!("{}", ByteSize::b(5345));
    println!("{}", humantime::Duration::from(Duration::new(6*60*60 + 30, 0)));

    let uploads_dir = "uploads";
    if let Err(e) = File::open(uploads_dir).await {
        if e.kind() == ErrorKind::NotFound {
            tokio::fs::create_dir_all(uploads_dir).await.unwrap();
        }
    }

    let pool = SqlitePool::connect("database.db").await.unwrap();
    let cleaner = Cleaner::new(pool.clone());
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