use std::sync::Arc;
use crate::database::Database;
use crate::websocket::WsState;

/// Shared application state for handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub ws_state: Arc<WsState>,
}

impl AppState {
    pub fn new(db: Arc<Database>, ws_state: Arc<WsState>) -> Self {
        Self { db, ws_state }
    }
}
