//! Regression: Cursor Pagination Issue
//!
//! # Background
//! When fetching multi-page result sets from the Horizon API the `cursor`
//! query parameter must be set to the `paging_token` of the **last record**
//! of the previous page.  An earlier version of `fetch_all_payments` failed
//! to propagate this value correctly, causing duplicate records or infinite
//! loops on live mainnet data.
//!
//! # Reproduction
//! Call `fetch_all_payments` with a page-size smaller than the total number
//! of available payments.  Without the fix the second request was sent
//! without a cursor, restarting from the beginning.
//!
//! # Fix
//! `StellarRpcClient::last_payment_cursor` now extracts the `paging_token`
//! from the last element of each page and passes it as `cursor` to the next
//! `fetch_payments_page` call inside `fetch_all_payments`.
//!
//! # References
//! - GitHub Issue: stellar-insights#cursor-pagination
//! - Stellar Horizon pagination docs: https://developers.stellar.org/api/introduction/pagination/

use stellar_insights_backend::rpc::StellarRpcClient;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Assert that every payment in the collection has a non-empty `paging_token`.
/// Horizon guarantees that every payment record carries a unique paging token;
/// if any are empty the client is discarding or not parsing them correctly.
fn assert_paging_tokens_present(payments: &[stellar_insights_backend::rpc::Payment]) {
    for (i, p) in payments.iter().enumerate() {
        assert!(
            !p.paging_token.is_empty(),
            "payment at index {i} (id={}) is missing its paging_token",
            p.id
        );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Regression: cursor is always propagated across pages.
///
/// In mock mode `fetch_all_payments` returns a flat list built from
/// `mock_payments`.  Every mock payment carries a deterministic
/// `paging_token` in the form `"paging_<index>"`.  This test verifies:
///
/// 1. The client returns the requested number of payments without
///    duplicates (a symptom of cursor not advancing).
/// 2. Every returned record has a non-empty `paging_token`, confirming
///    the field is parsed and preserved end-to-end.
/// 3. Tokens are unique across the full result set (no page restart).
#[tokio::test]
async fn test_cursor_pagination_no_duplicates() {
    let client = StellarRpcClient::new_with_defaults(true);
    let requested = 50u32;

    let payments = client
        .fetch_all_payments(Some(requested))
        .await
        .expect("fetch_all_payments failed in mock mode");

    // Correct count – no records dropped, no duplicates from page restart.
    assert_eq!(
        payments.len(),
        requested as usize,
        "expected exactly {requested} payments, got {}",
        payments.len()
    );

    // Every record must carry a paging_token.
    assert_paging_tokens_present(&payments);

    // Uniqueness: if cursor did not advance we would receive the first page
    // repeatedly, producing duplicate ids.
    let mut seen_ids = std::collections::HashSet::new();
    for p in &payments {
        assert!(
            seen_ids.insert(p.id.clone()),
            "duplicate payment id '{}' detected – cursor pagination is broken",
            p.id
        );
    }
}

/// Regression: paging_token values are in ascending order.
///
/// Horizon returns records in ledger-close-time order.  If the cursor is
/// reset between pages the sequence restarts and later tokens will be
/// numerically smaller than earlier ones.
#[tokio::test]
async fn test_cursor_pagination_token_ordering() {
    let client = StellarRpcClient::new_with_defaults(true);

    let payments = client
        .fetch_all_payments(Some(30))
        .await
        .expect("fetch_all_payments failed in mock mode");

    assert!(!payments.is_empty(), "expected at least one payment");

    // In mock mode tokens are `paging_<N>` where N = index; verify monotonic
    // growth as a proxy for correct cursor advancement. Tokens must be
    // compared by their numeric suffix, not lexicographically — a plain
    // string sort would put "paging_10" before "paging_2".
    let indices: Vec<u64> = payments
        .iter()
        .map(|p| {
            p.paging_token
                .strip_prefix("paging_")
                .and_then(|n| n.parse::<u64>().ok())
                .expect("paging_token should be in the form paging_<N>")
        })
        .collect();
    let sorted = {
        let mut n = indices.clone();
        n.sort_unstable();
        n
    };
    assert_eq!(
        indices, sorted,
        "paging_tokens are not in ascending numeric order – indicates cursor regression"
    );
}

/// Regression: large paginated requests stay within the record cap.
///
/// Previously, a missing cursor caused the loop to never terminate until
/// the hard `ABSOLUTE_MAX_TOTAL_RECORDS` cap was hit unexpectedly.
#[tokio::test]
async fn test_cursor_pagination_respects_limit() {
    let client = StellarRpcClient::new_with_defaults(true);

    let payments = client
        .fetch_all_payments(Some(200))
        .await
        .expect("fetch_all_payments failed in mock mode");

    // Must not exceed the requested limit.
    assert!(
        payments.len() <= 200,
        "received {} payments, expected <= 200",
        payments.len()
    );
}
