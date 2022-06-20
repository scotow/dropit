use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::SqlitePool;

use crate::include_query;
use crate::storage::dir::Dir;

pub struct Cleaner {
    dir: Dir,
    pool: SqlitePool,
}

impl Cleaner {
    pub fn new(dir: Dir, pool: SqlitePool) -> Self {
        Self { dir, pool }
    }

    pub async fn start(&self) {
        loop {
            self.clean_expires().await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    async fn clean_expires(&self) {
        let mut conn = match self.pool.acquire().await {
            Ok(conn) => conn,
            Err(err) => {
                log::error!("Cannot acquire database connection: {:?}", err);
                return;
            }
        };

        let now_timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(timestamp) => timestamp.as_secs(),
            Err(err) => {
                log::error!("Cannot generate timestamp: {}", err);
                return;
            }
        };

        let files = match sqlx::query_as::<_, (String,)>(include_query!("get_files_expired"))
            .bind(now_timestamp as i64)
            .fetch_all(&mut conn)
            .await
        {
            Ok(files) => files,
            Err(err) => {
                log::error!("Cannot fetch expired files: {:?}", err);
                return;
            }
        };

        if !files.is_empty() {
            for (id,) in files {
                if let Err(err) = self.dir.delete_file(&id).await {
                    log::error!(
                        "Cannot remove file with id {} from file system: {}",
                        id,
                        err
                    );
                    continue;
                }
                if let Err(err) = sqlx::query(include_query!("delete_file"))
                    .bind(&id)
                    .execute(&mut conn)
                    .await
                {
                    log::error!("Cannot remove file with id {} from database: {}", id, err);
                }
            }
        }
    }
}
