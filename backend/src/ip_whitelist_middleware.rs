use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use ipnetwork::IpNetwork;
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;

/// IP Whitelist configuration
#[derive(Clone, Debug)]
pub struct IpWhitelistConfig {
    /// List of allowed IP addresses and CIDR ranges
    pub allowed_networks: Arc<Vec<IpNetwork>>,
    /// Whether to trust X-Forwarded-For header (when behind proxy/load balancer)
    pub trust_proxy: bool,
    /// Maximum number of IPs to check in X-Forwarded-For chain (prevents header injection)
    pub max_forwarded_ips: usize,
}

impl IpWhitelistConfig {
    /// Create a new IP whitelist configuration from environment variables
    pub fn from_env() -> Result<Self, String> {
        let whitelist_str = std::env::var("ADMIN_IP_WHITELIST")
            .map_err(|_| "ADMIN_IP_WHITELIST environment variable not set".to_string())?;

        let trust_proxy = std::env::var("ADMIN_IP_TRUST_PROXY")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let max_forwarded_ips = std::env::var("ADMIN_IP_MAX_FORWARDED")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(3);

        let allowed_networks = Self::parse_whitelist(&whitelist_str)?;

        Ok(Self {
            allowed_networks: Arc::new(allowed_networks),
            trust_proxy,
            max_forwarded_ips,
        })
    }

    /// Parse comma-separated list of IPs and CIDR ranges
    fn parse_whitelist(whitelist_str: &str) -> Result<Vec<IpNetwork>, String> {
        let mut networks = Vec::new();

        for entry in whitelist_str.split(',') {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Try parsing as CIDR notation first
            if let Ok(network) = IpNetwork::from_str(trimmed) {
                networks.push(network);
            } else if let Ok(ip) = IpAddr::from_str(trimmed) {
                // Single IP address - convert to /32 (IPv4) or /128 (IPv6) network
                let network = match ip {
                    IpAddr::V4(ipv4) => IpNetwork::V4(
                        ipnetwork::Ipv4Network::new(ipv4, 32)
                            .map_err(|e| format!("Invalid IPv4 address {}: {}", trimmed, e))?,
                    ),
                    IpAddr::V6(ipv6) => IpNetwork::V6(
                        ipnetwork::Ipv6Network::new(ipv6, 128)
                            .map_err(|e| format!("Invalid IPv6 address {}: {}", trimmed, e))?,
                    ),
                };
                networks.push(network);
            } else {
                return Err(format!("Invalid IP address or CIDR range: {}", trimmed));
            }
        }

        if networks.is_empty() {
            return Err("IP whitelist cannot be empty".to_string());
        }

        Ok(networks)
    }

    /// Check if an IP address is whitelisted
    pub fn is_allowed(&self, ip: &IpAddr) -> bool {
        self.allowed_networks
            .iter()
            .any(|network| network.contains(*ip))
    }
}

/// Extract client IP address from request
fn extract_client_ip(req: &Request, config: &IpWhitelistConfig) -> Result<IpAddr, String> {
    // If behind proxy and trust_proxy is enabled, check X-Forwarded-For
    if config.trust_proxy {
        if let Some(forwarded_for) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded_for.to_str() {
                // X-Forwarded-For format: client, proxy1, proxy2
                // We want the leftmost (original client) IP
                let ips: Vec<&str> = forwarded_str
                    .split(',')
                    .take(config.max_forwarded_ips)
                    .map(|s| s.trim())
                    .collect();

                if let Some(first_ip) = ips.first() {
                    if let Ok(ip) = IpAddr::from_str(first_ip) {
                        return Ok(ip);
                    }
                }
            }
        }

        // Also check X-Real-IP header (common with nginx)
        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(real_ip_str) = real_ip.to_str() {
                if let Ok(ip) = IpAddr::from_str(real_ip_str.trim()) {
                    return Ok(ip);
                }
            }
        }
    }

    // Fall back to direct connection IP
    if let Some(connect_info) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return Ok(connect_info.0.ip());
    }

    Err("Unable to determine client IP address".to_string())
}

/// IP whitelist middleware for admin endpoints
pub async fn ip_whitelist_middleware(
    config: axum::extract::State<Arc<IpWhitelistConfig>>,
    req: Request,
    next: Next,
) -> Result<Response, IpWhitelistError> {
    let client_ip = extract_client_ip(&req, &config)?;

    if !config.is_allowed(&client_ip) {
        // Log blocked attempt (without exposing sensitive info)
        tracing::warn!(
            client_ip = %client_ip,
            path = %req.uri().path(),
            method = %req.method(),
            "IP whitelist: blocked access attempt to admin endpoint"
        );

        return Err(IpWhitelistError::Forbidden);
    }

    // Log successful access
    tracing::debug!(
        client_ip = %client_ip,
        path = %req.uri().path(),
        "IP whitelist: allowed access to admin endpoint"
    );

    Ok(next.run(req).await)
}

/// IP whitelist errors
#[derive(Debug)]
pub enum IpWhitelistError {
    Forbidden,
    InvalidIp(String),
}

impl From<String> for IpWhitelistError {
    fn from(err: String) -> Self {
        IpWhitelistError::InvalidIp(err)
    }
}

impl IntoResponse for IpWhitelistError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            IpWhitelistError::Forbidden => (
                StatusCode::FORBIDDEN,
                "Access denied: IP address not whitelisted",
            ),
            IpWhitelistError::InvalidIp(_) => (
                StatusCode::FORBIDDEN,
                "Access denied: Unable to verify IP address",
            ),
        };

        let body = json!({
            "error": message,
        });

        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_ipv4() {
        let config = IpWhitelistConfig::parse_whitelist("192.168.1.1").unwrap();
        assert_eq!(config.len(), 1);
        assert!(config[0].contains(IpAddr::from_str("192.168.1.1").unwrap()));
        assert!(!config[0].contains(IpAddr::from_str("192.168.1.2").unwrap()));
    }

    #[test]
    fn test_parse_ipv4_cidr() {
        let config = IpWhitelistConfig::parse_whitelist("192.168.1.0/24").unwrap();
        assert_eq!(config.len(), 1);
        assert!(config[0].contains(IpAddr::from_str("192.168.1.1").unwrap()));
        assert!(config[0].contains(IpAddr::from_str("192.168.1.254").unwrap()));
        assert!(!config[0].contains(IpAddr::from_str("192.168.2.1").unwrap()));
    }

    #[test]
    fn test_parse_multiple_ips() {
        let config =
            IpWhitelistConfig::parse_whitelist("192.168.1.1, 10.0.0.0/8, 172.16.0.1").unwrap();
        assert_eq!(config.len(), 3);
    }

    #[test]
    fn test_parse_ipv6() {
        let config = IpWhitelistConfig::parse_whitelist("::1, 2001:db8::/32").unwrap();
        assert_eq!(config.len(), 2);
        assert!(config[0].contains(IpAddr::from_str("::1").unwrap()));
        assert!(config[1].contains(IpAddr::from_str("2001:db8::1").unwrap()));
    }

    #[test]
    fn test_parse_invalid_ip() {
        let result = IpWhitelistConfig::parse_whitelist("invalid.ip.address");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_whitelist() {
        let result = IpWhitelistConfig::parse_whitelist("");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_allowed() {
        let config = IpWhitelistConfig {
            allowed_networks: Arc::new(
                IpWhitelistConfig::parse_whitelist("192.168.1.0/24, 10.0.0.1").unwrap(),
            ),
            trust_proxy: false,
            max_forwarded_ips: 3,
        };

        assert!(config.is_allowed(&IpAddr::from_str("192.168.1.100").unwrap()));
        assert!(config.is_allowed(&IpAddr::from_str("10.0.0.1").unwrap()));
        assert!(!config.is_allowed(&IpAddr::from_str("192.168.2.1").unwrap()));
        assert!(!config.is_allowed(&IpAddr::from_str("10.0.0.2").unwrap()));
    }

    #[test]
    fn test_localhost_ipv4_and_ipv6() {
        let config = IpWhitelistConfig {
            allowed_networks: Arc::new(
                IpWhitelistConfig::parse_whitelist("127.0.0.1, ::1").unwrap(),
            ),
            trust_proxy: false,
            max_forwarded_ips: 3,
        };

        assert!(config.is_allowed(&IpAddr::from_str("127.0.0.1").unwrap()));
        assert!(config.is_allowed(&IpAddr::from_str("::1").unwrap()));
    }
}
