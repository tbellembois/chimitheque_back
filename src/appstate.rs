use dashmap::DashMap;
use governor::{
    RateLimiter,
    clock::QuantaClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
};
use r2d2::{self, Pool};
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub db_connection_pool: Arc<Pool<SqliteConnectionManager>>,

    pub rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware>>,

    pub keycloak_base_url: String,
    pub keycloak_client_id: String,
    pub keycloak_realm: String,

    pub pkce_store: Arc<Mutex<DashMap<String, String>>>,
}
