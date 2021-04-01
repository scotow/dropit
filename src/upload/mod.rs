use hyper::{Request, Response, Body};
use std::convert::Infallible;
use futures::TryStreamExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio::fs::File;
use hyper::header::CONTENT_LENGTH;
use uuid::Uuid;
use crate::alias;
use sqlx::SqlitePool;
use routerify::ext::RequestExt;
use crate::include_query;

pub async fn upload_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut conn = req.data::<SqlitePool>().unwrap().acquire().await.unwrap();
    let (head, body) = req.into_parts();

    let id = Uuid::new_v4().to_hyphenated_ref().to_string();
    let name = head.headers.get("X-Filename").unwrap().to_str().unwrap();
    let size = head.headers.get(CONTENT_LENGTH).unwrap().to_str().unwrap().parse::<u64>().unwrap();
    let (short, long) = alias::random_aliases().unwrap();
    dbg!(&id, name, size, &short, &long);

    let mut ar = body
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        .into_async_read()
        .compat();

    let mut file = File::create(format!("uploads/{}", id)).await.unwrap();
    tokio::io::copy(&mut ar, &mut file).await.unwrap();

    sqlx::query(include_query!("insert_file"))
        .bind(&id)
        .bind(name)
        .bind(size as i64)
        .bind(short)
        .bind(long)
        .execute(&mut conn).await.unwrap();

    Ok(Response::new(Body::from("Upload page")))
}

// body.fold(file, |mut f, chunk| async move {
//     let chunk = chunk.unwrap();
//     println!("{:?}", chunk.len());
//     f.write_all(&chunk).await.unwrap();
//     f
// }).await;
// let fe = body.for_each(|chunk| {
//     println!("{}", chunk.unwrap().len());
//     futures::future::ready(())
// });
// fe.await;
// let fe = body.map_ok(|chunk| {
//     chunk.iter()
//         .map(|byte| byte.to_ascii_uppercase())
//         .collect::<Vec<u8>>()
// });