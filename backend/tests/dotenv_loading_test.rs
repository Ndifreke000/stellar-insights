//! Startup `.env` loading must distinguish "no file" (fine) from "broken file" (fatal).
//!
//! Regression tests for issue #2323, where load failures were silently ignored.

use payraider_backend::env_config::{load_dotenv_from, DotenvStatus};

#[test]
fn missing_env_file_is_reported_as_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let status = load_dotenv_from(&dir.path().join("does-not-exist.env")).unwrap();
    assert_eq!(status, DotenvStatus::NotFound);
    assert!(status.describe().contains("not found"));
}

#[test]
fn valid_env_file_is_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("valid.env");
    std::fs::write(&path, "DOTENV_LOADING_TEST_VALID=from_file\n").unwrap();

    let status = load_dotenv_from(&path).unwrap();
    assert_eq!(status, DotenvStatus::Loaded(path));
    assert_eq!(
        std::env::var("DOTENV_LOADING_TEST_VALID").as_deref(),
        Ok("from_file")
    );
}

#[test]
fn malformed_env_file_fails_instead_of_being_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("malformed.env");
    std::fs::write(&path, "GOOD_KEY=1\nthis line is not a valid assignment\n").unwrap();

    let err = load_dotenv_from(&path).unwrap_err().to_string();
    assert!(
        err.contains("could not be loaded"),
        "a malformed .env must not be treated as a missing one, got: {err}"
    );
}
