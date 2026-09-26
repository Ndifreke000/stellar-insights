//! Request/response access logging middleware (issues #1202, #2408).
//!
//! Emits one structured access-log event per HTTP request under the
//! `api_access` target, with field names matching the Logstash pipeline
//! (`elk/logstash/pipeline/logstash.conf`):
//!
//! - `request_id`, `correlation_id` (from `request_id_middleware`)
//! - `http_method`, `http_path`, `http_query` (sensitive params redacted)
//! - `http_status`, `response_time_ms`, `request_bytes`, `response_bytes`
//! - `client_ip` (redacted), `user_agent`, `instance_id` (which replica served it)
//!
//! 5xx are logged at ERROR, 4xx at WARN, everything else at INFO.
//!
//! Configuration (environment, read once at startup):
//!
//! | Variable                 | Default                         | Meaning |
//! |--------------------------|---------------------------------|---------|
//! | `API_LOG_SAMPLE_RATE`    | `1.0`                           | Fraction of successful, fast requests logged. Errors and slow requests are always logged. |
//! | `API_LOG_SLOW_MS`        | `1000`                          | Requests at least this slow are always logged. |
//! | `API_LOG_SKIP_PATHS`     | `/health,/metrics,/ready,/live` | Comma-separated exact paths never logged (probes, scrapes). |
//! | `API_LOG_BODIES`         | `false`                         | Also log request/response bodies (redacted, truncated) under `api_access_body`. |
//! | `API_LOG_MAX_BODY_BYTES` | `2048`                          | Truncate logged bodies to this many bytes. |
//!
//! Bodies are only captured for textual content types with a known size of at
//! most 64 KiB, so streaming responses (SSE, downloads) are never buffered.
//! Authorization, cookies and secret-looking JSON/form/query fields are never
//! logged in clear text.

use axum::{
    body::{Body, HttpBody},
    extract::{ConnectInfo, Request},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use http_body_util::BodyExt;
use std::net::SocketAddr;
use std::sync::OnceLock;
use std::time::Instant;

use crate::logging::redaction::redact_ip;
use crate::request_id::{CorrelationId, RequestId};

/// Never buffer bodies larger than this, even when body logging is enabled.
const MAX_BUFFER_BYTES: u64 = 64 * 1024;

/// Keys (matched case-insensitively as substrings) whose values are redacted
/// in JSON bodies, form bodies and query strings.
const SENSITIVE_KEYS: &[&str] = &[
    "password",
    "passwd",
    "secret",
    "token",
    "apikey",
    "api_key",
    "authorization",
    "private",
    "seed",
    "mnemonic",
    "signature",
    "jwt",
    "cookie",
    "credential",
    "session",
];

const REDACTED: &str = "[REDACTED]";

struct LogConfig {
    sample_rate: f64,
    slow_ms: u128,
    skip_paths: Vec<String>,
    log_bodies: bool,
    max_body_log_bytes: usize,
}

fn config() -> &'static LogConfig {
    static CONFIG: OnceLock<LogConfig> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let var = |name: &str| std::env::var(name).ok();
        LogConfig {
            sample_rate: var("API_LOG_SAMPLE_RATE")
                .and_then(|v| v.parse::<f64>().ok())
                .map_or(1.0, |r| r.clamp(0.0, 1.0)),
            slow_ms: var("API_LOG_SLOW_MS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            skip_paths: var("API_LOG_SKIP_PATHS")
                .unwrap_or_else(|| "/health,/metrics,/ready,/live".to_string())
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(String::from)
                .collect(),
            log_bodies: var("API_LOG_BODIES")
                .is_some_and(|v| v.eq_ignore_ascii_case("true") || v == "1"),
            max_body_log_bytes: var("API_LOG_MAX_BODY_BYTES")
                .and_then(|v| v.parse().ok())
                .unwrap_or(2048),
        }
    })
}

pub async fn request_response_logging_middleware(req: Request<Body>, next: Next) -> Response {
    let cfg = config();
    let path = req.uri().path().to_string();
    if cfg.skip_paths.iter().any(|p| p == &path) {
        return next.run(req).await;
    }

    let start = Instant::now();

    let request_id = req
        .extensions()
        .get::<RequestId>()
        .map_or_else(|| "unknown".to_string(), |r| r.0.clone());
    let correlation_id = req
        .extensions()
        .get::<CorrelationId>()
        .map_or_else(|| request_id.clone(), |c| c.0.clone());
    let method = req.method().to_string();
    let query = req.uri().query().map(redact_query).unwrap_or_default();
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0);
    let client_ip = crate::client_ip::client_ip(req.headers(), peer)
        .map_or_else(|| "unknown".to_string(), |ip| redact_ip(&ip.to_string()));
    let user_agent = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .chars()
        .take(256)
        .collect::<String>();
    let request_bytes = req.body().size_hint().exact();

    tracing::debug!(
        target: "api_access",
        http_method = %method,
        http_path = %path,
        "request started"
    );

    let (req, request_body) = if cfg.log_bodies {
        capture_request_body(req, cfg.max_body_log_bytes).await
    } else {
        (req, None)
    };

    let response = next.run(req).await;
    let latency_ms = start.elapsed().as_millis();
    let status = response.status();
    let response_bytes = response.body().size_hint().exact();

    // Errors and slow requests are always logged; the rest can be sampled.
    let always_log = status.is_client_error()
        || status.is_server_error()
        || latency_ms >= cfg.slow_ms;
    if !always_log && cfg.sample_rate < 1.0 && rand::random::<f64>() >= cfg.sample_rate {
        return response;
    }

    let (response, response_body) = if cfg.log_bodies {
        capture_response_body(response, cfg.max_body_log_bytes).await
    } else {
        (response, None)
    };

    let http_status = status.as_u16();
    macro_rules! access_log {
        ($level:ident) => {
            tracing::$level!(
                target: "api_access",
                request_id = %request_id,
                correlation_id = %correlation_id,
                http_method = %method,
                http_path = %path,
                http_query = %query,
                http_status,
                response_time_ms = latency_ms as u64,
                request_bytes,
                response_bytes,
                client_ip = %client_ip,
                user_agent = %user_agent,
                instance_id = crate::distributed_lock::instance_id(),
                "{} {} {}",
                method,
                path,
                http_status
            )
        };
    }
    if status.is_server_error() {
        access_log!(error);
    } else if status.is_client_error() {
        access_log!(warn);
    } else {
        access_log!(info);
    }

    if request_body.is_some() || response_body.is_some() {
        tracing::info!(
            target: "api_access_body",
            request_id = %request_id,
            http_method = %method,
            http_path = %path,
            http_status,
            request_body = request_body.as_deref().unwrap_or(""),
            response_body = response_body.as_deref().unwrap_or(""),
            "request/response bodies"
        );
    }

    response
}

async fn capture_request_body(req: Request<Body>, max_log: usize) -> (Request<Body>, Option<String>) {
    if !should_capture(req.headers(), req.body()) {
        return (req, None);
    }
    let content_type = content_type(req.headers());
    let (parts, body) = req.into_parts();
    match body.collect().await {
        Ok(collected) => {
            let bytes = collected.to_bytes();
            let snippet = render_body(&bytes, &content_type, max_log);
            (Request::from_parts(parts, Body::from(bytes)), Some(snippet))
        }
        // The body stream failed; the handler would have seen the same error.
        Err(_) => (Request::from_parts(parts, Body::empty()), None),
    }
}

async fn capture_response_body(response: Response, max_log: usize) -> (Response, Option<String>) {
    // 101 Switching Protocols (WebSocket) and bodiless statuses have nothing to log.
    if response.status() == StatusCode::SWITCHING_PROTOCOLS
        || !should_capture(response.headers(), response.body())
    {
        return (response, None);
    }
    let content_type = content_type(response.headers());
    let (parts, body) = response.into_parts();
    match body.collect().await {
        Ok(collected) => {
            let bytes = collected.to_bytes();
            let snippet = render_body(&bytes, &content_type, max_log);
            (Response::from_parts(parts, Body::from(bytes)), Some(snippet))
        }
        Err(_) => (Response::from_parts(parts, Body::empty()), None),
    }
}

/// Only buffer small, textual bodies whose size is known up front. Unknown
/// sizes indicate a stream (SSE, file download) that must not be buffered.
fn should_capture(headers: &HeaderMap, body: &Body) -> bool {
    let size_ok = body
        .size_hint()
        .exact()
        .is_some_and(|n| n > 0 && n <= MAX_BUFFER_BYTES);
    size_ok && is_textual(&content_type(headers))
}

fn content_type(headers: &HeaderMap) -> String {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_textual(content_type: &str) -> bool {
    content_type.contains("json")
        || (content_type.starts_with("text/") && !content_type.starts_with("text/event-stream"))
        || content_type.starts_with("application/x-www-form-urlencoded")
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    SENSITIVE_KEYS.iter().any(|s| key.contains(s))
}

/// Redact sensitive values in a URL query string / form body.
fn redact_query(query: &str) -> String {
    let mut out = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
        if is_sensitive_key(&key) {
            out.append_pair(&key, REDACTED);
        } else {
            out.append_pair(&key, &value);
        }
    }
    out.finish()
}

fn redact_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, v) in map.iter_mut() {
                if is_sensitive_key(key) {
                    *v = serde_json::Value::String(REDACTED.to_string());
                } else {
                    redact_json(v);
                }
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(redact_json),
        _ => {}
    }
}

/// Redact and truncate a body for logging.
fn render_body(bytes: &[u8], content_type: &str, max_len: usize) -> String {
    let rendered = if content_type.contains("json") {
        match serde_json::from_slice::<serde_json::Value>(bytes) {
            Ok(mut json) => {
                redact_json(&mut json);
                json.to_string()
            }
            // Unparseable JSON may still contain secrets; don't echo it.
            Err(_) => return "[unparseable json body]".to_string(),
        }
    } else if content_type.starts_with("application/x-www-form-urlencoded") {
        redact_query(&String::from_utf8_lossy(bytes))
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    truncate(rendered, max_len)
}

fn truncate(mut s: String, max_len: usize) -> String {
    if s.len() <= max_len {
        return s;
    }
    let mut cut = max_len;
    while !s.is_char_boundary(cut) {
        cut -= 1;
    }
    s.truncate(cut);
    s.push_str("…[truncated]");
    s
}
