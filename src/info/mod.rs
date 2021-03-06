use axum::{routing::get, Extension, Router};
use sqlx::SqlitePool;

mod valid;

pub fn router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/valid/:alias", get(valid::handler))
        .route_layer(Extension(pool))
}
