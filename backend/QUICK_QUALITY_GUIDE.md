# Quick Code Quality Reference

## TL;DR - Run Before Committing

```bash
# Linux/Mac
./check_warnings.sh

# Windows PowerShell
./check_warnings.ps1

# Or manually
cargo clippy --all-targets --all-features -- -D warnings
```

## Common Patterns

### ✅ Intentionally Unused Parameter
```rust
fn handler(&self, _unused: String) -> Result<()> {
    // Comment explaining why it's unused
}
```

### ✅ Database Row with Unused Fields
```rust
#[derive(sqlx::FromRow)]
struct MyRow {
    used_field: String,
    #[allow(dead_code)] // Fetched from DB but not used in Rust
    unused_field: String,
}
```

### ✅ Deserialization Struct
```rust
#[derive(Deserialize)]
struct ApiResponse {
    data: String,
    #[allow(dead_code)] // Required for JSON deserialization
    metadata: String,
}
```

### ✅ Future-Proofing
```rust
struct Service {
    active: String,
    #[allow(dead_code)] // Reserved for feature X (issue #123)
    future: String,
}
```

## When to Use What

| Situation | Solution | Example |
|-----------|----------|---------|
| Unused function parameter | Prefix with `_` | `_param: String` |
| Unused struct field (DB) | `#[allow(dead_code)]` + comment | See above |
| Unused struct field (JSON) | `#[allow(dead_code)]` + comment | See above |
| Truly unused code | Remove it | Delete the code |
| Future feature | `#[allow(dead_code)]` + TODO | Link to issue |

## Quick Checks

```bash
# Build only
cargo build

# Check formatting
cargo fmt -- --check

# Run tests
cargo test

# Full quality check
cargo clippy --all-targets --all-features -- -D warnings
```

## CI Status

The CI will fail if:
- ❌ Any compiler warnings exist
- ❌ Any clippy warnings exist
- ❌ Code is not formatted
- ❌ Tests fail
- ❌ Documentation has warnings

## Need Help?

See `CODE_QUALITY.md` for comprehensive guidelines.
