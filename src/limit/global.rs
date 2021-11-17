use async_trait::async_trait;
use sqlx::SqliteConnection;

use crate::include_query;
use crate::limit::Limiter;
use crate::upload::UploadRequest;

pub struct Global {
    size_sum: u64,
}

impl Global {
    pub fn new(size_sum: u64) -> Self {
        Self { size_sum }
    }
}

#[async_trait]
impl Limiter for Global {
    async fn accept(&self, req: &UploadRequest, conn: &mut SqliteConnection) -> Option<bool> {
        let (size,) = sqlx::query_as::<_, (i64,)>(include_query!("get_limit_global"))
            .fetch_one(conn)
            .await
            .ok()?;
        Some(size as u64 + req.size <= self.size_sum)
    }
}
