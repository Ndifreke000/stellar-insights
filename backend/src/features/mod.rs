//! Feature modules.
//!
//! Each submodule is a self-contained capability that can be wired into the
//! API layer. Modules are expected to keep their logic testable without HTTP
//! (see `search` for the FTS5-backed corridor search).

pub mod search;
