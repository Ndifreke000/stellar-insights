use std::sync::Arc;
use crate::database::Database;
use crate::websocket::WsState;
use crate::ingestion::DataIngestionService;

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
