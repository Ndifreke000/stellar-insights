//! Fuzz the bounded Horizon page decoder with arbitrary bytes.
//!
//! Run with: `cd backend && cargo +nightly fuzz run horizon_page`
#![no_main]

use libfuzzer_sys::fuzz_target;
use payraider_backend::rpc::bounded_body::decode_horizon_page;
use payraider_backend::rpc::{HorizonOperation, Payment};

fuzz_target!(|data: &[u8]| {
    // Must never panic; only Ok, ParseError or ResponseTooLarge are allowed.
    if let Ok(records) = decode_horizon_page::<Payment>(data, 16) {
        assert!(records.len() <= 16);
    }
    let _ = decode_horizon_page::<HorizonOperation>(data, 16);
    let _ = decode_horizon_page::<serde_json::Value>(data, 16);
});
