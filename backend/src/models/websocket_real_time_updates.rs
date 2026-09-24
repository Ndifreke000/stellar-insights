#[derive(Clone, Debug)]
pub struct WsConfig {
    pub max_connections: usize,
    pub heartbeat_interval_secs: u64,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            max_connections: 1_000,
            heartbeat_interval_secs: 30,
        }
    }
}
