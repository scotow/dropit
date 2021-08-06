use std::convert::{Infallible, TryFrom};
use std::net::IpAddr;
use std::path::Path;

use futures::StreamExt;
use hyper::{Body, header, HeaderMap, Request, Response, StatusCode};
use percent_encoding::percent_decode_str;
use routerify::ext::RequestExt;
use sanitize_filename::sanitize;
use sqlx::SqlitePool;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::alias;
use crate::error::Error;
use crate::error::upload as UploadError;
use crate::include_query;
use crate::limit::Chain as ChainLimiter;
use crate::limit::Limiter;
use crate::misc::generic_500;
use crate::misc::request_target;
use crate::response::{json_response, text_response};
use crate::storage::dir::Dir;
use crate::upload::expiration::Determiner;
use crate::upload::file::{Expiration, UploadInfo};
use crate::upload::origin::RealIp;

pub mod origin;
pub mod expiration;
pub mod file;

pub type UploadResult<T> = Result<T, Error>;

#[allow(unused)]
pub struct UploadRequest {
    pub name: Option<String>,
    pub size: u64,
    pub origin: IpAddr,
}

#[allow(clippy::wildcard_in_or_patterns)]
pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let response_type = req.headers().get(header::ACCEPT).map(|h| h.as_bytes().to_vec());
    let upload_res = process_upload(req).await;
    match response_type.as_deref() {
        Some(b"text/plain") => text_response(StatusCode::CREATED, upload_res.map(|i| i.short_link())),
        Some(b"application/json") | _ => json_response(StatusCode::CREATED, upload_res),
    }.or_else(|_| Ok(generic_500()))
}

#[allow(clippy::bool_comparison)]
async fn process_upload(req: Request<Body>) -> UploadResult<UploadInfo> {
    let id = Uuid::new_v4().to_hyphenated_ref().to_string();
    let admin = Uuid::new_v4().to_hyphenated_ref().to_string();
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
    let link_base = request_target(req.headers()).ok_or(UploadError::Target)?;

    // Expiration.
    let determiner = req.data::<Determiner>().ok_or(UploadError::TimeCalculation)?;
    let expiration = Expiration::try_from(
        determiner.determine(upload_req.size).ok_or(UploadError::TooLarge)?
    )?;

    sqlx::query(include_query!("insert_file"))
        .bind(&id)
        .bind(&admin)
        .bind(upload_req.origin.to_string())
        .bind(expiration.timestamp() as i64)
        .bind(&upload_req.name)
        .bind(upload_req.size as i64)
        .bind(&short)
        .bind(&long)
        .execute(&mut conn).await.map_err(|_| UploadError::Database)?;
    drop(conn);

    // Copy body to file system.
    let path = req.data::<Dir>().ok_or(UploadError::PathResolve)?.file_path(&id);
    let file = File::create(&path).await.map_err(|_| UploadError::CreateFile)?;
    match write_file(&upload_req, req.into_body(), file).await {
        Ok(_) => (),
        Err(err) => {
            clean_failed_upload(path.as_path(), &id, &pool).await;
            return Err(err)
        }
    }

    Ok(
        UploadInfo::new(
            admin,
            upload_req.name.unwrap_or_else(|| long.clone()),
            upload_req.size,
            (short, long),
            link_base,
            expiration,
        )
    )
}

fn parse_filename_header(headers: &HeaderMap) -> UploadResult<Option<String>> {
    if let Some(header) = headers.get("X-Filename") {
        Ok(Some(
            sanitize(
                percent_decode_str(
                    std::str::from_utf8(header.as_bytes()).map_err(|_| UploadError::Database)?
                ).decode_utf8().map_err(|_| UploadError::FilenameHeader)?
            )
        ))
    } else {
        Ok(None)
    }
}

fn parse_file_size(headers: &HeaderMap) -> UploadResult<u64> {
    headers.get(header::CONTENT_LENGTH)
        .ok_or(UploadError::ContentLength)?
        .to_str()
        .map_err(|_| UploadError::ContentLength)?
        .parse::<u64>()
        .map_err(|_| UploadError::ContentLength)
}

async fn write_file(req: &UploadRequest, mut body: Body, mut file: File) -> UploadResult<()> {
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
    if let Err(err) = tokio::fs::remove_file(file_path).await {
        log::error!("Cannot remove file with id {} from file system, file will retain quota: {}", id, err);
        return;
    }
    if let Err(err) = sqlx::query(include_query!("delete_file"))
        .bind(&id)
        .execute(pool).await {
        log::error!("Cannot remove file with id {} from database: {:?}", id, err);
    }
}