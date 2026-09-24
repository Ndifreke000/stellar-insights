//! Shared pagination types and helpers.
//!
//! All list endpoints should return [`PaginatedResponse<T>`] so clients always
//! receive a consistent envelope regardless of which resource they are querying.
//!
//! # Wire format
//!
//! ```json
//! {
//!   "data": [ ... ],
//!   "pagination": {
//!     "limit": 50,
//!     "offset": 0,
//!     "total": 312,
//!     "has_next": true,
//!     "has_prev": false,
//!     "next_offset": 50,
//!     "prev_offset": null
//!   }
//! }
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::pagination::PaginatedResponse;
//!
//! let items: Vec<MyItem> = db.list_items(limit, offset).await?;
//! let total: i64 = db.count_items().await?;
//! let response = PaginatedResponse::new(items, total, limit, offset);
//! Ok(Json(response))
//! ```

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Pagination metadata included in every list response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PageMeta {
    /// Maximum number of items requested.
    #[schema(example = 50)]
    pub limit: i64,
    /// Number of items skipped.
    #[schema(example = 0)]
    pub offset: i64,
    /// Total number of items available (across all pages).
    #[schema(example = 312)]
    pub total: i64,
    /// Whether there is a next page.
    #[schema(example = true)]
    pub has_next: bool,
    /// Whether there is a previous page.
    #[schema(example = false)]
    pub has_prev: bool,
    /// Offset to use for the next page, or `null` if on the last page.
    #[schema(example = 50)]
    pub next_offset: Option<i64>,
    /// Offset to use for the previous page, or `null` if on the first page.
    #[schema(example = json!(null))]
    pub prev_offset: Option<i64>,
}

impl PageMeta {
    /// Compute pagination metadata from the query parameters and total count.
    #[must_use]
    pub fn new(total: i64, limit: i64, offset: i64) -> Self {
        let has_next = offset + limit < total;
        let has_prev = offset > 0;

        Self {
            limit,
            offset,
            total,
            has_next,
            has_prev,
            next_offset: has_next.then(|| offset + limit),
            prev_offset: has_prev.then(|| (offset - limit).max(0)),
        }
    }
}

/// Standard paginated response envelope used by all list endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T: Serialize> {
    /// The page of items.
    pub data: Vec<T>,
    /// Pagination metadata.
    pub pagination: PageMeta,
}

impl<T: Serialize> PaginatedResponse<T> {
    /// Wrap a page of items with computed pagination metadata.
    #[must_use]
    pub fn new(data: Vec<T>, total: i64, limit: i64, offset: i64) -> Self {
        Self {
            pagination: PageMeta::new(total, limit, offset),
            data,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_page_has_next_no_prev() {
        let meta = PageMeta::new(100, 10, 0);
        assert!(meta.has_next);
        assert!(!meta.has_prev);
        assert_eq!(meta.next_offset, Some(10));
        assert_eq!(meta.prev_offset, None);
    }

    #[test]
    fn last_page_has_prev_no_next() {
        let meta = PageMeta::new(100, 10, 90);
        assert!(!meta.has_next);
        assert!(meta.has_prev);
        assert_eq!(meta.next_offset, None);
        assert_eq!(meta.prev_offset, Some(80));
    }

    #[test]
    fn middle_page_has_both() {
        let meta = PageMeta::new(100, 10, 50);
        assert!(meta.has_next);
        assert!(meta.has_prev);
        assert_eq!(meta.next_offset, Some(60));
        assert_eq!(meta.prev_offset, Some(40));
    }

    #[test]
    fn single_page_no_next_no_prev() {
        let meta = PageMeta::new(5, 50, 0);
        assert!(!meta.has_next);
        assert!(!meta.has_prev);
        assert_eq!(meta.next_offset, None);
        assert_eq!(meta.prev_offset, None);
    }

    #[test]
    fn empty_result_set() {
        let meta = PageMeta::new(0, 50, 0);
        assert!(!meta.has_next);
        assert!(!meta.has_prev);
        assert_eq!(meta.total, 0);
    }

    #[test]
    fn prev_offset_clamps_to_zero() {
        // offset=5, limit=10 → prev would be -5, should clamp to 0
        let meta = PageMeta::new(100, 10, 5);
        assert_eq!(meta.prev_offset, Some(0));
    }

    #[test]
    fn paginated_response_wraps_data() {
        let items = vec![1u32, 2, 3];
        let resp = PaginatedResponse::new(items.clone(), 100, 10, 0);
        assert_eq!(resp.data, items);
        assert_eq!(resp.pagination.total, 100);
        assert!(resp.pagination.has_next);
    }
}
