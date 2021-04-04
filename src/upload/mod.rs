use hyper::{Request, Response, Body, StatusCode};
use std::convert::Infallible;
use futures::TryStreamExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio::fs::File;
use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
use uuid::Uuid;
use crate::alias;
use sqlx::SqlitePool;
use routerify::ext::RequestExt;
use crate::include_query;
use serde::Serialize;
use bytesize::ByteSize;
use std::time::Duration;
use rand::Rng;

pub mod origin;

pub struct UploadRequest {
    name: String,
    size: u64,
}

#[derive(Serialize)]
pub struct UploadResponse<T: Serialize> {
    pub success: bool,
    #[serde(flatten)]
    pub data: T,
}

#[derive(Serialize)]
pub struct UploadInfo {
    name: String,
    size: Size,
    alias: Aliases,
    link: Links,
    expiration: Expiration,
}

#[derive(Serialize)]
struct Aliases {
    short: String,
    long: String,
}

#[derive(Serialize)]
struct Links {
    short: String,
    long: String,
}

#[derive(Serialize)]
struct Size {
    bytes: u64,
    readable: String,
}

#[derive(Serialize)]
struct Expiration {
    duration: u64,
    readable: String,
}

pub async fn upload_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut conn = req.data::<SqlitePool>().unwrap().acquire().await.unwrap();
    let (head, body) = req.into_parts();

    let id = Uuid::new_v4().to_hyphenated_ref().to_string();
    let name = head.headers.get("X-Filename").unwrap().to_str().unwrap();
    let size = head.headers.get(CONTENT_LENGTH).unwrap().to_str().unwrap().parse::<u64>().unwrap();
    let (short, long) = alias::random_aliases().unwrap();
    dbg!(&id, name, size, &short, &long);

    let mut ar = body
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        .into_async_read()
        .compat();

    let mut file = File::create(format!("uploads/{}", id)).await.unwrap();
    tokio::io::copy(&mut ar, &mut file).await.unwrap();

    sqlx::query(include_query!("insert_file"))
        .bind(&id)
        .bind(&name)
        .bind(size as i64)
        .bind(&short)
        .bind(&long)
        .execute(&mut conn).await.unwrap();

    let link_base = origin::upload_base(&head.headers).unwrap();
    let duration = rand::thread_rng().gen_range(2..=12);
    let resp = UploadResponse {
        success: true,
        data: UploadInfo {
            name: name.to_owned(),
            size: Size {
                bytes: size,
                readable: ByteSize::b(size).to_string().replace(' ', ""),
            },
            alias: Aliases {
                short: short.clone(),
                long: long.clone(),
            },
            link: Links {
                short: format!("{}/{}", link_base, &short),
                long: format!("{}/{}", link_base, &long),
            },
            expiration: Expiration {
                duration: duration*60*60,
                readable: humantime::format_duration(Duration::new(duration*60*60, 0)).to_string().replace(' ', ""),
            }
        }
    };
    let resp = serde_json::to_string(&resp).unwrap();

    Ok(
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/json")
            .body(resp.into())
            .unwrap()
    )
}

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