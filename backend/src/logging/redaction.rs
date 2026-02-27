use serde::{Serialize, Serializer};
use std::fmt;

/// Wrapper type that redacts sensitive data in logs
///
/// Usage:
/// ```
/// let sensitive_data = "secret_value";
/// tracing::info!("Processing data: {:?}", Redacted(&sensitive_data));
/// // Logs: Processing data: [REDACTED]
/// ```
#[derive(Clone)]
pub struct Redacted<T>(pub T);

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl<T> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl<T: Serialize> Serialize for Redacted<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("[REDACTED]")
    }
}

/// Redact Stellar account addresses (show first 4 and last 4 chars)
///
/// Example: `GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX`
/// becomes `GXXX...XXXX`
pub fn redact_account(account: &str) -> String {
    if account.len() <= 8 {
        return "[REDACTED]".to_string();
    }
    format!("{}...{}", &account[..4], &account[account.len() - 4..])
}

/// Redact payment amounts (show only order of magnitude)
///
/// Example: `1234.56` becomes `~10^3`
pub fn redact_amount(amount: f64) -> String {
    if amount <= 0.0 {
        return "~10^0".to_string();
    }
    let magnitude = amount.log10().floor() as i32;
    format!("~10^{}", magnitude)
}

/// Redact transaction hash (show first 4 and last 4 chars)
pub fn redact_hash(hash: &str) -> String {
    if hash.len() <= 8 {
        return "[REDACTED]".to_string();
    }
    format!("{}...{}", &hash[..4], &hash[hash.len() - 4..])
}

/// Redact user ID (show only prefix)
///
/// Example: `user_12345678` becomes `user_****`
pub fn redact_user_id(user_id: &str) -> String {
    if let Some(pos) = user_id.find('_') {
        format!("{}****", &user_id[..=pos])
    } else if user_id.len() > 4 {
        format!("{}****", &user_id[..4])
    } else {
        "[REDACTED]".to_string()
    }
}

/// Redact email address (show only domain)
///
/// Example: `user@example.com` becomes `****@example.com`
pub fn redact_email(email: &str) -> String {
    if let Some(pos) = email.find('@') {
        format!("****{}", &email[pos..])
    } else {
        "[REDACTED]".to_string()
    }
}

/// Redact IP address (show only first two octets)
///
/// Example: `192.168.1.100` becomes `192.168.*.*`
pub fn redact_ip(ip: &str) -> String {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() == 4 {
        format!("{}.{}.*.*", parts[0], parts[1])
    } else if ip.contains(':') {
        // IPv6 - show only first segment
        let parts: Vec<&str> = ip.split(':').collect();
        if !parts.is_empty() {
            format!("{}:****", parts[0])
        } else {
            "[REDACTED]".to_string()
        }
    } else {
        "[REDACTED]".to_string()
    }
}

/// Redact API key or token (show only first 4 chars)
pub fn redact_token(token: &str) -> String {
    if token.len() > 4 {
        format!("{}****", &token[..4])
    } else {
        "[REDACTED]".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_account() {
        let account = "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
        let redacted = redact_account(account);
        assert_eq!(redacted, "GXXX...XXXX");
    }

    #[test]
    fn test_redact_account_short() {
        let account = "SHORT";
        let redacted = redact_account(account);
        assert_eq!(redacted, "[REDACTED]");
    }

    #[test]
    fn test_redact_amount() {
        assert_eq!(redact_amount(1234.56), "~10^3");
        assert_eq!(redact_amount(50.0), "~10^1");
        assert_eq!(redact_amount(0.5), "~10^-1");
    }

    #[test]
    fn test_redact_hash() {
        let hash = "abcdef1234567890abcdef1234567890";
        let redacted = redact_hash(hash);
        assert_eq!(redacted, "abcd...7890");
    }

    #[test]
    fn test_redact_user_id() {
        assert_eq!(redact_user_id("user_12345678"), "user_****");
        assert_eq!(redact_user_id("12345678"), "1234****");
    }

    #[test]
    fn test_redact_email() {
        assert_eq!(redact_email("user@example.com"), "****@example.com");
        assert_eq!(redact_email("invalid"), "[REDACTED]");
    }

    #[test]
    fn test_redact_ip() {
        assert_eq!(redact_ip("192.168.1.100"), "192.168.*.*");
        assert_eq!(redact_ip("2001:db8::1"), "2001:****");
    }

    #[test]
    fn test_redact_token() {
        assert_eq!(redact_token("abcdef1234567890"), "abcd****");
        assert_eq!(redact_token("abc"), "[REDACTED]");
    }

    #[test]
    fn test_redacted_wrapper() {
        let secret = "my_secret_value";
        let redacted = Redacted(secret);
        assert_eq!(format!("{:?}", redacted), "[REDACTED]");
        assert_eq!(format!("{}", redacted), "[REDACTED]");
    }
}
