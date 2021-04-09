use hyper::{Request, Response, Body, StatusCode, HeaderMap, header};
use std::convert::Infallible;
use futures::TryStreamExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio::fs::File;
use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
use uuid::Uuid;
use crate::alias;
use sqlx::{SqlitePool, Connection};
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
use crate::upload::error::{Error as UploadError, Error};
use std::borrow::Cow::Owned;

pub mod limit;
pub mod origin;
pub mod expiration;
pub mod error;

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

impl From<UploadInfo> for UploadResponse<UploadInfo> {
    fn from(info: UploadInfo) -> Self {
        Self {
            success: true,
            data: info,
        }
    }
}

impl From<UploadError> for UploadResponse<UploadError> {
    fn from(err: Error) -> Self {
        Self {
            success: false,
            data: err
        }
    }
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

async fn process_upload(req: Request<Body>) -> Result<UploadInfo, UploadError> {
    let id = Uuid::new_v4().to_hyphenated_ref().to_string();
    let name = parse_filename_header(req.headers())?;
    let size = parse_file_size(req.headers())?;
    let duration = DEV_EXPIRATION_DETERMINER.determine(size).ok_or(UploadError::TooLarge)?;
    let expiration = SystemTime::now() + duration;
    let expiration_timestamp = expiration.duration_since(UNIX_EPOCH).map_err(|_| UploadError::TimeCalculation)?.as_secs();
    let (short, long) = alias::random_aliases().ok_or(UploadError::AliasGeneration)?;
    let origin = origin::real_ip(&req).ok_or(UploadError::Origin)?.to_string();
    let link_base = origin::upload_base(req.headers()).ok_or(UploadError::Target)?;
    dbg!(&id, &name, size, duration, expiration, &short, &long, &origin, &link_base);

    let mut conn = req.data::<SqlitePool>()
        .ok_or(UploadError::Database)?
        .acquire().await
        .map_err(|_| UploadError::Database)?;

    let limiter = req.data::<IpLimiter>().ok_or(UploadError::QuotaAccess)?;
    if !limiter.accept(&req, size, &mut conn).await {
        return Err(UploadError::QuotaExceeded);
    }

    sqlx::query(include_query!("insert_file"))
        .bind(&id)
        .bind(&name)
        .bind(size as i64)
        .bind(expiration_timestamp as i64)
        .bind(&short)
        .bind(&long)
        .bind(&origin)
        .execute(&mut conn).await.map_err(|_| UploadError::Database)?;
    drop(conn);

    let (_, body) = req.into_parts();
    let mut ar = body
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        .into_async_read()
        .compat();

    let mut file = File::create(format!("uploads/{}", id)).await.map_err(|_| UploadError::CreateFile)?;
    tokio::io::copy(&mut ar, &mut file).await.map_err(|_| UploadError::CopyFile)?;

    Ok(
        UploadInfo {
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
    )
}

pub async fn upload_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok (
        match process_upload(req).await {
            Ok(info) => {
                Response::builder()
                    .status(StatusCode::CREATED)
                    .header(CONTENT_TYPE, "application/json")
                    .body(serde_json::to_string(&UploadResponse::from(info)).unwrap().into())
            },
            Err(err) => {
                Response::builder()
                    .status(err.status_code())
                    .header(CONTENT_TYPE, "application/json")
                    .body(serde_json::to_string(&UploadResponse::from(err)).unwrap().into())
            }
        }.unwrap()
    )
}

fn parse_filename_header(headers: &HeaderMap) -> Result<String, UploadError> {
    headers.get("X-Filename")
        .ok_or(UploadError::FilenameHeader)?
        .to_str()
        .map(ToOwned::to_owned)
        .map_err(|_| UploadError::FilenameHeader)
}

fn parse_file_size(headers: &HeaderMap) -> Result<u64, UploadError> {
    headers.get(header::CONTENT_LENGTH)
        .ok_or(UploadError::ContentLength)?
        .to_str()
        .map_err(|_| UploadError::ContentLength)?
        .parse::<u64>()
        .map_err(|_| UploadError::ContentLength)
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