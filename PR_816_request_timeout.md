# Pull Request: Implement Request Timeout Configuration

## Description
This pull request implements a request timeout layer in the backend to prevent requests from hanging indefinitely, improving performance and reliability.

### Changes:
- **`backend/Cargo.toml`**: Added the `timeout` feature to the `tower-http` dependency.
- **`backend/src/main.rs`**: 
    - Configured a new `TimeoutLayer` in the Axum application.
    - Added support for a `REQUEST_TIMEOUT_SECONDS` environment variable (defaults to 30 seconds).
    - Added logging to confirm the timeout configuration on startup.

## Testing
- Verified implementation against Axum/Tower best practices.
- Confirmed that the default and environment-based timeouts are correctly configured in the router logic.

## Related Issues
Closes #816
