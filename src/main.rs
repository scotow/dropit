mod alias;
mod download;
mod upload;
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

async fn home_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(
        Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(include_str!("public/index.html")))
            .unwrap()
    )
}

async fn archive(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let (w, r) = io::duplex(1024);

    tokio::spawn(async move {
        let mut ar = Builder::new(w.compat());
        ar.mode(HeaderMode::Deterministic);

        let mut header = Header::new_gnu();
        header.set_path("README.md").unwrap();
        header.set_mode(0o400);
        header.set_size(12);
        header.set_cksum();
        ar.append(&mut header, &b"Hello, World"[..]).await.unwrap();

        ar.finish().await.unwrap();
    });

    Ok(Response::new(Body::wrap_stream(ReaderStream::new(r))))
}

async fn logger(req: Request<Body>) -> Result<Request<Body>, Infallible> {
    println!("{} {} {}", req.remote_addr(), req.method(), req.uri().path());
    Ok(req)
}

async fn remove_powered_header(mut res: Response<Body>) -> Result<Response<Body>, Infallible> {
    res.headers_mut().remove("x-powered-by");
    Ok(res)
}

async fn router() -> Router<Body, Infallible> {
    let pool = SqlitePool::connect("database.db").await.unwrap();
    Router::builder()
        .data(pool)
        .middleware(Middleware::pre(logger))
        .middleware(Middleware::post(remove_powered_header))
        .get("/", home_handler)
        .get("/index.html", home_handler)
        // .get("/about", about_handler)
        .get("/:alias", download::download_handler)
        // .get("/archive", archive)
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

    let uploads_dir = "uploads";
    if let Err(e) = File::open(uploads_dir).await {
        if e.kind() == ErrorKind::NotFound {
            tokio::fs::create_dir_all(uploads_dir).await.unwrap();
        }
    }

    let router = router().await;

    // Create a Service from the router above to handle incoming requests.
    let service = RouterService::new(router).unwrap();

    // The address on which the server will be listening.
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));

    // Create a server by passing the created service to `.serve` method.
    let server = Server::bind(&addr).serve(service);

    println!("App is running on: {}", addr);
    if let Err(err) = server.await {
        eprintln!("Server error: {}", err);
    }
}