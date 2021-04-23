pub mod origin;
pub mod expiration;
pub mod error;
pub mod file;

use hyper::{Request, Response, Body, StatusCode, HeaderMap, header};
use std::convert::{Infallible, TryFrom};
use futures::StreamExt;
use tokio::fs::File;
use uuid::Uuid;
use crate::alias;
use sqlx::SqlitePool;
use routerify::ext::RequestExt;
use crate::include_query;
use serde::Serialize;
use crate::upload::expiration::Determiner;
use crate::upload::error::{Error as UploadError};
use crate::upload::origin::{upload_base, RealIp};
use crate::upload::file::{UploadInfo, Expiration};
use tokio::io::AsyncWriteExt;
use std::path::Path;
use crate::storage::dir::Dir;
use std::net::IpAddr;
use crate::limit::Chain as ChainLimiter;
use crate::limit::Limiter;

#[allow(unused)]
pub struct UploadRequest {
    pub name: String,
    pub size: u64,
    pub origin: IpAddr,
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

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok (
        match process_upload(req).await {
            Ok(info) => {
                Response::builder()
                    .status(StatusCode::CREATED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(serde_json::to_string(&UploadResponse::from(info)).unwrap().into())
            },
            Err(err) => {
                Response::builder()
                    .status(err.status_code())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(serde_json::to_string(&UploadResponse::from(err)).unwrap().into())
            }
        }.unwrap() // How to remove this unwrap? Fallback to a generic 500.
    )
}

async fn process_upload(req: Request<Body>) -> Result<UploadInfo, UploadError> {
    let id = Uuid::new_v4().to_hyphenated_ref().to_string();
    let upload_req = UploadRequest {
        name: parse_filename_header(req.headers())?,
        size: parse_file_size(req.headers())?,
        origin: req.data::<RealIp>().ok_or(UploadError::Origin)?
            .find(&req).ok_or(UploadError::Origin)?,
    };

    let pool = req.data::<SqlitePool>().ok_or(UploadError::Database)?.clone();
    let mut conn = pool.acquire().await.map_err(|_| UploadError::Database)?;

    // Quota.
    if req.data::<ChainLimiter>().ok_or(UploadError::QuotaAccess)?
        .accept(&upload_req, &mut conn).await
        .ok_or(UploadError::QuotaAccess)? == false {
        return Err(UploadError::QuotaExceeded);
    }

    // Aliases and links.
    let (short, long) = alias::random_unused_aliases(&mut conn).await
        .ok_or(UploadError::AliasGeneration)?;
    let link_base = upload_base(req.headers()).ok_or(UploadError::Target)?;

    // Expiration.
    let determiner = req.data::<Determiner>().ok_or(UploadError::TimeCalculation)?;
    let expiration = Expiration::try_from(
        determiner.determine(upload_req.size).ok_or(UploadError::TooLarge)?
    )?;

    sqlx::query(include_query!("insert_file"))
        .bind(&id)
        .bind(&upload_req.name)
        .bind(*&upload_req.size as i64)
        .bind(expiration.timestamp() as i64)
        .bind(&short)
        .bind(&long)
        .bind(upload_req.origin.to_string())
        .execute(&mut conn).await.map_err(|_| UploadError::Database)?;
    drop(conn);

    // Copy body to file system.
    let path = req.data::<Dir>().ok_or(UploadError::CreateFile)?.file_path(&id);
    let file = File::create(&path).await.map_err(|_| UploadError::CreateFile)?;
    match write_file(&upload_req, req.into_body(), file).await {
        Ok(_) => (),
        Err(err) => {
            clean_failed_upload(path.as_path(), &id, &pool).await;
            return Err(err)
        }
    }

    let UploadRequest { name, size, ..} = upload_req;
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

async fn write_file(req: &UploadRequest, mut body: Body, mut file: File) -> Result<(), UploadError> {
    let mut written = 0;
    while let Some(chunk) = body.next().await {
        let data = chunk.map_err(|_| UploadError::CopyFile)?;

        if written + data.len() as u64 > req.size {
            return Err(UploadError::SizeMismatch);
        }
        written += data.len() as u64;

        if file.write_all(&data).await.is_err() {
            return Err(UploadError::CopyFile);
        }
    }
    // Check difference just in case, but inferior check should be enough.
    if written != req.size {
        return Err(UploadError::SizeMismatch);
    }

    Ok(())
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