use hyper::{Request, Body};
use async_trait::async_trait;
use sqlx::{SqlitePool, Executor, SqliteConnection, Transaction, Sqlite};
use crate::upload::origin::real_ip;
use crate::include_query;
use sqlx::pool::PoolConnection;

#[async_trait]
pub trait Limiter<'a> {
    async fn accept<E>(&self, req: &Request<Body>, upload_size: u64, conn: E) -> bool
    where E: Executor<'a, Database = Sqlite>;
}

pub struct IpLimiter {
    max_size: u64,
    max_file: usize,
}

impl IpLimiter {
    pub fn new(max_size: u64, max_file: usize) -> Self {
        Self {
            max_size,
            max_file,
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
        dbg!(size, file);
        size as u64 + upload_size <= self.max_size && file as usize + 1 <= self.max_file
    }
}