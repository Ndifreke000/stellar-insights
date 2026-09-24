#[derive(Clone, Debug)]
pub struct SchemaConfig {
    pub prefix: Option<String>,
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self { prefix: None }
    }
}
