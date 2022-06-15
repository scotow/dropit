use std::sync::Arc;

use axum::extract::Query;
use axum::headers::authorization::Basic;
use axum::headers::{Authorization, Cookie, UserAgent};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Router, TypedHeader};
use serde::Deserialize;
use sqlx::{FromRow, SqlitePool};

use crate::alias::group::AliasGroup;
use crate::auth::{AuthStatus, Authenticator, Features};
use crate::error::auth as AuthError;
use crate::error::download as DownloadError;
use crate::storage::dir::Dir;
use crate::{error::Error, include_query};

mod archive;
mod file;
mod open_graph;

#[derive(FromRow)]
struct FileInfo {
    id: String,
    name: String,
    size: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ForceDownload {
    #[serde(default)]
    force_download: bool,
}

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    authenticator: Extension<Arc<Authenticator>>,
    auth_header: Option<TypedHeader<Authorization<Basic>>>,
    cookie: Option<TypedHeader<Cookie>>,
    AliasGroup(aliases): AliasGroup,
    force_download: Query<ForceDownload>,
    user_agent: Option<TypedHeader<UserAgent>>,
    Extension(dir): Extension<Dir>,
) -> Result<impl IntoResponse, Error> {
    match authenticator
        .allows(
            auth_header.map(|h| h.0),
            cookie.map(|h| h.0),
            Features::DOWNLOAD,
        )
        .await
    {
        AuthStatus::NotNeeded | AuthStatus::Valid(_) => (),
        AuthStatus::Error(err) => return Err(err),
        AuthStatus::Prompt => {
            return Err(AuthError::MissingAuthorization);
        }
    };
    let mut conn = pool.acquire().await.map_err(|_| DownloadError::Database)?;

    let mut files_info = Vec::with_capacity(aliases.len());
    for alias in aliases {
        files_info.push(
            sqlx::query_as::<_, FileInfo>(include_query!("get_file"))
                .bind(alias.inner())
                .bind(alias.inner())
                .fetch_optional(&mut conn)
                .await
                .map_err(|_| DownloadError::Database)?
                .ok_or(DownloadError::FileNotFound)?,
        );
    }

    if !force_download.force_download {
        if let Some(user_agent) = user_agent {
            if let Some(og_resp) =
                open_graph::proxy_request(user_agent.as_str().to_lowercase(), &files_info)
            {
                return Ok(og_resp);
            }
        }
    }

    match files_info.len() {
        0 => Err(DownloadError::AliasExtract),
        1 => file::handler(pool, &files_info[0], dir).await,
        _ => archive::handler(pool, files_info, dir).await,
    }
}

async fn file_downloaded(pool: &SqlitePool, dir: &Dir, id: &str) -> Result<(), String> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|err| format!("Cannot acquire database connect: {:?}", err))?;
    let (downloads,) = sqlx::query_as::<_, (Option<u16>,)>(include_query!("get_file_downloads"))
        .bind(id)
        .fetch_optional(&mut conn)
        .await
        .map_err(|err| format!("Cannot fetch downloads count: {:?}", err))?
        .ok_or("Cannot find file for downloads count decrement")?;
    match downloads {
        None => (),
        Some(0) => return Err(format!("Found a zero downloads counter file: {}", id)),
        Some(1) => {
            tokio::fs::remove_file(dir.file_path(id))
                .await
                .map_err(|err| {
                    format!(
                        "Failed to delete decremented to zero file from fs {}: {:?}",
                        id, err
                    )
                })?;
            sqlx::query(include_query!("delete_file"))
                .bind(id)
                .execute(&mut conn)
                .await
                .map_err(|err| {
                    format!(
                        "Failed to delete decremented to zero file from database {}: {:?}",
                        id, err
                    )
                })?;
        }
        Some(count) => {
            sqlx::query(include_query!("update_file_downloads"))
                .bind(count - 1)
                .bind(id)
                .execute(&mut conn)
                .await
                .map_err(|err| {
                    format!("Failed to decremented file from database {}: {:?}", id, err)
                })?;
        }
    };
    Ok(())
}

pub fn router(pool: SqlitePool, authenticator: Arc<Authenticator>, dir: Dir) -> Router {
    Router::new()
        .route("/:alias", get(handler))
        .route_layer(Extension(pool))
        .route_layer(Extension(authenticator))
        .route_layer(Extension(dir))
}
