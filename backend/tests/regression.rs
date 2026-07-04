// Regression test suite entry point for `cargo test --test regression`.
// Each submodule corresponds to a named mainnet issue that has been
// diagnosed, fixed, and permanently covered here to prevent re-introduction.
//
// # Naming Convention
// Each file is named after the issue it covers: `issue_<short_slug>.rs`
//
// # Adding New Regressions
// 1. Copy `issue_template.rs` -> `issue_<your_slug>.rs`
// 2. Add `#[path = "regression/issue_<your_slug>.rs"] mod issue_<your_slug>;` below.
// 3. Implement your test(s) following the template.
// 4. Run `cargo test --test regression` to verify.

#[path = "regression/issue_analytics_nplusone.rs"]
mod issue_analytics_nplusone;
#[path = "regression/issue_cursor_pagination.rs"]
mod issue_cursor_pagination;
#[path = "regression/issue_reserve_offbyone.rs"]
mod issue_reserve_offbyone;
#[path = "regression/issue_template.rs"]
mod issue_template;
