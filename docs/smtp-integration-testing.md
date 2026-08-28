# SMTP Integration Testing Guide

Closes #1859 — confirms end-to-end email delivery through a real SMTP connection
rather than just verifying digest-content generation.

## Production SMTP provider

The production SMTP provider is configured via environment variables in
`backend/.env.example` (section **SMTP / Email Configuration**).  Supported
providers and example settings:

| Provider | `SMTP_HOST` | `SMTP_USER` | `SMTP_PASS` |
|---|---|---|---|
| SendGrid | `smtp.sendgrid.net` | `apikey` | `SG.xxxxxxxx` |
| Postmark | `smtp.postmarkapp.com` | `<account-token>` | `<account-token>` |
| AWS SES | `email-smtp.<region>.amazonaws.com` | IAM SMTP credential | IAM SMTP credential |

Port 587 (STARTTLS) is the default. Change `SMTP_PORT` if your provider requires 465 (SMTPS).

## Local SMTP stub for CI (Mailpit)

[Mailpit](https://github.com/axllent/mailpit) is a lightweight Mailhog-compatible
SMTP stub with a web UI.  It accepts all mail without auth and exposes a REST API
for assertions.

### Docker Compose snippet

Add this service to your test compose file (or use the existing
`docker-compose.jaeger.yml` pattern):

```yaml
services:
  mailpit:
    image: axllent/mailpit:latest
    ports:
      - "1025:1025"   # SMTP (no auth, plain)
      - "8025:8025"   # HTTP REST + Web UI
    environment:
      MP_MAX_MESSAGES: 500
```

### Backend configuration

```bash
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_USER=test@example.com
SMTP_PASS=nopassword
DIGEST_RECIPIENTS=recipient@example.com
```

### Triggering a test digest

The `DigestScheduler::send_digest` method is callable directly.  To exercise it
end-to-end without waiting for the weekly cron:

```bash
# With the backend running in RPC_MOCK_MODE=true and Mailpit up:
curl -X POST http://localhost:8080/api/internal/digest/send \
  -H "Authorization: Bearer <admin-token>" \
  -d '{"period":"Test"}'
```

Or from a Rust integration test (no live SMTP required in unit tests):

```rust
// backend/tests/smtp_integration.rs  (requires Mailpit on localhost:1025)
// Run with: cargo test --test smtp_integration -- --ignored
#[tokio::test]
#[ignore = "requires local Mailpit SMTP stub on port 1025"]
async fn test_digest_delivery_via_mailpit() {
    use stellar_insights_backend::email::service::EmailService;
    use stellar_insights_backend::email::report::{DigestReport, generate_html_report};

    let service = EmailService::new(
        "localhost".to_string(),
        "test@example.com".to_string(),
        "nopassword".to_string(),
    );

    let report = DigestReport {
        period: "Test".to_string(),
        top_corridors: vec![],
        top_anchors: vec![],
        total_volume: 0.0,
        avg_success_rate: 100.0,
    };
    let html = generate_html_report(&report);

    service
        .send_html("inbox@example.com", "Test digest", &html)
        .expect("SMTP send should succeed against local Mailpit stub");

    // Assert delivery via Mailpit REST API
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get("http://localhost:8025/api/v1/messages")
        .send()
        .await
        .expect("Mailpit API reachable")
        .json()
        .await
        .expect("valid JSON");

    let count = resp["total"].as_u64().unwrap_or(0);
    assert!(count >= 1, "Expected at least 1 delivered message, got {count}");
}
```

### Verifying delivery in the Mailpit UI

Open http://localhost:8025 while the test backend is running.  Every email
dispatched by `EmailService::send_html` will appear there with full headers,
HTML body preview, and raw source.

## Failure handling

If the SMTP connection fails or a recipient address is invalid, `send_digest`
logs the error at `ERROR` level (structured field `error`) and **continues to
the next recipient**.  It does not abort the digest run or return an error to
the scheduler.  The next scheduled run (weekly / monthly) is the natural retry.

For guaranteed delivery, wire up a persistent `scheduled_emails` table: on send
failure insert a row with the recipient and digest content; a separate worker
polls and retries with exponential backoff.
