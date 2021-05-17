use crate::limit::Limiter;
use sqlx::SqliteConnection;
use async_trait::async_trait;
use crate::include_query;
use crate::upload::UploadRequest;

pub struct Ip {
    size_sum: u64,
    file_count: usize,
}

impl Ip {
    pub fn new(size_sum: u64, file_count: usize) -> Self {
        Self {
            size_sum,
            file_count,
        }
    }
}

#[async_trait]
impl Limiter for Ip {
    #[allow(clippy::int_plus_one)]
    async fn accept(&self, req: &UploadRequest, conn: &mut SqliteConnection) -> Option<bool> {
        let (size, count) = sqlx::query_as::<_, (i64, i64)>(include_query!("get_limit_origin"))
            .bind(req.origin.to_string())
            .fetch_one(conn).await.ok()?;
        Some(size as u64 + req.size <= self.size_sum && count as usize + 1 <= self.file_count)
    }
}