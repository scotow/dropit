use crate::upload::expiration::Threshold;
use sqlx::{SqlitePool, query_as_with};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use crate::include_query;

pub struct Cleaner {
    pool: SqlitePool,
}

impl Cleaner {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn start(&self) {
        loop {
            let mut conn = self.pool.acquire().await.unwrap();
            let files = sqlx::query_as::<_, (String,)>(include_query!("get_files_expired"))
                .bind(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64)
                .fetch_all(&mut conn).await.unwrap();

            if !files.is_empty() {
                dbg!(&files);
                for (id,) in files {
                    tokio::fs::remove_file(format!("uploads/{}", &id)).await.unwrap();
                    sqlx::query(include_query!("delete_file"))
                        .bind(&id)
                        .execute(&mut conn).await.unwrap();
                }
            }
            drop(conn);

            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}