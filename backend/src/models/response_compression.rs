use serde::{Deserialize, Serialize};

/// Compression algorithm preference, used to communicate client capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionAlgorithm {
    Gzip,
    Brotli,
    None,
}

impl Default for CompressionAlgorithm {
    fn default() -> Self {
        Self::Gzip
    }
}

/// Configuration for response compression behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Minimum response body size (bytes) before compression is applied.
    pub min_size_bytes: u16,
    /// Preferred algorithm for mobile clients.
    pub mobile_algorithm: CompressionAlgorithm,
    /// Whether compression is enabled at all.
    pub enabled: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_size_bytes: 1024,
            mobile_algorithm: CompressionAlgorithm::Gzip,
            enabled: true,
        }
    }
}

impl CompressionConfig {
    /// Load from environment variables, falling back to defaults.
    pub fn from_env() -> Self {
        let min_size_bytes = std::env::var("COMPRESSION_MIN_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1024);

        let mobile_algorithm = match std::env::var("COMPRESSION_MOBILE_ALGORITHM")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "brotli" | "br" => CompressionAlgorithm::Brotli,
            "none" => CompressionAlgorithm::None,
            _ => CompressionAlgorithm::Gzip,
        };

        let enabled = std::env::var("COMPRESSION_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        Self {
            min_size_bytes,
            mobile_algorithm,
            enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = CompressionConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.min_size_bytes, 1024);
        assert_eq!(cfg.mobile_algorithm, CompressionAlgorithm::Gzip);
    }

    #[test]
    fn test_algorithm_serialization() {
        let json = serde_json::to_string(&CompressionAlgorithm::Brotli).unwrap();
        assert_eq!(json, "\"brotli\"");
    }
}
