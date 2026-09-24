#[derive(Clone, Debug)]
pub struct NetworkAwareRpcConfig {
    pub timeout_ms: u64,
}

impl Default for NetworkAwareRpcConfig {
    fn default() -> Self {
        Self { timeout_ms: 5_000 }
    }
}
