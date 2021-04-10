use hyper::{Request, Response, Body, StatusCode, HeaderMap, header};
use std::convert::{Infallible, TryFrom};
use futures::TryStreamExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio::fs::File;
use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
use uuid::Uuid;
use crate::alias;
use sqlx::{SqlitePool, Connection, SqliteConnection};
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
use crate::upload::origin::{real_ip, upload_base};
use crate::upload::file::{Size, UploadInfo, Expiration};
use sqlx::pool::PoolConnection;
use tokio::io::AsyncWriteExt;

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

async fn process_upload(req: Request<Body>) -> Result<UploadInfo, UploadError> {
    let id = Uuid::new_v4().to_hyphenated_ref().to_string();
    let name = parse_filename_header(req.headers())?;
    let size = parse_file_size(req.headers())?;
    let expiration = Expiration::try_from(
        DEV_EXPIRATION_DETERMINER.determine(size).ok_or(UploadError::TooLarge)?
    )?;
    let (short, long) = alias::random_aliases().ok_or(UploadError::AliasGeneration)?;
    let origin = real_ip(&req).ok_or(UploadError::Origin)?.to_string();
    let link_base = upload_base(req.headers()).ok_or(UploadError::Target)?;

    let pool = req.data::<SqlitePool>().ok_or(UploadError::Database)?.clone();
    let mut conn = pool.acquire().await.map_err(|_| UploadError::Database)?;

    let limiter = req.data::<IpLimiter>().ok_or(UploadError::QuotaAccess)?;
    if !limiter.accept(&req, size, &mut conn).await {
        return Err(UploadError::QuotaExceeded);
    }

    sqlx::query(include_query!("insert_file"))
        .bind(&id)
        .bind(&name)
        .bind(size as i64)
        .bind(expiration.timestamp() as i64)
        .bind(&short)
        .bind(&long)
        .bind(&origin)
        .execute(&mut conn).await.map_err(|_| UploadError::Database)?;
    drop(conn);

    let file_path = format!("uploads/{}", id);
    let mut file = File::create(&file_path).await.map_err(|_| UploadError::CreateFile)?;
    let mut body = req.into_body().map_err(|_| UploadError::CopyFile);

    let mut written = 0;
    while let Some(chunk) = body.try_next().await? {
        if written + chunk.len() as u64 > size {
            clean_failed_upload(&file_path, &id, &pool).await;
            return Err(UploadError::SizeMismatch);
        }

        if file.write_all(&chunk).await.is_err() {
            clean_failed_upload(&file_path, &id, &pool).await;
            return Err(UploadError::CopyFile);
        }
    }

    Ok(
        UploadInfo::new(
            name,
            size,
            (short, long),
            link_base,
            expiration
        )
    )
}

pub async fn upload(req: Request<Body>) -> Result<Response<Body>, Infallible> {
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
        }.unwrap() // How to remove this unwrap?
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

async fn clean_failed_upload(file_path: &str, id: &str, pool: &SqlitePool) {
    let _ = tokio::fs::remove_file(file_path).await;
    let _ = sqlx::query(include_query!("delete_file"))
        .bind(&id)
        .execute(pool).await;
}