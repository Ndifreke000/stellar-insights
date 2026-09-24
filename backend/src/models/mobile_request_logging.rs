#[derive(Clone, Debug)]
pub struct MobileLogConfig {
    pub log_headers: bool,
    pub log_body: bool,
}

impl Default for MobileLogConfig {
    fn default() -> Self {
        Self {
            log_headers: false,
            log_body: false,
        }
    }
}
