use crate::database::Database;
use crate::ingestion::DataIngestionService;
use crate::websocket::WsState;
use std::sync::Arc;

/// Shared application state for handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub ws_state: Arc<WsState>,
    pub ingestion: Arc<DataIngestionService>,
}

impl AppState {
    pub fn new(
        db: Arc<Database>,
        ws_state: Arc<WsState>,
        ingestion: Arc<DataIngestionService>,
    ) -> Self {
        Self {
            db,
            ws_state,
            ingestion,
        }
    }
}
