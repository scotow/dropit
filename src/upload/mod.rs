use std::{convert::TryFrom, net::SocketAddr, sync::Arc};

use axum::{
    extract::{BodyStream, ConnectInfo},
    headers::{authorization::Basic, Authorization, ContentLength, Cookie},
    routing::post,
    Extension, Router, TypedHeader,
};
use file::UploadInfo;
use filename::Filename;
use futures::StreamExt;
use sqlx::SqlitePool;
use tokio::{fs::File, io::AsyncWriteExt};
use uuid::Uuid;

use crate::{
    alias,
    auth::{AuthStatus, Authenticator, Features, Origin},
    error::{auth as AuthError, upload as UploadError, Error},
    include_query,
    limit::{Chain as ChainLimiter, Limiter},
    response::{ApiResponse, ResponseType},
    storage::Dir,
    upload::origin::ForwardedForHeader,
};

mod expiration;
mod file;
mod filename;
mod origin;

pub use expiration::{Determiner, Threshold};
pub use file::Expiration;
pub use origin::{DomainUri, RealIp};

pub struct UploadRequest {
    pub filename: Option<String>,
    pub size: u64,
    pub origin: String,
}

#[allow(clippy::too_many_arguments)]
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
        AuthStatus::Error(err) => return Err(ApiResponse(response_type, err)),
        AuthStatus::Prompt => {
            return Err(ApiResponse(response_type, AuthError::MissingAuthorization));
        }
    };

    let origin = match origin {
        Origin::IpAddress => real_ip
            .resolve(addr.ip(), forwarded_address.map(|fa| fa.0))
            .ok_or(ApiResponse(response_type, UploadError::Origin))?
            .to_string(),
        Origin::Username => username.ok_or(ApiResponse(response_type, UploadError::Origin))?,
    };

    let info = process_upload(
        pool, limiter, origin, determiner, domain_uri, dir, size, filename, body,
    )
    .await
    .map_err(|err| ApiResponse(response_type, err))?;
    Ok(ApiResponse(response_type, info))
}

#[allow(clippy::too_many_arguments)]
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

    // Aliases and links.
    let (short, long) = alias::random_unused_aliases(&mut conn)
        .await
        .ok_or(UploadError::AliasGeneration)?;

    // Expiration.
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
    let file = dir
        .create_file(&id)
        .await
        .map_err(|_| UploadError::CreateFile)?;
    if let Err(err) = write_file(&upload_req, body, file).await {
        clean_failed_upload(&dir, &id, &pool).await;
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

async fn clean_failed_upload(dir: &Dir, id: &str, pool: &SqlitePool) {
    if let Err(err) = dir.delete_file(id).await {
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
