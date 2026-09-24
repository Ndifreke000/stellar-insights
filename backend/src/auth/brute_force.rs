//! Brute-force detection and CAPTCHA verification shared by the auth endpoints.
//!
//! Per-account rate limiting and lockout live in `crate::api::auth`; this module adds
//! the cross-account view (one IP failing against many accounts) and real CAPTCHA
//! verification.

use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::observability::metrics::record_auth_security_event;

/// Window over which per-IP failures are counted.
const IP_FAILURE_WINDOW: Duration = Duration::from_secs(15 * 60);
/// Failures from one IP within the window that raise a brute-force alert.
const IP_FAILURE_ALERT_THRESHOLD: usize = 20;
/// Distinct accounts failing from one IP within the window that raise a
/// credential-stuffing alert.
const IP_DISTINCT_ACCOUNTS_ALERT_THRESHOLD: usize = 5;
/// Tracked IPs above which stale entries are pruned.
const MAX_TRACKED_IPS: usize = 10_000;

#[derive(Default)]
struct IpFailures {
    failures: VecDeque<(Instant, String)>,
    last_alert: Option<Instant>,
}

static IP_FAILURES: LazyLock<Mutex<HashMap<String, IpFailures>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default()
});

/// Record a failed authentication attempt and raise an alert when one IP shows a
/// brute-force or credential-stuffing pattern.
pub fn record_failure(endpoint: &str, account: &str, client_ip: &str) {
    record_auth_security_event(endpoint, "failure");

    let now = Instant::now();
    let (failures, distinct_accounts, should_alert) = {
        let mut ips = IP_FAILURES.lock().unwrap_or_else(|e| e.into_inner());
        if ips.len() > MAX_TRACKED_IPS {
            ips.retain(|_, entry| {
                entry
                    .failures
                    .back()
                    .is_some_and(|(t, _)| now.duration_since(*t) < IP_FAILURE_WINDOW)
            });
        }

        let entry = ips.entry(client_ip.to_string()).or_default();
        while entry
            .failures
            .front()
            .is_some_and(|(t, _)| now.duration_since(*t) >= IP_FAILURE_WINDOW)
        {
            entry.failures.pop_front();
        }
        entry.failures.push_back((now, account.to_string()));

        let failures = entry.failures.len();
        let distinct_accounts = entry
            .failures
            .iter()
            .map(|(_, a)| a.as_str())
            .collect::<HashSet<_>>()
            .len();
        let should_alert = (failures >= IP_FAILURE_ALERT_THRESHOLD
            || distinct_accounts >= IP_DISTINCT_ACCOUNTS_ALERT_THRESHOLD)
            && entry
                .last_alert
                .is_none_or(|t| now.duration_since(t) >= IP_FAILURE_WINDOW);
        if should_alert {
            entry.last_alert = Some(now);
        }
        (failures, distinct_accounts, should_alert)
    };

    if should_alert {
        let kind = if distinct_accounts >= IP_DISTINCT_ACCOUNTS_ALERT_THRESHOLD {
            "credential_stuffing"
        } else {
            "brute_force"
        };
        record_auth_security_event(endpoint, kind);
        tracing::error!(
            target: "security_alert",
            alert = kind,
            endpoint,
            client_ip,
            failures,
            distinct_accounts,
            window_seconds = IP_FAILURE_WINDOW.as_secs(),
            "Suspicious authentication pattern detected"
        );
    }
}

#[derive(Deserialize)]
struct CaptchaVerifyResponse {
    success: bool,
}

/// Verify a CAPTCHA response token with the configured provider.
///
/// hCaptcha, reCAPTCHA and Cloudflare Turnstile share the same verify API; configure
/// with `AUTH_CAPTCHA_SECRET` and optionally `AUTH_CAPTCHA_VERIFY_URL` (defaults to
/// hCaptcha). Without a secret the token cannot be checked, so only its presence is
/// enforced (the behaviour before this was added).
pub async fn verify_captcha(token: &str, client_ip: &str) -> bool {
    let Some(secret) = std::env::var("AUTH_CAPTCHA_SECRET")
        .ok()
        .filter(|s| !s.is_empty())
    else {
        tracing::warn!("AUTH_CAPTCHA_SECRET not set: CAPTCHA token accepted without verification");
        return true;
    };
    let verify_url = std::env::var("AUTH_CAPTCHA_VERIFY_URL")
        .unwrap_or_else(|_| "https://hcaptcha.com/siteverify".to_string());

    // Build an application/x-www-form-urlencoded body without the reqwest `form` feature.
    let mut encoder = reqwest::Url::parse("http://localhost/").expect("static URL is valid");
    encoder
        .query_pairs_mut()
        .append_pair("secret", &secret)
        .append_pair("response", token);
    if client_ip != "unknown" {
        encoder.query_pairs_mut().append_pair("remoteip", client_ip);
    }
    let body = encoder.query().unwrap_or_default().to_string();

    let response = HTTP
        .post(&verify_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await;

    match response {
        Ok(resp) => match resp.json::<CaptchaVerifyResponse>().await {
            Ok(parsed) => parsed.success,
            Err(e) => {
                tracing::error!("Failed to parse CAPTCHA verification response: {e}");
                false
            }
        },
        Err(e) => {
            tracing::error!("CAPTCHA verification request failed: {e}");
            false
        }
    }
}
