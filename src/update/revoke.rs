use axum::Extension;
use sqlx::SqlitePool;

use crate::alias::Alias;
use crate::error::revoke as RevokeError;
use crate::error::Error;
use crate::include_query;
use crate::response::{ApiResponse, ResponseType};
use crate::storage::Dir;
use crate::update::AdminToken;

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    response_type: ResponseType,
    AdminToken(admin_token): AdminToken,
    alias: Alias,
    Extension(dir): Extension<Dir>,
) -> Result<ApiResponse<()>, ApiResponse<Error>> {
    process_revoke(pool, alias, admin_token, dir)
        .await
        .map_err(|err| response_type.to_api_response(err))?;
    Ok(response_type.to_api_response(()))
}

async fn process_revoke(
    pool: SqlitePool,
    alias: Alias,
    admin_token: String,
    dir: Dir,
) -> Result<(), Error> {
    let (id, _size, mut conn) = super::authorize(pool, &alias, &admin_token).await?;

    dir.delete_file(&id)
        .await
        .map_err(|_| RevokeError::RemoveFile)?;

    sqlx::query(include_query!("delete_file"))
        .bind(&id)
        .execute(&mut conn)
        .await
        .map_err(|_| RevokeError::PartialRemove)?;
    Ok(())
}
