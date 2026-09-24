#[derive(Clone, Debug)]
pub struct PaginationConfig {
    pub page_size: usize,
}

impl Default for PaginationConfig {
    fn default() -> Self {
        Self { page_size: 25 }
    }
}
