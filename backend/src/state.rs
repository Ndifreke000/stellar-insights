use crate::cache::CacheManager;
use crate::database::Database;
use crate::ingestion::DataIngestionService;
use crate::multi_network::{MultiNetworkConfig, NetworkContext};
use crate::network::StellarNetwork;
use crate::rpc::StellarRpcClient;
use crate::websocket::WsState;
use std::sync::{atomic::AtomicU64, Arc};
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
    pub multi_network_config: Arc<MultiNetworkConfig>,
    pub network_context: Arc<NetworkContext>,
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
        let multi_network_config = Arc::new(MultiNetworkConfig::from_env());
        let network_context = Arc::new(multi_network_config.get_primary_context());

        Self {
            db,
            cache,
            ws_state,
            ingestion,
            rpc_client,
            server_start_time: Arc::new(AtomicU64::new(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            )),
            multi_network_config,
            network_context,
        }
    }

    /// Create AppState with explicit network selection
    #[must_use]
    pub fn with_network(
        db: Arc<Database>,
        cache: Arc<CacheManager>,
        ws_state: Arc<WsState>,
        ingestion: Arc<DataIngestionService>,
        rpc_client: Arc<StellarRpcClient>,
        network: StellarNetwork,
    ) -> Self {
        let multi_network_config = Arc::new(MultiNetworkConfig::from_env());
        let network_context = Arc::new(
            multi_network_config
                .get_context(network)
                .unwrap_or_else(|| multi_network_config.get_primary_context()),
        );

        Self {
            db,
            cache,
            ws_state,
            ingestion,
            rpc_client,
            server_start_time: Arc::new(AtomicU64::new(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            )),
            multi_network_config,
            network_context,
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
