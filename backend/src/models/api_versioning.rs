#[derive(Clone, Debug)]
pub struct VersioningConfig {
    pub current_version: u32,
    pub min_supported_version: u32,
}

impl Default for VersioningConfig {
    fn default() -> Self {
        Self {
            current_version: 1,
            min_supported_version: 1,
        }
    }
}
