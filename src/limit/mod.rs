use async_trait::async_trait;
use sqlx::SqliteConnection;

use crate::upload::UploadRequest;

pub mod ip;
pub mod global;

#[async_trait]
pub trait Limiter {
    async fn accept(&self, req: &UploadRequest, conn: &mut SqliteConnection) -> Option<bool>;
}

pub struct Chain(Vec<Box<dyn Limiter + Send + Sync>>);

impl Chain {
    pub fn new(limiters: Vec<Box<dyn Limiter + Send + Sync>>) -> Self {
        Self(limiters)
    }
}

#[async_trait]
impl Limiter for Chain {
    async fn accept(&self, req: &UploadRequest, conn: &mut SqliteConnection) -> Option<bool> {
        for l in self.0.iter() {
            if !l.accept(&req, conn).await? {
                return Some(false)
            }
        }
        Some(true)
    }
}