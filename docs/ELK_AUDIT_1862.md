# ELK Pipeline Audit — Issue #1862

## Summary

This document records the findings of the end-to-end ELK logging pipeline audit
requested in issue #1862.

---

## 1. Pipeline architecture

```
Backend (Rust / tracing JSON)
    │
    ├── TCP port 5000 ──────────────────────────────────┐
    │   (json_lines codec)                              │
    └── Filebeat (container / file tailing) → port 5044┤
                                                        ▼
                                                   Logstash
                                                        │
                                                        ▼
                                               Elasticsearch
                                                        │
                                                        ▼
                                                    Kibana
```

The backend emits structured JSON logs via `tracing-subscriber`'s JSON layer
(`backend/src/logging.rs`).  Logstash receives them on TCP 5000 and Filebeat
ships container logs on Beats port 5044.

---

## 2. What was audited

### 2a. docker-compose.elk.yml vs docker-compose.elk.prod.yml

| Concern | Dev compose | Prod compose (before fix) | Prod compose (after fix) |
|---|---|---|---|
| `xpack.security.enabled` | `false` | `true` ✓ | `true` ✓ |
| Password enforcement | `changeme` hardcoded | `changeme` default kept | `${ELASTIC_PASSWORD:?...}` — fails fast if unset |
| Elasticsearch heap | 512 m | 2 g ✓ | 2 g ✓ |
| Filebeat service | present | **absent** ✗ | added ✓ |
| Resource limits (`deploy.resources`) | absent | absent ✗ | added (ES 4 g, LS 2 g, Kibana 1 g, Filebeat 256 m) ✓ |
| Port binding | `0.0.0.0` | `0.0.0.0` ✗ | `127.0.0.1` (loopback) ✓ |
| ILM config volume | absent | absent ✗ | `ilm-policy.json` mounted ✓ |
| Logstash monitoring auth | absent | `logstash-prod.yml` referenced ✓ | `logstash-prod.yml` referenced ✓ |
| Kibana health-check | present | absent ✗ | added ✓ |
| `restart: unless-stopped` | absent | absent ✗ | added on all services ✓ |

The prod compose was previously almost a copy of the dev compose with only the
`xpack.security` flag and heap size changed — it was missing Filebeat, resource
limits, restart policies, and port hardening.

### 2b. Logstash pipeline redaction (elk/logstash/pipeline/logstash.conf)

Logstash already drops several field names in its `mutate { remove_field }` block:

```
remove_field => ["password", "api_key", "secret", "token", "authorization"]
```

This is a server-side safety net.  However, it only fires on fields that are
present at the top level of the JSON document.  Nested fields (e.g.
`request.headers.authorization`) are **not** removed by this rule.

**Recommendation:** extend the `remove_field` list to include common nested
patterns, or use a `ruby` filter for recursive scrubbing of header fields.

### 2c. Rust call-site audit for unredacted secrets

Audit scope: `backend/src/**/*.rs`.

| Finding | Location | Status |
|---|---|---|
| `PRICE_FEED_API_KEY` logged at startup | `env_config.rs:167` | ✓ already redacted (`[REDACTED]`) |
| `TELEGRAM_BOT_TOKEN` logged at startup | `env_config.rs:177` | ✓ already redacted (`[REDACTED]`) |
| `client_id` in JWT middleware | `middleware/jwt_token_refresh.rs:126,134` | ⚠ logged in plaintext. `client_id` is a non-secret identifier, but if it maps to a user it should use `redact_user_id()`. Low risk. |
| Cache key logged on hit/miss | `cache.rs:168,193,205`, `cache/helpers.rs:208,212` | ⚠ cache keys sometimes include account IDs or corridor identifiers. Evaluate whether keys need partial masking. |
| `log_secure!` macro usage | Only in its own doctest (`logging.rs:89`) | ✗ The macro exists but is not used in any production call site. All secure-field logging goes through `tracing::` directly with `Redacted()` wrappers. |

**The `log_secure!` macro and `Redacted` wrapper are available but adoption is
incomplete.** The highest-risk fields (API keys, tokens) are redacted at the
startup dump in `env_config.rs`, but the broader logging surface (cache key
logging, request body logging) is not systematically using `Redacted`.

No evidence of JWT payloads, raw API keys, or Stellar secret keys being written
to logs in plaintext was found in the audited source.

---

## 3. Confirming logs reach Kibana

To run the end-to-end check manually:

```bash
# 1. Start the ELK stack (dev)
docker compose -f docker-compose.elk.yml up -d

# 2. Wait for health (≈60 s)
docker compose -f docker-compose.elk.yml ps

# 3. Start the backend with TCP log forwarding enabled
# The backend emits JSON to stdout; pipe via netcat or configure
# RUST_LOG=info and point Filebeat at the container's stdout.
# Alternatively, send a test log directly:
echo '{"level":"info","message":"elk-test","timestamp":"'$(date -u +%FT%TZ)'"}' \
  | nc -q1 localhost 5000

# 4. Verify in Elasticsearch
curl -s http://localhost:9200/stellar-insights-*/_search?size=1 \
  | python3 -m json.tool | grep -A2 '"message"'

# 5. Spot-check redaction: the following fields must NOT appear in any document
curl -s 'http://localhost:9200/stellar-insights-*/_search' \
  -H 'Content-Type: application/json' \
  -d '{"query":{"exists":{"field":"password"}}}' \
  | python3 -m json.tool
# Expected: "total": { "value": 0 }
```

---

## 4. Remaining work

- [ ] Extend Logstash `remove_field` to cover nested `headers.*` and
      `request.body.*` fields.
- [ ] Audit `cache.rs` cache-key logging and apply `redact_account()` where
      keys embed Stellar account IDs.
- [ ] Add at least one production call site that uses `log_secure!` to
      demonstrate the macro works end-to-end.
- [ ] Configure an ILM policy in Kibana after first boot to enforce 30-day /
      50 GB retention on `stellar-insights-*` indices
      (template: `elk/elasticsearch/config/ilm-policy.json`).
