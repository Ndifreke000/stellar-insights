use crate::cache::CacheManager;
use crate::database::Database;
use crate::ingestion::DataIngestionService;
use crate::websocket::WsState;
use crate::rpc::StellarRpcClient;
use std::sync::Arc;

/// Shared application state for handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub cache: Arc<CacheManager>,
    pub ws_state: Arc<WsState>,
    pub ingestion: Arc<DataIngestionService>,
    pub rpc_client: Arc<StellarRpcClient>,
}

impl AppState {
    #[must_use]
    pub const fn new(
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
            cache,
            rpc_client,
        }
    }
}
