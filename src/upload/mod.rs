use axum::extract::{BodyStream, ConnectInfo};
use axum::headers::authorization::Basic;
use axum::headers::{Authorization, ContentLength, Cookie};
use axum::response::IntoResponse;
use axum::{Extension, Router, TypedHeader};
use std::convert::TryFrom;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use futures::StreamExt;
use hyper::{header, Body, HeaderMap, Request, Response, StatusCode};
use percent_encoding::percent_decode_str;
// use routerify::ext::RequestExt;
use axum::routing::post;
use sanitize_filename::sanitize;
use sqlx::SqlitePool;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::auth::{AuthStatus, Authenticator, Features};
use crate::error::auth as AuthError;
use crate::error::upload as UploadError;
use crate::error::Error;
use crate::include_query;
use crate::limit::Chain as ChainLimiter;
use crate::limit::Limiter;
use crate::misc::request_target;
// use crate::response::adaptive_response;
use crate::storage::dir::Dir;
use crate::upload::expiration::Determiner;
use crate::upload::file::{Expiration, UploadInfo};
use crate::upload::origin::{DomainUri, ForwardedForHeader, RealIp};
// use crate::{alias, Origin};
use crate::alias;
use crate::auth::Origin;
use crate::response::{ApiResponse, ResponseType};
use crate::upload::filename::Filename;

pub mod expiration;
pub mod file;
pub mod filename;
pub mod origin;

pub struct UploadRequest {
    pub filename: Option<String>,
    pub size: u64,
    pub origin: String,
}

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    response_type: ResponseType,
    authenticator: Extension<Arc<Authenticator>>,
    auth_header: Option<TypedHeader<Authorization<Basic>>>,
    cookie: Option<TypedHeader<Cookie>>,
    Extension(real_ip): Extension<RealIp>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    forwarded_address: Option<ForwardedForHeader>,
    Extension(origin): Extension<Origin>,
    Extension(limiter): Extension<Arc<ChainLimiter>>,
    Extension(determiner): Extension<Arc<Determiner>>,
    Extension(dir): Extension<Dir>,
    DomainUri(domain_uri): DomainUri,
    TypedHeader(ContentLength(size)): TypedHeader<ContentLength>,
    Filename(filename): Filename,
    body: BodyStream,
) -> Result<ApiResponse<UploadInfo>, ApiResponse<Error>> {
    let username = match authenticator
        .allows(
            auth_header.map(|h| h.0),
            cookie.map(|h| h.0),
            Features::UPLOAD,
        )
        .await
    {
        AuthStatus::NotNeeded => None,
        AuthStatus::Valid(username) => Some(username),
        AuthStatus::Error(err) => return Err(response_type.to_api_response(err)),
        AuthStatus::Prompt => {
            return Err(response_type.to_api_response(AuthError::MissingAuthorization));
        }
    };

    let origin = match origin {
        Origin::IpAddress => real_ip
            .resolve(addr.ip(), forwarded_address.map(|fa| fa.0))
            .ok_or_else(|| response_type.to_api_response(UploadError::Origin))?
            .to_string(),
        Origin::Username => {
            username.ok_or_else(|| response_type.to_api_response(UploadError::Origin))?
        }
    };

    let upload_res = process_upload(
        pool, limiter, origin, determiner, domain_uri, dir, size, filename, body,
    )
    .await
    .map_err(|err| response_type.to_api_response(err))?;
    Ok(response_type.to_api_response(upload_res))
    // Ok(adaptive_response(
    //     response_type,
    //     StatusCode::CREATED,
    //     upload_res,
    // )?)
}

#[allow(clippy::bool_comparison)]
async fn process_upload(
    pool: SqlitePool,
    limiter: Arc<ChainLimiter>,
    origin: String,
    determiner: Arc<Determiner>,
    domain_uri: String,
    dir: Dir,
    size: u64,
    filename: Option<String>,
    body: BodyStream,
) -> Result<UploadInfo, Error> {
    // let origin = match *req.data::<Origin>().ok_or(UploadError::Origin)? {
    //     Origin::IpAddress => req
    //         .data::<RealIp>()
    //         .ok_or(UploadError::Origin)?
    //         .find(&req)
    //         .ok_or(UploadError::Origin)?
    //         .to_string(),
    //     Origin::Username => username.ok_or(UploadError::Origin)?,
    // };

    // let upload_req = UploadRequest {
    //     name: parse_filename_header(req.headers())?,
    //     size: parse_file_size(req.headers())?,
    //     origin,
    // };
    let upload_req = UploadRequest {
        filename,
        size,
        origin,
    };
    let mut conn = pool.acquire().await.map_err(|_| UploadError::Database)?;

    // Quota.
    if !limiter
        .accept(&upload_req, &mut conn)
        .await
        .ok_or(UploadError::QuotaAccess)?
    {
        return Err(UploadError::QuotaExceeded);
    }
    // if req
    //     .data::<ChainLimiter>()
    //     .ok_or(UploadError::QuotaAccess)?
    //     .accept(&upload_req, &mut conn)
    //     .await
    //     .ok_or(UploadError::QuotaAccess)?
    //     == false
    // {
    //     return Err(UploadError::QuotaExceeded);
    // }

    // Aliases and links.
    let (short, long) = alias::random_unused_aliases(&mut conn)
        .await
        .ok_or(UploadError::AliasGeneration)?;
    // let link_base = request_target(req.headers()).ok_or(UploadError::Target)?;

    // Expiration.
    // let determiner = req
    //     .data::<Determiner>()
    //     .ok_or(UploadError::TimeCalculation)?;
    let expiration = Expiration::try_from(
        determiner
            .determine(upload_req.size)
            .ok_or(UploadError::TooLarge)?,
    )?;

    let id = Uuid::new_v4().to_hyphenated_ref().to_string();
    let admin = Uuid::new_v4().to_hyphenated_ref().to_string();

    sqlx::query(include_query!("insert_file"))
        .bind(&id)
        .bind(&admin)
        .bind(upload_req.origin.to_string())
        .bind(expiration.timestamp() as i64)
        .bind(&upload_req.filename)
        .bind(upload_req.size as i64)
        .bind(&short)
        .bind(&long)
        .execute(&mut conn)
        .await
        .map_err(|_| UploadError::Database)?;
    drop(conn);

    // Copy body to file system.
    // let path = req
    //     .data::<Dir>()
    //     .ok_or(UploadError::PathResolve)?
    //     .file_path(&id);
    let path = dir.file_path(&id);
    let file = File::create(&path)
        .await
        .map_err(|_| UploadError::CreateFile)?;
    if let Err(err) = write_file(&upload_req, body, file).await {
        clean_failed_upload(path.as_path(), &id, &pool).await;
        return Err(err);
    }

    Ok(UploadInfo::new(
        admin,
        upload_req.filename.unwrap_or_else(|| long.clone()),
        upload_req.size,
        (short, long),
        domain_uri,
        expiration,
    ))
}

// fn parse_filename_header(headers: &HeaderMap) -> UploadResult<Option<String>> {
//     if let Some(header) = headers.get("X-Filename") {
//         Ok(Some(sanitize(
//             percent_decode_str(
//                 std::str::from_utf8(header.as_bytes()).map_err(|_| UploadError::Database)?,
//             )
//             .decode_utf8()
//             .map_err(|_| UploadError::FilenameHeader)?,
//         )))
//     } else {
//         Ok(None)
//     }
// }
//
// fn parse_file_size(headers: &HeaderMap) -> UploadResult<u64> {
//     headers
//         .get(header::CONTENT_LENGTH)
//         .ok_or(UploadError::ContentLength)?
//         .to_str()
//         .map_err(|_| UploadError::ContentLength)?
//         .parse::<u64>()
//         .map_err(|_| UploadError::ContentLength)
// }

async fn write_file(
    req: &UploadRequest,
    mut body: BodyStream,
    mut file: File,
) -> Result<(), Error> {
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
        log::error!(
            "Cannot remove file with id {} from file system, file will retain quota: {}",
            id,
            err
        );
        return;
    }
    if let Err(err) = sqlx::query(include_query!("delete_file"))
        .bind(&id)
        .execute(pool)
        .await
    {
        log::error!("Cannot remove file with id {} from database: {:?}", id, err);
    }
}

pub fn router(
    pool: SqlitePool,
    auth: Arc<Authenticator>,
    real_ip: RealIp,
    origin: Origin,
    limiters: ChainLimiter,
    determiner: Arc<Determiner>,
    dir: Dir,
) -> Router {
    Router::new()
        .route("/", post(handler))
        .route("/upload", post(handler))
        .route_layer(Extension(pool))
        .route_layer(Extension(auth))
        .route_layer(Extension(real_ip))
        .route_layer(Extension(origin))
        .route_layer(Extension(Arc::new(limiters)))
        .route_layer(Extension(determiner))
        .route_layer(Extension(dir))
}
