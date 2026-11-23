use serde::{Serialize, Serializer};
use std::fmt;

/// Wrapper type that redacts sensitive data in logs
///
/// Usage:
/// ```rust
/// use payraider_backend::logging::redaction::Redacted;
///
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
#[must_use]
pub fn redact_account(account: &str) -> String {
    if account.len() <= 8 {
        return "[REDACTED]".to_string();
    }
    format!("{}...{}", &account[..4], &account[account.len() - 4..])
}

/// Redact payment amounts (show only order of magnitude)
///
/// Example: `1234.56` becomes `~10^3`
#[must_use]
pub fn redact_amount(amount: f64) -> String {
    if amount <= 0.0 {
        return "~10^0".to_string();
    }
    let magnitude = amount.log10().floor() as i32;
    format!("~10^{magnitude}")
}

/// Redact transaction hash (show first 4 and last 4 chars)
#[must_use]
pub fn redact_hash(hash: &str) -> String {
    if hash.len() <= 8 {
        return "[REDACTED]".to_string();
    }
    format!("{}...{}", &hash[..4], &hash[hash.len() - 4..])
}

/// Redact user ID (show only prefix)
///
/// Example: `user_12345678` becomes `user_****`
#[must_use]
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
#[must_use]
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
#[must_use]
pub fn redact_ip(ip: &str) -> String {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() == 4 {
        format!("{}.{}.*.*", parts[0], parts[1])
    } else if ip.contains(':') {
        // IPv6 - show only first segment
        let parts: Vec<&str> = ip.split(':').collect();
        if parts.is_empty() {
            "[REDACTED]".to_string()
        } else {
            format!("{}:****", parts[0])
        }
    } else {
        "[REDACTED]".to_string()
    }
}

/// Redact API key or token (show only first 4 chars)
#[must_use]
pub fn redact_token(token: &str) -> String {
    if token.len() > 4 {
        format!("{}****", &token[..4])
    } else {
        "[REDACTED]".to_string()
    }
}

/// Redact a complete log line of pattern-based PII before it is written.
///
/// This is the emission-time safety net: regardless of which code path
/// produced the event, values that look like email addresses, Stellar
/// secret keys, JWTs, or sensitive `key: value` / `"key":"value"` pairs are
/// replaced with `[REDACTED]` before the line reaches stdout or the log
/// file. Field-level redaction (`Redacted`, `redact_email`, …) remains the
/// preferred mechanism; this catches everything the field-level calls miss.
#[must_use]
pub fn redact_pii_line(input: &str) -> String {
    let spans = pii_spans(input);
    if spans.is_empty() {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut pos = 0usize;
    for (start, end) in spans {
        if start < pos {
            continue; // overlapping span already covered
        }
        out.push_str(&input[pos..start]);
        out.push_str("[REDACTED]");
        pos = end;
    }
    out.push_str(&input[pos..]);
    out
}

fn is_email_local(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'%' | b'+' | b'-')
}

fn is_strkey_char(b: u8) -> bool {
    b.is_ascii_uppercase() || b.is_ascii_digit() || (b'2'..=b'7').contains(&b)
}

fn pii_spans(input: &str) -> Vec<(usize, usize)> {
    let b = input.as_bytes();
    let n = b.len();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut i = 0usize;

    while i < n {
        // JWT: eyJ... (header.payload.signature, base64url)
        if b[i] == b'e' && i + 2 < n && b[i + 1] == b'y' && b[i + 2] == b'J' {
            let mut end = i;
            while end < n
                && (b[end].is_ascii_alphanumeric() || matches!(b[end], b'_' | b'-' | b'.' | b'='))
            {
                end += 1;
            }
            if end - i > 20 {
                spans.push((i, end));
                i = end;
                continue;
            }
        }

        // Stellar secret key: 'S' + long strkey payload (never logged).
        if b[i] == b'S' {
            let mut end = i + 1;
            while end < n && is_strkey_char(b[end]) {
                end += 1;
            }
            if end - i - 1 >= 50 {
                spans.push((i, end));
                i = end;
                continue;
            }
        }

        // Email local part: keep the domain for debuggability.
        if b[i] == b'@' && i > 0 {
            let mut start = i;
            while start > 0 && is_email_local(b[start - 1]) {
                start -= 1;
            }
            let mut end = i + 1;
            while end < n && (b[end].is_ascii_alphanumeric() || matches!(b[end], b'.' | b'-')) {
                end += 1;
            }
            if start < i && end > i + 1 && input[i + 1..end].contains('.') {
                spans.push((start, i));
            }
            i = end.max(i + 1);
            continue;
        }

        i += 1;
    }

    // Sensitive key/value pairs: "password":"x", password=x, token: y …
    const SENSITIVE_KEYS: &[&str] = &[
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "authorization",
        "private_key",
        "seed",
    ];
    let lower = input.to_ascii_lowercase();
    let lb = lower.as_bytes();
    for key in SENSITIVE_KEYS {
        let mut from = 0usize;
        while let Some(rel) = lower[from..].find(key) {
            let kpos = from + rel;
            from = kpos + key.len();
            // The key must be a whole word (bounded by non-alphanumeric).
            let before_ok = kpos == 0 || !lb[kpos - 1].is_ascii_alphanumeric();
            let after = kpos + key.len();
            let after_ok = after >= n || !lb[after].is_ascii_alphanumeric();
            if !before_ok || !after_ok {
                continue;
            }
            // Find the value start: skip whitespace, one ':' or '=', optional quote.
            let mut j = after;
            while j < n && (lb[j] == b' ' || lb[j] == b'\t') {
                j += 1;
            }
            if j < n && (lb[j] == b':' || lb[j] == b'=') {
                j += 1;
                while j < n && (lb[j] == b' ' || lb[j] == b'\t') {
                    j += 1;
                }
            } else {
                continue;
            }
            let quote = j < n && lb[j] == b'"';
            if quote {
                j += 1;
            }
            let vstart = j;
            let vend = if quote {
                let mut e = j;
                while e < n && b[e] != b'"' {
                    e += 1;
                }
                e
            } else {
                let mut e = j;
                while e < n && !(lb[e] == b',' || lb[e] == b'}' || lb[e] == b' ' || lb[e] == b'\n')
                {
                    e += 1;
                }
                e
            };
            if vend > vstart {
                spans.push((vstart, vend));
            }
        }
    }

    spans.sort_unstable();
    spans.dedup();
    spans
}

/// Hash an API key's database ID for structured logs (#2657).
///
/// Support needs to correlate log lines for a single caller (e.g. "which key
/// is hammering this endpoint") without the logs themselves becoming another
/// place a key identifier can be read out of and reused to look up or impact
/// the live key. This takes the already-internal `api_keys.id` (never the
/// plaintext secret) through SHA-256 and returns a short hex prefix, which is
/// stable per key (so the same key always yields the same `key_id` in logs,
/// letting support group by it) but is one-way and not the value an attacker
/// could present back to the API.
#[must_use]
pub fn hash_key_id(api_key_id: &str) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(api_key_id.as_bytes());
    // 16 hex chars (64 bits) is plenty of entropy to distinguish keys in logs
    // without printing the full 32-byte digest on every line.
    hex::encode(&digest[..8])
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
    fn test_hash_key_id_is_stable_and_one_way() {
        let id = "01234567-89ab-cdef-0123-456789abcdef";
        let first = hash_key_id(id);
        let second = hash_key_id(id);
        // Same key id always hashes the same way, so support can group log
        // lines by `key_id` for one caller.
        assert_eq!(first, second);
        // Never just the raw id re-encoded.
        assert_ne!(first, id);
        assert_eq!(first.len(), 16);
    }

    #[test]
    fn test_hash_key_id_differs_per_key() {
        assert_ne!(hash_key_id("key-a"), hash_key_id("key-b"));
    }

    #[test]
    fn test_redact_pii_line_email() {
        let line = r#"{"msg":"login","email":"alice@example.com","ip":"10.0.0.1"}"#;
        let out = redact_pii_line(line);
        assert!(out.contains("****@example.com"), "got: {out}");
        assert!(!out.contains("alice@"), "got: {out}");
    }

    #[test]
    fn test_redact_pii_line_stellar_secret() {
        let secret = format!("S{}", "A".repeat(55));
        let line = format!("signing with {secret} now");
        let out = redact_pii_line(&line);
        assert!(!out.contains(&secret), "got: {out}");
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_pii_line_jwt() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let line = format!("token={jwt}");
        let out = redact_pii_line(&line);
        assert!(!out.contains(jwt), "got: {out}");
    }

    #[test]
    fn test_redact_pii_line_sensitive_fields() {
        let line = r#"{"password":"hunter2","user":"bob"}"#;
        let out = redact_pii_line(line);
        assert!(!out.contains("hunter2"), "got: {out}");
        assert!(out.contains("bob"));
    }

    #[test]
    fn test_redact_pii_line_clean_passthrough() {
        let line = r#"{"msg":"backup completed","path":"/tmp/x.db","size":1024}"#;
        assert_eq!(redact_pii_line(line), line);
    }

    #[test]
    fn test_redacted_wrapper() {
        let secret = "my_secret_value";
        let redacted = Redacted(secret);
        assert_eq!(format!("{:?}", redacted), "[REDACTED]");
        assert_eq!(format!("{}", redacted), "[REDACTED]");
    }
}
