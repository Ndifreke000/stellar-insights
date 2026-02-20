# Code Quality Guide - Stellar Insights Backend

## Quick Reference for Developers

### 🚀 Before Committing

Run these commands to ensure your code meets quality standards:

```bash
cd backend

# 1. Format your code
cargo fmt

# 2. Check for warnings
cargo build --all-targets

# 3. Run clippy
cargo clippy --all-targets --all-features

# 4. Run tests
cargo test
```

All commands should complete without errors or warnings.

---

## 📋 Handling Unused Code

### Unused Parameters

**When to use**: Function signature requires parameter but implementation doesn't use it.

```rust
// ✅ GOOD: Prefix with underscore and document
/// Process data from the API
/// Note: `metadata` parameter reserved for future filtering logic
fn process_data(data: &Data, _metadata: &Metadata) -> Result<()> {
    // Implementation only uses data
    Ok(())
}

// ❌ BAD: No prefix, no documentation
fn process_data(data: &Data, metadata: &Metadata) -> Result<()> {
    Ok(())
}
```

### Unused Variables

**When to use**: Variable needed for future implementation or debugging.

```rust
// ✅ GOOD: Prefix with underscore and explain
fn calculate_metrics() -> Metrics {
    let _failed_count = 0; // TODO: Implement failure tracking in #123
    let success_count = 100;
    
    Metrics { success_count }
}

// ❌ BAD: No prefix, no explanation
fn calculate_metrics() -> Metrics {
    let failed_count = 0;
    let success_count = 100;
    
    Metrics { success_count }
}
```

### Unused Struct Fields

**When to use**: Field required for deserialization or future use.

```rust
// ✅ GOOD: Document why field is kept
/// API response from external service
/// Note: All fields required for proper JSON deserialization
#[derive(Deserialize)]
#[allow(dead_code)]
struct ApiResponse {
    status: String,
    #[allow(dead_code)]
    request_id: String,  // Not used but part of API spec
    data: Vec<Item>,
}

// ❌ BAD: No documentation
#[derive(Deserialize)]
#[allow(dead_code)]
struct ApiResponse {
    status: String,
    request_id: String,
    data: Vec<Item>,
}
```

### Dead Code

**When to use**: Code planned for future use or required by external interface.

```rust
// ✅ GOOD: Explain why code is kept
/// Helper function for future batch processing feature
/// TODO: Implement batch processing in #456
#[allow(dead_code)]
fn process_batch(items: &[Item]) -> Result<()> {
    // Implementation ready for when feature is enabled
    Ok(())
}

// ❌ BAD: No explanation
#[allow(dead_code)]
fn process_batch(items: &[Item]) -> Result<()> {
    Ok(())
}
```

---

## 🎯 Common Scenarios

### Scenario 1: Database Row Struct

You have a struct that deserializes from database but not all fields are used:

```rust
// ✅ SOLUTION
/// Database row for payments table
/// Note: Some fields fetched for completeness but not all are used in conversion
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct PaymentRow {
    id: String,
    #[allow(dead_code)]
    transaction_hash: String,  // Fetched but not used in conversion
    amount: f64,
    created_at: String,
}
```

### Scenario 2: API Compatibility

You need to keep a parameter for API consistency:

```rust
// ✅ SOLUTION
/// Verify submission success
/// Note: `submission` kept for API consistency but verification uses contract query
async fn verify_submission(
    &self,
    hash: &str,
    _submission: &SubmissionResult,  // Not used, verification via contract
) -> Result<bool> {
    self.contract.verify(hash).await
}
```

### Scenario 3: Future Implementation

You're preparing for a feature that's not yet implemented:

```rust
// ✅ SOLUTION
fn process_payments(&self, payments: &[Payment]) -> Result<()> {
    let mut success_count = 0;
    let _failure_count = 0;  // TODO: Track failures when error handling is added (#789)
    
    for payment in payments {
        success_count += 1;
    }
    
    Ok(())
}
```

### Scenario 4: Protocol Compliance

You need to match an external specification:

```rust
// ✅ SOLUTION
/// JSON-RPC 2.0 response structure
/// All fields required by specification even if not all are accessed
#[derive(Deserialize)]
#[allow(dead_code)]
struct JsonRpcResponse<T> {
    jsonrpc: String,  // Required by spec
    id: u64,          // Required by spec
    result: Option<T>,
    error: Option<RpcError>,
}
```

---

## ⚠️ When NOT to Use `#[allow(dead_code)]`

### ❌ Don't Use For:

1. **Truly Unused Code**: Delete it instead
2. **Commented Out Code**: Remove it (use git history)
3. **Experimental Code**: Move to separate branch
4. **Duplicate Code**: Refactor to remove duplication
5. **Temporary Debugging**: Remove before committing

### ✅ Do Use For:

1. **Protocol Compliance**: External API/spec requirements
2. **Future Features**: Documented with issue reference
3. **API Consistency**: Maintaining function signatures
4. **Deserialization**: Required for proper parsing
5. **Database Schema**: Matching table structure

---

## 🔍 Clippy Warnings

### Common Clippy Issues

#### 1. Unnecessary `mut`

```rust
// ❌ BAD
let mut count = 0;
println!("{}", count);

// ✅ GOOD
let count = 0;
println!("{}", count);
```

#### 2. Unnecessary Clone

```rust
// ❌ BAD
fn process(data: String) {
    let copy = data.clone();
    println!("{}", copy);
}

// ✅ GOOD
fn process(data: &str) {
    println!("{}", data);
}
```

#### 3. Redundant Pattern Matching

```rust
// ❌ BAD
match result {
    Ok(value) => Ok(value),
    Err(e) => Err(e),
}

// ✅ GOOD
result
```

#### 4. Unnecessary Return

```rust
// ❌ BAD
fn calculate() -> i32 {
    return 42;
}

// ✅ GOOD
fn calculate() -> i32 {
    42
}
```

---

## 📚 Documentation Standards

### Function Documentation

```rust
/// Brief one-line description
///
/// More detailed explanation if needed.
///
/// # Arguments
/// * `param1` - Description of param1
/// * `_param2` - Unused parameter, kept for API consistency
///
/// # Returns
/// Description of return value
///
/// # Errors
/// When this function returns an error
///
/// # Examples
/// ```
/// let result = function(data, metadata)?;
/// ```
fn function(param1: &Data, _param2: &Metadata) -> Result<Output> {
    // Implementation
}
```

### Struct Documentation

```rust
/// Brief description of struct purpose
///
/// More details about when and how to use this struct.
///
/// # Fields
/// * `field1` - Description
/// * `field2` - Unused but required for deserialization
#[derive(Debug)]
struct MyStruct {
    field1: String,
    #[allow(dead_code)]
    field2: i32,  // Required by external API spec
}
```

---

## 🛠️ CI/CD Integration

### What CI Checks

1. **Compiler Warnings**: Fails if any warnings present
2. **Clippy Lints**: Fails if clippy reports issues
3. **Code Formatting**: Fails if code not formatted
4. **Tests**: Fails if any tests fail

### Fixing CI Failures

```bash
# If CI reports warnings
cargo build --all-targets 2>&1 | grep "warning:"

# If CI reports clippy issues
cargo clippy --all-targets --all-features -- -D warnings

# If CI reports formatting issues
cargo fmt

# If CI reports test failures
cargo test
```

---

## 📖 Additional Resources

### Internal Documentation
- `WARNINGS_FIXED.md` - Detailed fix documentation
- `REFACTOR_CHECKLIST.md` - Implementation checklist
- `REFACTOR_SUMMARY.md` - Executive summary

### External Resources
- [Rust Compiler Error Index](https://doc.rust-lang.org/error-index.html)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)

---

## 💬 Getting Help

### Questions?

1. Check this guide first
2. Review existing code for examples
3. Check internal documentation
4. Ask in team chat
5. Create an issue for clarification

### Found an Issue?

1. Document the problem
2. Propose a solution
3. Create a PR with fix
4. Update this guide if needed

---

**Remember**: Quality code is maintainable code. Take the time to document your decisions!
