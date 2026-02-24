# Rust Documentation Standards

## Overview

All public APIs (`pub fn`, `pub struct`, `pub enum`, `pub trait`) must have documentation comments using `///`. This ensures:
- IDE hover tooltips work properly
- API usage is clear without reading implementation
- Automated documentation generation works
- Reduced support burden

**Current Coverage: ~26% (89/342 functions)**  
**Target: 100%**

---

## Required Sections

### 1. Summary (Required)
One-line description of what the function/type does.

```rust
/// Retrieves an anchor by its unique identifier.
```

### 2. Arguments (Required for functions with parameters)
Document each parameter with its purpose.

```rust
/// # Arguments
///
/// * `id` - The UUID of the anchor to retrieve
/// * `limit` - Maximum number of results (default: 50, max: 200)
```

### 3. Returns (Required for functions)
Document all possible return values.

```rust
/// # Returns
///
/// * `Ok(Some(Anchor))` - Anchor found and returned
/// * `Ok(None)` - No anchor exists with the given ID
/// * `Err(DomainError::DatabaseError)` - Database query failed
```

### 4. Examples (Recommended)
At least one usage example.

```rust
/// # Examples
///
/// ```rust
/// let anchor_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
/// let anchor = db.get_anchor_by_id(anchor_id).await?;
///
/// match anchor {
///     Some(a) => println!("Found: {}", a.name),
///     None => println!("Not found"),
/// }
/// ```
```

### 5. Errors (Required if function returns Result)
Document error conditions.

```rust
/// # Errors
///
/// Returns `DomainError::CorridorNotFound` if no corridor exists with the given ID.
/// Returns `DomainError::DatabaseError` if the database query fails.
```

---

## Optional Sections

### Performance
Document performance characteristics for critical paths.

```rust
/// # Performance
///
/// This query is indexed on the `id` column and typically completes in <1ms.
/// For large result sets (>1000 rows), consider using pagination.
```

### Side Effects
Document state changes, I/O operations, or external interactions.

```rust
/// # Side Effects
///
/// - Updates the `updated_at` timestamp
/// - Invalidates related cache entries
/// - Triggers metric history recording
/// - Sends webhook notifications if configured
```

### Panics
Document conditions that cause panics.

```rust
/// # Panics
///
/// Panics if the internal mutex is poisoned.
```

### Safety
For `unsafe` functions only.

```rust
/// # Safety
///
/// Caller must ensure the pointer is valid and properly aligned.
```

---

## Complete Template

```rust
/// [One-line summary of what this does]
///
/// [Optional: More detailed explanation if the function is complex]
///
/// # Arguments
///
/// * `param1` - Description of first parameter
/// * `param2` - Description of second parameter
///
/// # Returns
///
/// * `Ok(T)` - Success case description
/// * `Err(E)` - Error case description
///
/// # Examples
///
/// ```rust
/// // Example usage
/// let result = function_name(arg1, arg2).await?;
/// ```
///
/// # Errors
///
/// Returns error if [condition].
///
/// # Performance
///
/// [Optional: Performance notes]
///
/// # Side Effects
///
/// [Optional: State changes, I/O, etc.]
pub async fn function_name(param1: Type1, param2: Type2) -> Result<T, E> {
    // Implementation
}
```

---

## Examples by Category

### Database Functions

```rust
/// Retrieves an anchor by its unique identifier.
///
/// # Arguments
///
/// * `id` - The UUID of the anchor to retrieve
///
/// # Returns
///
/// * `Ok(Some(Anchor))` - Anchor found and returned
/// * `Ok(None)` - No anchor exists with the given ID
/// * `Err(_)` - Database query failed
///
/// # Examples
///
/// ```rust
/// let anchor_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
/// let anchor = db.get_anchor_by_id(anchor_id).await?;
/// ```
///
/// # Performance
///
/// Indexed query, typically <1ms.
pub async fn get_anchor_by_id(&self, id: Uuid) -> Result<Option<Anchor>> {
    // Implementation
}
```

### API Endpoints

```rust
/// List all payment corridors with filtering and pagination.
///
/// Returns a paginated list of payment corridors sorted by health score.
/// Supports filtering by success rate, volume, asset code, and time period.
///
/// # Query Parameters
///
/// * `limit` - Maximum results (default: 50, max: 200)
/// * `offset` - Results to skip (default: 0)
/// * `sort_by` - Sort field: `success_rate` or `volume`
/// * `success_rate_min` - Minimum success rate (0-100)
/// * `asset_code` - Filter by asset (e.g., "USDC")
///
/// # Response
///
/// Returns `CorridorsResponse` with:
/// - `corridors`: Array of corridor objects
/// - `total`: Total matching corridors
/// - `limit`: Applied limit
/// - `offset`: Applied offset
///
/// # Examples
///
/// ```bash
/// # Get top 10 corridors
/// curl "http://localhost:8080/api/corridors?limit=10"
///
/// # Filter by success rate
/// curl "http://localhost:8080/api/corridors?success_rate_min=95"
/// ```
///
/// # Errors
///
/// * `400 Bad Request` - Invalid query parameters
/// * `429 Too Many Requests` - Rate limit exceeded
/// * `500 Internal Server Error` - Database or RPC error
///
/// # Caching
///
/// Responses cached for 5 minutes. Use `Cache-Control: no-cache` to bypass.
///
/// # Rate Limiting
///
/// Limited to 100 requests per minute per IP.
pub async fn get_corridors(/* ... */) -> ApiResult<Json<CorridorsResponse>> {
    // Implementation
}
```

### Service Functions

```rust
/// Computes corridor metrics from a list of payment records.
///
/// Calculates success rate, total volume, average settlement time,
/// and health score for each unique corridor in the payment set.
///
/// # Arguments
///
/// * `payments` - Slice of payment records to analyze
///
/// # Returns
///
/// Vector of `CorridorMetrics`, one per unique corridor found.
/// Returns empty vector if no payments provided.
///
/// # Examples
///
/// ```rust
/// let payments = vec![payment1, payment2, payment3];
/// let metrics = compute_metrics_from_payments(&payments);
///
/// for metric in metrics {
///     println!("Corridor: {}, Success Rate: {}%",
///              metric.corridor_key, metric.success_rate);
/// }
/// ```
///
/// # Performance
///
/// O(n) where n is the number of payments. Uses HashMap for grouping.
pub fn compute_metrics_from_payments(payments: &[PaymentRecord]) -> Vec<CorridorMetrics> {
    // Implementation
}
```

### Utility Functions

```rust
/// Checks if a string is a valid Stellar muxed address.
///
/// Muxed addresses start with 'M' and are 69 characters long.
///
/// # Arguments
///
/// * `addr` - The address string to validate
///
/// # Returns
///
/// `true` if the address is a valid muxed address format, `false` otherwise.
///
/// # Examples
///
/// ```rust
/// assert!(is_muxed_address("MAAAAAAAAAAAAAB7BQ2L7E5NBWMXDUCMZSIPOBKRDSBYVLMXGSSKF6YNPIB7Y77ITKNOG"));
/// assert!(!is_muxed_address("GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H"));
/// ```
pub fn is_muxed_address(addr: &str) -> bool {
    // Implementation
}
```

### Struct/Enum Documentation

```rust
/// Configuration for database connection pool.
///
/// Controls connection lifecycle, timeouts, and pool size limits.
/// Use `PoolConfig::from_env()` to load from environment variables.
///
/// # Examples
///
/// ```rust
/// let config = PoolConfig {
///     max_connections: 20,
///     min_connections: 5,
///     connect_timeout_seconds: 30,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of idle connections to maintain
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_seconds: u64,
}
```

---

## Testing Documentation

Run these commands to check documentation coverage:

```bash
# Generate documentation and open in browser
cargo doc --no-deps --open

# Fail build on missing docs (add to CI)
cargo rustdoc -- -D missing-docs

# Check for missing docs with clippy
cargo clippy -- -W clippy::missing_docs_in_private_items

# Count documentation coverage
echo "Total public functions:"
rg "^pub fn" backend/src/ | wc -l

echo "Documented functions:"
rg "^/// " backend/src/ | wc -l
```

---

## Priority Order

Document in this order:

1. **Public API endpoints** (`backend/src/api/*.rs`)
2. **Database functions** (`backend/src/database.rs`)
3. **Service layer** (`backend/src/services/*.rs`)
4. **RPC client** (`backend/src/rpc/*.rs`)
5. **Utility functions** (`backend/src/*.rs`)
6. **Internal modules** (lower priority)

---

## Common Mistakes to Avoid

❌ **Don't:**
```rust
// No documentation
pub fn do_something() -> Result<()> { }

// Vague documentation
/// Does something
pub fn do_something() -> Result<()> { }

// Missing error cases
/// Returns the thing
pub fn get_thing() -> Result<Thing> { }
```

✅ **Do:**
```rust
/// Retrieves a thing by its ID from the database.
///
/// # Arguments
///
/// * `id` - The unique identifier
///
/// # Returns
///
/// * `Ok(Thing)` - Thing found and returned
/// * `Err(NotFound)` - No thing with this ID
/// * `Err(DatabaseError)` - Query failed
///
/// # Examples
///
/// ```rust
/// let thing = get_thing(123).await?;
/// ```
pub async fn get_thing(id: i64) -> Result<Thing> { }
```

---

## Enforcement

Add to CI pipeline:

```yaml
# .github/workflows/backend-ci.yml
- name: Check documentation
  run: |
    cd backend
    cargo rustdoc -- -D missing-docs
```

Add to `Cargo.toml`:

```toml
[lints.rust]
missing_docs = "warn"

[lints.clippy]
missing_docs_in_private_items = "allow"  # Only warn on public items
```

---

## Resources

- [Rust Documentation Guidelines](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
- [RFC 1574 - API Documentation Conventions](https://rust-lang.github.io/rfcs/1574-more-api-documentation-conventions.html)
- [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
