//! Regression module: declares one submodule per tracked mainnet issue.
//!
//! # Naming Convention
//! Each file is named after the issue it covers:
//!   `issue_<short_slug>.rs`
//!
//! # Adding New Regressions
//! 1. Copy `issue_template.rs` → `issue_<your_slug>.rs`
//! 2. Add `pub mod issue_<your_slug>;` here.
//! 3. Implement your test(s) following the template.
//! 4. Run `cargo test --test regression` to verify.

pub mod issue_analytics_nplusone;
pub mod issue_cursor_pagination;
pub mod issue_reserve_offbyone;
pub mod issue_template;
