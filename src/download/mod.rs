use hyper::{Request, Body, Response, StatusCode};
use std::convert::Infallible;
use routerify::ext::RequestExt;
use crate::alias::Alias;
use sqlx::{SqlitePool, Row};
use crate::include_query;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use hyper::header::{CONTENT_DISPOSITION, CONTENT_LENGTH};
use sqlx::FromRow;

#[derive(FromRow)]
struct FileInfo {
    id: String,
    name: String,
    size: i64,
}

pub async fn download_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let alias = req.param("alias").unwrap().parse::<Alias>().unwrap();
    dbg!(&alias);

    let query = match &alias {
        Alias::Short(_) => include_query!("get_file_short"),
        Alias::Long(_) => include_query!("get_file_long"),
    };

    let mut conn = req.data::<SqlitePool>().unwrap().acquire().await.unwrap();
    let info = sqlx::query_as::<_, FileInfo>(query)
        .bind(alias.inner())
        .fetch_optional(&mut conn).await.unwrap().unwrap();

    let file = File::open(format!("uploads/{}", info.id)).await.unwrap();

    Ok(
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_LENGTH, info.size as u64)
            .header(CONTENT_DISPOSITION, format!(r#"attachment; filename="{}""#, info.name))
            .body(Body::wrap_stream(ReaderStream::new(file)))
            .unwrap()
    )
}