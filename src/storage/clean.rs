use crate::upload::expiration::Threshold;
use sqlx::SqlitePool;

pub struct Cleaner {
    rules: Vec<Threshold>,
    pool: SqlitePool,
}

impl Cleaner {
    pub fn new(rules: Vec<Threshold>, connection_pool: SqlitePool) -> Self {
        Self {
            rules,
            pool: connection_pool,
        }
    }

    pub async fn start() {

    }
}