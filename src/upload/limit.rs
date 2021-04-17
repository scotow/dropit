use hyper::{Request, Body};
use async_trait::async_trait;
use sqlx::{Executor, Sqlite};
use crate::upload::origin::real_ip;
use crate::include_query;

#[async_trait]
pub trait Limiter<'a> {
    async fn accept<E>(&self, req: &Request<Body>, upload_size: u64, conn: E) -> bool
    where E: Executor<'a, Database = Sqlite>;
}

pub struct IpLimiter {
    size_sum: u64,
    file_count: usize,
}

impl IpLimiter {
    pub fn new(size_sum: u64, file_count: usize) -> Self {
        Self {
            size_sum,
            file_count,
        }
    }
}

#[async_trait]
impl<'a> Limiter<'a> for IpLimiter {
    async fn accept<E>(&self, req: &Request<Body>, upload_size: u64, conn: E) -> bool
    where E: Executor<'a, Database = Sqlite> {
        let (size, file) = sqlx::query_as::<_, (i64, i64)>(include_query!("get_limit_origin"))
            .bind(real_ip(req).unwrap().to_string())
            .fetch_optional(conn).await.unwrap().unwrap();
        size as u64 + upload_size <= self.size_sum && file as usize + 1 <= self.file_count
    }
}