use hyper::{Request, Body};
use async_trait::async_trait;
use sqlx::SqlitePool;
use crate::upload::origin::real_ip;
use crate::include_query;

#[async_trait]
pub trait Limiter {
    async fn accept(&self, req: &Request<Body>, upload_size: u64) -> bool;
}

pub struct IpLimiter {
    max_size: u64,
    max_file: usize,
    pool: SqlitePool,
}

impl IpLimiter {
    pub fn new(max_size: u64, max_file: usize, pool: SqlitePool) -> Self {
        Self {
            max_size,
            max_file,
            pool,
        }
    }
}

#[async_trait]
impl Limiter for IpLimiter {
    async fn accept(&self, req: &Request<Body>, upload_size: u64) -> bool {
        let mut conn = self.pool.acquire().await.unwrap();
        let (size, file) = sqlx::query_as::<_, (i64, i64)>(include_query!("get_limit_origin"))
            .bind(real_ip(req).unwrap().to_string())
            .fetch_optional(&mut conn).await.unwrap().unwrap();
        dbg!(size, file);
        size as u64 + upload_size <= self.max_size && file as usize + 1 <= self.max_file
    }
}