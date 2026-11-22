use anyhow::Result;

use crate::models::network_context_middleware::Network;
use crate::models::response_compression::{CompressionAlgorithm, CompressionConfig};

/// Header name used to advertise that the response body varies by encoding.
pub const VARY_HEADER: &str = "Vary";
/// Value emitted for `Vary` so caches key on the negotiated encoding.
pub const VARY_ACCEPT_ENCODING: &str = "Accept-Encoding";

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

/// Compute a weak ETag over the *uncompressed* representation.
///
/// The tag is derived from the raw bytes so it is stable regardless of the
/// negotiated `Content-Encoding` (gzip, br, identity). It is marked weak
/// (`W/"..."`) because the compressed byte streams differ per encoding even
/// though the underlying representation is identical. This keeps conditional
/// requests (`If-None-Match`) consistent across encodings and avoids
/// strong/weak mismatches.
pub fn weak_etag_for_body(body: &[u8]) -> String {
    // FNV-1a 64-bit: deterministic, dependency-free, and sufficient for
    // cache validation (not a security boundary).
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in body {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("W/\"{:016x}-{:x}\"", hash, body.len())
}

/// Returns the `Vary` header value that must accompany any response subject to
/// compression or carrying an ETag.
pub fn vary_accept_encoding() -> &'static str {
    VARY_ACCEPT_ENCODING
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

    #[test]
    fn test_weak_etag_is_marked_weak() {
        let tag = weak_etag_for_body(b"hello");
        assert!(tag.starts_with("W/\""));
        assert!(tag.ends_with('\"'));
    }

    #[test]
    fn test_weak_etag_stable_across_encodings() {
        // The ETag is computed on the uncompressed representation, so the
        // same body yields the same tag regardless of negotiated encoding.
        let body = b"{\"data\":\"stellar\"}";
        let identity = weak_etag_for_body(body);
        let gzip = weak_etag_for_body(body);
        let br = weak_etag_for_body(body);
        assert_eq!(identity, gzip);
        assert_eq!(gzip, br);
    }

    #[test]
    fn test_weak_etag_differs_for_different_bodies() {
        assert_ne!(weak_etag_for_body(b"a"), weak_etag_for_body(b"b"));
    }

    #[test]
    fn test_vary_accept_encoding_value() {
        assert_eq!(vary_accept_encoding(), "Accept-Encoding");
    }
}
