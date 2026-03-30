# Backend Input Validation Issues

## Priority: High

### Overview
Multiple backend API endpoints lack proper input validation and rate limiting, creating security vulnerabilities and potential for abuse.

---

## corridors.rs:83 - Pattern Matching Without Validation

### ❌ Issue Location: Lines 83-99

```rust
"payment" | _ => {
    // Regular payments: same asset for source and destination
    let asset = if payment.asset_type == "native" {
        "XLM:native".to_string()
    } else {
        format!(
            "{}:{}",
            payment.get_asset_code().as_deref().unwrap_or("UNKNOWN"),
            payment.get_asset_issuer().as_deref().unwrap_or("unknown")
        )
    };
```

### Security Issues:

#### 1. **Unvalidated Pattern Matching**
- **Problem**: Wildcard `_` catch-all without validation
- **Risk**: Unexpected operation types processed incorrectly
- **Impact**: Data corruption, logic errors

#### 2. **Asset Code Validation Missing**
- **Problem**: `get_asset_code()` returns `Option<&str>` but not validated
- **Current**: Uses `unwrap_or("UNKNOWN")` - insecure fallback
- **Risk**: Invalid asset codes processed, potential injection

#### 3. **Asset Issuer Validation Missing**
- **Problem**: `get_asset_issuer()` not validated for format
- **Current**: Uses `unwrap_or("unknown")` - invalid issuer
- **Risk**: Invalid Stellar addresses processed

#### 4. **Asset Type Validation Missing**
- **Problem**: `payment.asset_type` compared to string literal
- **Risk**: Case sensitivity issues, unexpected values

### Recommended Fix:
```rust
// Validate operation type first
match payment.operation_type.as_deref() {
    Some("payment") => {
        // Validate asset type
        let asset = match payment.asset_type.as_str() {
            "native" => "XLM:native".to_string(),
            // Validate non-native assets
            asset_type if !asset_type.is_empty() => {
                let code = payment.get_asset_code()
                    .and_then(|c| validate_asset_code(c))
                    .ok_or_else(|| anyhow::anyhow!("Invalid asset code"))?;
                let issuer = payment.get_asset_issuer()
                    .and_then(|i| validate_stellar_address(i))
                    .ok_or_else(|| anyhow::anyhow!("Invalid asset issuer"))?;
                format!("{}:{}", code, issuer)
            }
            _ => return Err(anyhow::anyhow!("Invalid asset type")),
        };
        // ... rest of logic
    }
    Some(op) => return Err(anyhow::anyhow!("Unsupported operation: {}", op)),
    None => return Err(anyhow::anyhow!("Missing operation type")),
}
```

---

## api_keys.rs - Missing Rate Limiting

### ❌ Issues:

#### 1. **API Key Rotation Not Rate-Limited**
- **Location**: `rotate_api_key()` function (lines 127-145)
- **Problem**: No rate limiting on key rotation
- **Risk**: 
  - Abuse: Rapid key rotations to bypass usage limits
  - DoS: Excessive database operations
  - Security: Circumvent key revocation mechanisms

#### 2. **API Key Creation Not Rate-Limited**
- **Location**: `create_api_key()` function (lines 35-52)
- **Problem**: No rate limiting on key creation
- **Risk**:
  - Spam: Create unlimited API keys
  - Resource exhaustion: Database storage abuse
  - Attack surface: More keys to compromise

#### 3. **Missing Input Validation**
- **Problem**: Only basic name validation (line 42-44)
- **Missing**:
  - Wallet address format validation
  - API key name length limits
  - Special character restrictions

### Current Validation (Insufficient):
```rust
if req.name.trim().is_empty() {
    return Err(ApiKeyError::BadRequest("Key name is required".to_string()));
}
```

### Recommended Improvements:

#### 1. Add Rate Limiting:
```rust
use tower::limit::RateLimitLayer;
use tower::ServiceBuilder;

// Apply rate limiting middleware
let app = Router::new()
    .route("/api/api-keys", post(create_api_key))
    .layer(
        ServiceBuilder::new()
            .layer(RateLimitLayer::new(5, std::time::Duration::from_secs(60)) // 5 requests per minute
    )
```

#### 2. Enhanced Input Validation:
```rust
fn validate_api_key_request(req: &CreateApiKeyRequest) -> Result<(), ApiKeyError> {
    // Name validation
    if req.name.trim().is_empty() {
        return Err(ApiKeyError::BadRequest("Key name is required".to_string()));
    }
    
    if req.name.len() > 100 {
        return Err(ApiKeyError::BadRequest("Key name too long (max 100 chars)".to_string()));
    }
    
    // Validate no special characters that could cause issues
    if req.name.contains(['<', '>', '&', '"', '\'', '\\']) {
        return Err(ApiKeyError::BadRequest("Key name contains invalid characters".to_string()));
    }
    
    Ok(())
}
```

---

## alerts.rs - Missing Rate Limiting

### ❌ Issues:

#### 1. **Alert Operations Not Rate-Limited**
- **Problem**: All alert endpoints lack rate limiting
- **Affected endpoints**:
  - `POST /api/alerts/rules` (create_rule)
  - `PUT /api/alerts/rules/{id}` (update_rule)
  - `DELETE /api/alerts/rules/{id}` (delete_rule)
  - `POST /api/alerts/history/{id}/snooze` (snooze_rule_from_history)

#### 2. **WebSocket Connection Abuse**
- **Location**: `alert_websocket_handler()` (lines 244-249)
- **Problem**: No connection limits per user
- **Risk**:
  - DoS: Excessive WebSocket connections
  - Resource exhaustion: Memory/CPU usage
  - Abuse: Spam alert subscriptions

#### 3. **Missing Input Validation**
- **Problem**: Path parameters not validated
- **Risk**: Invalid UUIDs, SQL injection potential

### Current Issues:
```rust
// No validation of path parameter `id`
async fn update_rule(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>, // Should be validated UUID
    Json(payload): Json<UpdateAlertRuleRequest>,
) -> ApiResult<impl IntoResponse> {
```

### Recommended Improvements:

#### 1. Add Rate Limiting:
```rust
// Different limits for different operations
let alert_rate_limits = ServiceBuilder::new()
    .layer(RateLimitLayer::new(10, std::time::Duration::from_secs(60)) // 10 operations per minute
    .layer(RateLimitLayer::new(100, std::time::Duration::from_secs(3600))); // 100 per hour
```

#### 2. UUID Validation:
```rust
use uuid::Uuid;

fn validate_uuid(id: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(id).map_err(|_| {
        ApiError::BadRequest("Invalid ID format".to_string())
    })
}

async fn update_rule(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAlertRuleRequest>,
) -> ApiResult<impl IntoResponse> {
    let rule_id = validate_uuid(&id)?;
    // ... rest of function
}
```

#### 3. WebSocket Connection Limits:
```rust
// Track connections per user
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

static WS_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
const MAX_WS_CONNECTIONS: usize = 1000;

pub async fn alert_websocket_handler(
    ws: WebSocketUpgrade,
    State(alert_manager): State<Arc<AlertManager>>,
) -> Response {
    // Check global limit
    if WS_CONNECTIONS.load(Ordering::Relaxed) >= MAX_WS_CONNECTIONS {
        return (StatusCode::SERVICE_UNAVAILABLE, "Too many connections").into_response();
    }
    
    WS_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
    
    ws.on_upgrade(|socket| {
        handle_alert_socket(socket, alert_manager);
        WS_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    })
}
```

---

## Security Impact Assessment

### High Risk Issues:
1. **API Key Abuse**: Unlimited rotations can bypass security controls
2. **Resource DoS**: No rate limiting enables resource exhaustion
3. **Data Integrity**: Unvalidated pattern matching can corrupt data

### Medium Risk Issues:
1. **WebSocket Abuse**: Connection flooding
2. **Input Injection**: Poorly validated parameters
3. **Logic Bypass**: Wildcard pattern matching

### Compliance Impact:
- **OWASP Top 10**: 
  - A01: Broken Access Control (rate limiting)
  - A03: Injection (input validation)
  - A05: Security Misconfiguration

---

## Recommended Implementation Priority

### Phase 1 (Critical - Immediate):
1. **Add rate limiting to API key operations**
2. **Fix pattern matching validation in corridors.rs**
3. **Add UUID validation to all path parameters**

### Phase 2 (High - Next Sprint):
1. **Rate limiting for alert operations**
2. **WebSocket connection limits**
3. **Enhanced input validation for all endpoints**

### Phase 3 (Medium - Future):
1. **Request size limits**
2. **Input sanitization**
3. **Comprehensive audit logging**

---

## Configuration Recommendations

### Rate Limiting Configuration:
```yaml
rate_limits:
  api_keys:
    create: "5 per minute"
    rotate: "10 per minute"
    list: "100 per minute"
  
  alerts:
    crud_operations: "20 per minute"
    websocket_connections: "5 per user"
  
  global:
    requests_per_ip: "1000 per hour"
    concurrent_connections: 1000
```

### Validation Rules:
```yaml
validation:
  uuid: strict_uuid_format
  asset_code: "^[A-Z0-9]{1,12}$"
  stellar_address: "^G[A-Z0-9]{56}$"
  api_key_name: "^[a-zA-Z0-9\\s\\-_]{1,100}$"
```
