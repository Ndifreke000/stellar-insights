//! Regression test template.
//!
//! Copy this file to `issue_<slug>.rs` and fill in the details below.
//!
//! # Template Instructions
//! - Replace `ISSUE_NUMBER` with the GitHub issue number (e.g. `1697`).
//! - Replace `ISSUE_TITLE` with a brief human-readable description.
//! - Replace the `REPRODUCTION` block with the minimal code that used to trigger the bug.
//! - Replace the `EXPECTED` block with the correct behaviour post-fix.
//! - Add `pub mod issue_<slug>;` to `regression/mod.rs`.
//!
//! # Example
//! ```ignore
//! // Issue #1234 – Off-by-one in fee calculation
//! // Reproduction: call `calculate_fee(0)` – previously panicked.
//! // Expected:     returns `Ok(0)` without panic.
//! #[test]
//! fn test_fee_calculation_zero_amount() {
//!     let result = calculate_fee(0);
//!     assert_eq!(result.unwrap(), 0);
//! }
//! ```

// ---------------------------------------------------------------------------
// Issue metadata (fill in when copying)
// ---------------------------------------------------------------------------
// GitHub Issue : #ISSUE_NUMBER
// Title        : ISSUE_TITLE
// Reported     : YYYY-MM-DD
// Fixed in     : <commit sha or PR number>
// Relates to   : <links to related issues / PRs>
// ---------------------------------------------------------------------------

/// Placeholder smoke-test so the template compiles and shows up in test output.
/// Delete this and replace with your actual regression test(s).
#[test]
fn test_template_placeholder() {
    // This test intentionally does nothing; it is a compile-time proof that
    // the regression module is wired up correctly.
    assert!(true, "template placeholder – replace with a real regression test");
}
