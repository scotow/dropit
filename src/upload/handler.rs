use hyper::{Request, Response, Body, StatusCode, HeaderMap, header};
use std::convert::{Infallible, TryFrom};
use futures::{TryStreamExt, StreamExt};
use tokio::fs::File;
use uuid::Uuid;
use crate::alias;
use sqlx::SqlitePool;
use routerify::ext::RequestExt;
use crate::include_query;
use serde::Serialize;
use crate::upload::expiration::Determiner;
use crate::upload::limit::{IpLimiter, Limiter};
use crate::upload::error::{Error as UploadError};
use crate::upload::origin::{real_ip, upload_base};
use crate::upload::file::{UploadInfo, Expiration};
use tokio::io::AsyncWriteExt;
use std::path::Path;
use crate::storage::dir::Dir;

#[allow(unused)]
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
    fn from(err: UploadError) -> Self {
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

    // Expiration.
    let determiner = req.data::<Determiner>().ok_or(UploadError::TimeCalculation)?;
    let expiration = Expiration::try_from(
        determiner.determine(size).ok_or(UploadError::TooLarge)?
    )?;

    // Aliases and links.
    let (short, long) = alias::random_aliases().ok_or(UploadError::AliasGeneration)?;
    let origin = real_ip(&req).ok_or(UploadError::Origin)?.to_string();
    let link_base = upload_base(req.headers()).ok_or(UploadError::Target)?;

    let pool = req.data::<SqlitePool>().ok_or(UploadError::Database)?.clone();
    let mut conn = pool.acquire().await.map_err(|_| UploadError::Database)?;

    let limiter = req.data::<IpLimiter>().ok_or(UploadError::QuotaAccess)?;
    if !limiter.accept(&req, size, &mut conn).await {
        return Err(UploadError::QuotaExceeded);
    }

    let file_path = req.data::<Dir>().ok_or(UploadError::CreateFile)?.file_path(&id);

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

    let mut file = File::create(&file_path).await.map_err(|_| UploadError::CreateFile)?;
    let mut body = req.into_body().map_err(|_| UploadError::CopyFile);

    let mut written = 0;
    while let Some(chunk) = body.next().await {
        let data = match chunk {
            Ok(data) => data,
            Err(_) => {
                clean_failed_upload(file_path.as_path(), &id, &pool).await;
                return Err(UploadError::CopyFile);
            }
        };

        if written + data.len() as u64 > size {
            clean_failed_upload(file_path.as_path(), &id, &pool).await;
            return Err(UploadError::SizeMismatch);
        }
        written += data.len() as u64;

        if file.write_all(&data).await.is_err() {
            clean_failed_upload(file_path.as_path(), &id, &pool).await;
            return Err(UploadError::CopyFile);
        }
    }
    // Check difference just in case, but inferior check should be enough.
    if written != size {
        clean_failed_upload(file_path.as_path(), &id, &pool).await;
        return Err(UploadError::SizeMismatch);
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
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(serde_json::to_string(&UploadResponse::from(info)).unwrap().into())
            },
            Err(err) => {
                dbg!(&err);
                Response::builder()
                    .status(err.status_code())
                    .header(header::CONTENT_TYPE, "application/json")
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

async fn clean_failed_upload(file_path: &Path, id: &str, pool: &SqlitePool) {
    if tokio::fs::remove_file(file_path).await.is_err() {
        eprintln!("[UPLOAD] cannot remove file with id {}, file will retain quota", id);
        return;
    }
    if sqlx::query(include_query!("delete_file"))
        .bind(&id)
        .execute(pool).await.is_err() {
        eprintln!("[UPLOAD] cannot remove file with id {} from database", id);
    }
}