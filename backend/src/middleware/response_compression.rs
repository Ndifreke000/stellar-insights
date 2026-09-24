use anyhow::Result;

use crate::models::network_context_middleware::Network;
use crate::models::response_compression::{CompressionAlgorithm, CompressionConfig};

/// Middleware component that selects and validates compression settings
/// based on network context and client type.
#[derive(Clone)]
pub struct ResponseCompression {
    config: CompressionConfig,
}

impl ResponseCompression {
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Returns the effective algorithm for the given network and whether the
    /// client is a mobile client. Mobile clients on Testnet prefer gzip for
    /// compatibility; Mainnet mobile clients use the configured algorithm.
    pub fn effective_algorithm(&self, network: Network, is_mobile: bool) -> CompressionAlgorithm {
        if !self.config.enabled {
            return CompressionAlgorithm::None;
        }
        if is_mobile {
            match network {
                // Testnet: always gzip for mobile (broader compatibility)
                Network::Testnet => CompressionAlgorithm::Gzip,
                Network::Mainnet => self.config.mobile_algorithm,
            }
        } else {
            // Non-mobile: prefer brotli when available
            CompressionAlgorithm::Brotli
        }
    }

    /// Validate that the configuration is sensible, returning an error if not.
    pub fn validate(&self) -> Result<()> {
        if self.config.min_size_bytes == 0 {
            anyhow::bail!("COMPRESSION_MIN_SIZE must be > 0");
        }
        Ok(())
    }

    pub fn config(&self) -> &CompressionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::response_compression::CompressionConfig;

    fn middleware() -> ResponseCompression {
        ResponseCompression::new(CompressionConfig::default())
    }

    #[test]
    fn test_mobile_mainnet_uses_configured_algorithm() {
        let m = middleware();
        let algo = m.effective_algorithm(Network::Mainnet, true);
        assert_eq!(algo, CompressionAlgorithm::Gzip);
    }

    #[test]
    fn test_mobile_testnet_always_gzip() {
        let m = middleware();
        let algo = m.effective_algorithm(Network::Testnet, true);
        assert_eq!(algo, CompressionAlgorithm::Gzip);
    }

    #[test]
    fn test_non_mobile_prefers_brotli() {
        let m = middleware();
        let algo = m.effective_algorithm(Network::Mainnet, false);
        assert_eq!(algo, CompressionAlgorithm::Brotli);
    }

    #[test]
    fn test_disabled_returns_none() {
        let mut cfg = CompressionConfig::default();
        cfg.enabled = false;
        let m = ResponseCompression::new(cfg);
        assert_eq!(
            m.effective_algorithm(Network::Mainnet, false),
            CompressionAlgorithm::None
        );
    }

    #[test]
    fn test_validate_rejects_zero_min_size() {
        let mut cfg = CompressionConfig::default();
        cfg.min_size_bytes = 0;
        let m = ResponseCompression::new(cfg);
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_validate_accepts_valid_config() {
        assert!(middleware().validate().is_ok());
    }
}
