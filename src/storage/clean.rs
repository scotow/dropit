use crate::upload::expiration::Threshold;
use sqlx::{SqlitePool, query_as_with, Sqlite, Error};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use crate::include_query;
use std::path::PathBuf;
use sqlx::pool::PoolConnection;

pub struct Cleaner {
    dir: PathBuf,
    pool: SqlitePool,
}

impl Cleaner {
    pub fn new(dir: PathBuf, pool: SqlitePool) -> Self {
        Self {
            dir,
            pool,
        }
    }

    pub async fn start(&self) {
        loop {
            self.clean_expires().await;
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }

    async fn clean_expires(&self) {
        let mut conn = match self.pool.acquire().await {
            Ok(conn) => conn,
            Err(_) => {
                eprintln!("[CLEAN] cannot acquire connection");
                return;
            }
        };

        let now_timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(timestamp) => timestamp.as_secs() as i64,
            Err(_) => {
                eprintln!("[CLEAN] cannot generate timestamp");
                return;
            }
        };

        let mut files = match sqlx::query_as::<_, (String,)>(include_query!("get_files_expired"))
            .bind(now_timestamp)
            .fetch_all(&mut conn).await {
            Ok(files) => files,
            Err(_) => {
                eprintln!("[CLEAN] cannot fetch expired files");
                return;
            }
        };

        if !files.is_empty() {
            for (id,) in files {
                if tokio::fs::remove_file(self.dir.join(&id)).await.is_err() {
                    eprintln!("[CLEAN] cannot remove file with id {}", id);
                    continue;
                }
                if sqlx::query(include_query!("delete_file"))
                    .bind(&id)
                    .execute(&mut conn).await.is_err() {
                    eprintln!("[CLEAN] cannot remove file with id {} from database", id);
                }
            }
        }
    }
}