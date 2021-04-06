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
use serde_json::json;
use bytesize::ByteSize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rand::Rng;
use lazy_static::lazy_static;
use crate::upload::expiration::{Determiner, Threshold};
use crate::upload::limit::{IpLimiter, Limiter};

pub mod limit;
pub mod origin;
pub mod expiration;

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
    duration: ExpirationDuration,
    date: ExpirationDate,
}

#[derive(Serialize)]
struct ExpirationDuration {
    seconds: u64,
    readable: String,
}

#[derive(Serialize)]
struct ExpirationDate {
    timestamp: u64,
    readable: String,
}

lazy_static! {
    static ref DEFAULT_EXPIRATION_DETERMINER: Determiner = Determiner::new(
        vec![
            Threshold { size: 64 * 1024 * 1024, duration: Duration::from_secs(24 * 60 * 60) },
            Threshold { size: 256 * 1024 * 1024, duration: Duration::from_secs(6 * 60 * 60) }
        ]
    ).unwrap();
    static ref DEV_EXPIRATION_DETERMINER: Determiner = Determiner::new(
        vec![
            Threshold { size: 1024 * 1024 * 1024, duration: Duration::from_secs(30) },
        ]
    ).unwrap();
}

pub async fn upload_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let id = Uuid::new_v4().to_hyphenated_ref().to_string();
    let name = req.headers().get("X-Filename").unwrap().to_str().unwrap().to_owned();
    let size = req.headers().get(CONTENT_LENGTH).unwrap().to_str().unwrap().parse::<u64>().unwrap();
    let duration = DEV_EXPIRATION_DETERMINER.determine(size).unwrap();
    let expiration = SystemTime::now() + duration;
    let expiration_timestamp = expiration.duration_since(UNIX_EPOCH).unwrap().as_secs();
    let (short, long) = alias::random_aliases().unwrap();
    let origin = origin::real_ip(&req).unwrap().to_string();
    dbg!(&id, &name, size, duration, expiration, &short, &long, &origin);

    let mut conn = req.data::<SqlitePool>().unwrap().acquire().await.unwrap();
    let limiter = req.data::<IpLimiter>().unwrap();

    if !limiter.accept(&req, size).await {
        return Ok(
            Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header(CONTENT_TYPE, "application/json")
                .body(
                    serde_json::to_string(
                        &UploadResponse {
                            success: false,
                            data: json!({
                                "error": "too many uploads"
                            }),
                        }
                    ).unwrap().into()
                ).unwrap()
        )
    }

    sqlx::query(include_query!("insert_file"))
        .bind(&id)
        .bind(&name)
        .bind(size as i64)
        .bind(expiration_timestamp as i64)
        .bind(&short)
        .bind(&long)
        .bind(&origin)
        .execute(&mut conn).await.unwrap();

    let (head, body) = req.into_parts();
    let mut ar = body
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        .into_async_read()
        .compat();

    let mut file = File::create(format!("uploads/{}", id)).await.unwrap();
    tokio::io::copy(&mut ar, &mut file).await.unwrap();

    let link_base = origin::upload_base(&head.headers).unwrap();
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
                duration: ExpirationDuration {
                    seconds: duration.as_secs(),
                    readable: humantime::format_duration(duration).to_string().replace(' ', ""),
                },
                date: ExpirationDate {
                    timestamp: expiration_timestamp,
                    readable: {
                        let mut full = humantime::format_rfc3339_seconds(expiration).to_string();
                        full.truncate(full.len() - 4);
                        full.replace('T', " ").replace('-', "/")
                    },
                },
            },
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