//! Shared pagination types and helpers.
//!
//! All list endpoints should accept [`PaginationParams`] and return
//! [`PaginatedResponse<T>`] so clients always receive a consistent envelope
//! regardless of which resource they are querying.
//!
//! # Request format
//!
//! | Parameter | Description                                                        |
//! |-----------|--------------------------------------------------------------------|
//! | `limit`   | Page size. Clamped to `1..=max` for the endpoint (default varies). |
//! | `cursor`  | Opaque cursor copied from `pagination.next_cursor` / `prev_cursor`. |
//! | `offset`  | **Deprecated.** Accepted for backwards compatibility; `cursor` wins. |
//!
//! Cursors are opaque: clients must not parse or construct them. Today they
//! encode a position for endpoints that page over computed/in-memory result
//! sets, and upstream paging tokens (e.g. Horizon) for proxied endpoints. The
//! encoding may change without notice.
//!
//! # Wire format
//!
//! ```json
//! {
//!   "data": [ ... ],
//!   "pagination": {
//!     "limit": 50,
//!     "total": 312,
//!     "has_next": true,
//!     "has_prev": false,
//!     "next_cursor": "djE6bzo1MA",
//!     "prev_cursor": null,
//!     "offset": 0,
//!     "next_offset": 50,
//!     "prev_offset": null
//!   },
//!   "links": {
//!     "self": "/api/corridors?limit=50",
//!     "next": "/api/corridors?limit=50&cursor=djE6bzo1MA",
//!     "prev": null
//!   }
//! }
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::pagination::{PaginatedResponse, PaginationParams};
//!
//! async fn list(
//!     OriginalUri(uri): OriginalUri,
//!     Query(page): Query<PaginationParams>,
//! ) -> ApiResult<Json<PaginatedResponse<MyItem>>> {
//!     let page = page.resolve(50, 200)?;
//!     let items = db.list_items(page.limit, page.offset).await?;
//!     let total = db.count_items().await?;
//!     Ok(Json(PaginatedResponse::from_page(items, total, page).with_links(&uri)))
//! }
//! ```

use axum::http::Uri;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::error::ApiError;

/// Version prefix embedded in position cursors so the format can evolve.
const CURSOR_PREFIX: &str = "v1:o:";

/// Query parameters accepted by every paginated list endpoint.
///
/// Endpoints with extra filters should declare `limit`/`cursor`/`offset` on
/// their own query struct and build this from them — `#[serde(flatten)]` does
/// not work for numeric fields with `serde_urlencoded`.
#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PaginationParams {
    /// Maximum number of items to return.
    #[param(example = 50)]
    pub limit: Option<i64>,
    /// Opaque cursor from a previous response's `next_cursor` / `prev_cursor`.
    #[param(example = "djE6bzo1MA")]
    pub cursor: Option<String>,
    /// Deprecated: number of items to skip. Prefer `cursor`.
    #[param(example = 0)]
    pub offset: Option<i64>,
}

/// A validated, clamped page request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub limit: i64,
    pub offset: i64,
}

impl PaginationParams {
    /// Validate and clamp the request. `cursor` takes precedence over `offset`.
    pub fn resolve(&self, default_limit: i64, max_limit: i64) -> Result<Page, ApiError> {
        let limit = self.limit.unwrap_or(default_limit).clamp(1, max_limit);
        let offset = match self.cursor.as_deref().filter(|c| !c.is_empty()) {
            Some(cursor) => decode_position_cursor(cursor)?,
            None => self.offset.unwrap_or(0).max(0),
        };
        Ok(Page { limit, offset })
    }
}

/// Encode an offset as an opaque cursor.
#[must_use]
pub fn encode_position_cursor(offset: i64) -> String {
    URL_SAFE_NO_PAD.encode(format!("{CURSOR_PREFIX}{offset}"))
}

/// Decode an opaque position cursor back into an offset.
pub fn decode_position_cursor(cursor: &str) -> Result<i64, ApiError> {
    let invalid = || ApiError::bad_request("INVALID_CURSOR", "The pagination cursor is invalid");
    let bytes = URL_SAFE_NO_PAD.decode(cursor).map_err(|_| invalid())?;
    let raw = String::from_utf8(bytes).map_err(|_| invalid())?;
    raw.strip_prefix(CURSOR_PREFIX)
        .and_then(|n| n.parse::<i64>().ok())
        .filter(|n| *n >= 0)
        .ok_or_else(invalid)
}

/// Pagination metadata included in every list response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PageMeta {
    /// Maximum number of items requested.
    #[schema(example = 50)]
    pub limit: i64,
    /// Total number of items available across all pages, when cheaply known.
    #[schema(example = 312)]
    pub total: Option<i64>,
    /// Whether there is a next page.
    #[schema(example = true)]
    pub has_next: bool,
    /// Whether there is a previous page.
    #[schema(example = false)]
    pub has_prev: bool,
    /// Cursor for the next page, or `null` on the last page.
    #[schema(example = "djE6bzo1MA")]
    pub next_cursor: Option<String>,
    /// Cursor for the previous page, or `null` on the first page.
    #[schema(example = json!(null))]
    pub prev_cursor: Option<String>,
    /// Deprecated: number of items skipped. `null` for token-paged endpoints.
    #[schema(example = 0)]
    pub offset: Option<i64>,
    /// Deprecated: offset for the next page. Prefer `next_cursor`.
    #[schema(example = 50)]
    pub next_offset: Option<i64>,
    /// Deprecated: offset for the previous page. Prefer `prev_cursor`.
    #[schema(example = json!(null))]
    pub prev_offset: Option<i64>,
}

impl PageMeta {
    /// Compute pagination metadata for a position-paged result with a known total.
    #[must_use]
    pub fn new(total: i64, limit: i64, offset: i64) -> Self {
        let has_next = offset.saturating_add(limit) < total;
        let has_prev = offset > 0;
        let next_offset = has_next.then(|| offset.saturating_add(limit));
        let prev_offset = has_prev.then(|| (offset - limit).max(0));

        Self {
            limit,
            total: Some(total),
            has_next,
            has_prev,
            next_cursor: next_offset.map(encode_position_cursor),
            prev_cursor: prev_offset.map(encode_position_cursor),
            offset: Some(offset),
            next_offset,
            prev_offset,
        }
    }

    /// Metadata for a position-paged result where counting is too expensive.
    /// `has_next` is typically derived by fetching `limit + 1` rows.
    #[must_use]
    pub fn without_total(limit: i64, offset: i64, has_next: bool) -> Self {
        let has_prev = offset > 0;
        let next_offset = has_next.then(|| offset.saturating_add(limit));
        let prev_offset = has_prev.then(|| (offset - limit).max(0));

        Self {
            limit,
            total: None,
            has_next,
            has_prev,
            next_cursor: next_offset.map(encode_position_cursor),
            prev_cursor: prev_offset.map(encode_position_cursor),
            offset: Some(offset),
            next_offset,
            prev_offset,
        }
    }

    /// Metadata for endpoints paged by upstream tokens where the total is unknown.
    #[must_use]
    pub fn from_tokens(
        limit: i64,
        next_cursor: Option<String>,
        prev_cursor: Option<String>,
    ) -> Self {
        Self {
            limit,
            total: None,
            has_next: next_cursor.is_some(),
            has_prev: prev_cursor.is_some(),
            next_cursor,
            prev_cursor,
            offset: None,
            next_offset: None,
            prev_offset: None,
        }
    }
}

/// Ready-to-follow links to adjacent pages.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct PageLinks {
    #[serde(rename = "self")]
    #[schema(example = "/api/corridors?limit=50")]
    pub self_: String,
    #[schema(example = "/api/corridors?limit=50&cursor=djE6bzo1MA")]
    pub next: Option<String>,
    #[schema(example = json!(null))]
    pub prev: Option<String>,
}

/// Standard paginated response envelope used by all list endpoints.
///
/// Reference concrete instantiations directly in `#[utoipa::path]` bodies
/// (e.g. `body = PaginatedResponse<CorridorResponse>`); utoipa 5 resolves the
/// generic so the spec documents the real item type.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T: Serialize> {
    /// The page of items.
    pub data: Vec<T>,
    /// Pagination metadata.
    pub pagination: PageMeta,
    /// Links to the current, next and previous pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<PageLinks>,
}

impl<T: Serialize> PaginatedResponse<T> {
    /// Wrap a page of items with computed pagination metadata.
    #[must_use]
    pub fn new(data: Vec<T>, total: i64, limit: i64, offset: i64) -> Self {
        Self {
            pagination: PageMeta::new(total, limit, offset),
            data,
            links: None,
        }
    }

    /// Wrap a page of items for a resolved [`Page`].
    #[must_use]
    pub fn from_page(data: Vec<T>, total: i64, page: Page) -> Self {
        Self::new(data, total, page.limit, page.offset)
    }

    /// Wrap rows fetched with `LIMIT page.limit + 1`: the extra row (if any)
    /// is dropped and only used to signal that another page exists.
    #[must_use]
    pub fn from_probe(mut data: Vec<T>, page: Page) -> Self {
        let has_next = data.len() as i64 > page.limit;
        data.truncate(page.limit as usize);
        Self {
            pagination: PageMeta::without_total(page.limit, page.offset, has_next),
            data,
            links: None,
        }
    }

    /// Wrap a page of items for token-paged endpoints (total unknown).
    #[must_use]
    pub fn from_tokens(
        data: Vec<T>,
        limit: i64,
        next_cursor: Option<String>,
        prev_cursor: Option<String>,
    ) -> Self {
        Self {
            pagination: PageMeta::from_tokens(limit, next_cursor, prev_cursor),
            data,
            links: None,
        }
    }

    /// Attach `self`/`next`/`prev` links built from the request URI.
    ///
    /// Pass the full request URI (`axum::extract::OriginalUri`) so nested
    /// routers still produce absolute paths.
    #[must_use]
    pub fn with_links(mut self, uri: &Uri) -> Self {
        let link = |cursor: &Option<String>| cursor.as_deref().map(|c| page_link(uri, Some(c)));
        self.links = Some(PageLinks {
            self_: uri
                .path_and_query()
                .map_or_else(|| uri.path().to_string(), ToString::to_string),
            next: link(&self.pagination.next_cursor),
            prev: link(&self.pagination.prev_cursor),
        });
        self
    }
}

/// Rebuild `uri` with `cursor` replaced and any legacy `offset` removed.
fn page_link(uri: &Uri, cursor: Option<&str>) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    if let Some(query) = uri.query() {
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            if key != "cursor" && key != "offset" {
                serializer.append_pair(&key, &value);
            }
        }
    }
    if let Some(cursor) = cursor {
        serializer.append_pair("cursor", cursor);
    }
    let query = serializer.finish();
    if query.is_empty() {
        uri.path().to_string()
    } else {
        format!("{}?{}", uri.path(), query)
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
        assert_eq!(meta.total, Some(0));
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
        assert_eq!(resp.pagination.total, Some(100));
        assert!(resp.pagination.has_next);
    }
}
