use crate::limit::Limiter;
use sqlx::SqliteConnection;
use crate::upload::handler::UploadRequest;
use async_trait::async_trait;
use crate::include_query;

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
    async fn accept(&self, req: &UploadRequest, conn: &mut SqliteConnection) -> bool {
        let (size,) = sqlx::query_as::<_, (i64,)>(include_query!("get_limit_global"))
            .fetch_one(conn).await.unwrap();
        size as u64 + req.size <= self.size_sum
    }
}