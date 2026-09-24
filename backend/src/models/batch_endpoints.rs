use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum number of items allowed in a single batch request
    pub max_batch_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest<T> {
    pub items: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItemResult<T> {
    pub index: usize,
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse<T> {
    pub results: Vec<BatchItemResult<T>>,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
}
