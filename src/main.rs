mod alias;

use hyper::{Body, Request, Response, Server};
use routerify::prelude::*;
use routerify::{Middleware, Router, RouterService, ext::RequestExt};
use std::{convert::Infallible, net::SocketAddr};
use futures::StreamExt;
use futures::TryStreamExt;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use sqlx::{SqlitePool, Sqlite, Pool};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};
use tokio_util::codec::{BytesCodec, FramedRead};
use async_tar::{Builder, Header, HeaderMode};
use tokio::io;
use tokio_util::io::ReaderStream;


// A handler for "/" page.
async fn home_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut conn = req.data::<Pool<Sqlite>>().unwrap()
        .acquire().await.unwrap();
    let res = sqlx::query!("SELECT id FROM Files")
        .fetch_all(&mut conn).await;
    // dbg!(res);

    Ok(Response::new(Body::from("Home page")))
}

// A handler for "/about" page.
async fn about_handler(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("About page")))
}

async fn upload(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let (head, body) = req.into_parts();
    let mut ar = body
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        .into_async_read()
        .compat();

    dbg!(head.headers.get("X-Filename").unwrap());

    let filename = head.headers.get("X-Filename").unwrap().to_str().unwrap();
    let mut file = File::create(format!("uploads/{}", filename)).await.unwrap();
    tokio::io::copy(&mut ar, &mut file).await.unwrap();

    // body.fold(file, |mut f, chunk| async move {
    //     let chunk = chunk.unwrap();
    //     println!("{:?}", chunk.len());
    //     f.write_all(&chunk).await.unwrap();
    //     f
    // }).await;
    // let fe = body.for_each(|chunk| {
    //     println!("{}", chunk.unwrap().len());
    //     futures::future::ready(())
    // });
    // fe.await;
    // let fe = body.map_ok(|chunk| {
    //     chunk.iter()
    //         .map(|byte| byte.to_ascii_uppercase())
    //         .collect::<Vec<u8>>()
    // });

    Ok(Response::new(Body::from("Upload page")))
}

async fn archive(req: Request<Body>) -> Result<Response<Body>, Infallible> {
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

// A middleware which logs an http request.
async fn logger(req: Request<Body>) -> Result<Request<Body>, Infallible> {
    println!("{} {} {}", req.remote_addr(), req.method(), req.uri().path());
    Ok(req)
}

async fn router() -> Router<Body, Infallible> {
    let pool = SqlitePool::connect("database.db").await.unwrap();

    // Create a router and specify the logger middleware and the handlers.
    // Here, "Middleware::pre" means we're adding a pre middleware which will be executed
    // before any route handlers.
    Router::builder()
        .data(pool)
        .middleware(Middleware::pre(logger))
        .get("/", home_handler)
        .get("/about", about_handler)
        .get("/archive", archive)
        .post("/upload", upload)
        .build()
        .unwrap()
}

#[tokio::main]
async fn main() {
    // let pool = SqlitePool::connect("database.db").await.unwrap();
    // let mut conn = pool.acquire().await.unwrap();
    // let id = sqlx::query!("INSERT INTO files (id) values (44)")
    //     .execute(&mut conn).await.unwrap()
    //     .last_insert_rowid();
    //
    // print_type_of(&id);
    // println!("{:?}", id);

    for i in 1..10 {
        println!("{} {}", alias::short::random().unwrap(), alias::long::random().unwrap());
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