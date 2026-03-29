use crate::cache::CacheManager;
use crate::database::Database;
use crate::ingestion::DataIngestionService;
use crate::rpc::StellarRpcClient;
use crate::websocket::WsState;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::SystemTime;

/// Shared application state for handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub cache: Arc<CacheManager>,
    pub ws_state: Arc<WsState>,
    pub ingestion: Arc<DataIngestionService>,
    pub rpc_client: Arc<StellarRpcClient>,
    pub server_start_time: Arc<AtomicU64>,
}

impl AppState {
    #[must_use]
    pub fn new(
        db: Arc<Database>,
        cache: Arc<CacheManager>,
        ws_state: Arc<WsState>,
        ingestion: Arc<DataIngestionService>,
        rpc_client: Arc<StellarRpcClient>,
    ) -> Self {
        Self {
            db,
            cache,
            ws_state,
            ingestion,
            rpc_client,
            server_start_time: Arc::new(AtomicU64::new(
                SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
            )),
        }
    }
}

use axum::extract::FromRef;

impl FromRef<AppState> for Arc<Database> {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for Arc<CacheManager> {
    fn from_ref(state: &AppState) -> Self {
        state.cache.clone()
    }
}

impl FromRef<AppState> for Arc<StellarRpcClient> {
    fn from_ref(state: &AppState) -> Self {
        state.rpc_client.clone()
    }
}
