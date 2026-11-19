//! Size-bounded reading and decoding of RPC / Horizon responses.
//!
//! Providers are configurable third parties, so their response sizes are not
//! trusted:
//!
//! * [`read_bounded`] rejects a response whose `Content-Length` exceeds the
//!   limit for its call type, and otherwise reads the body chunk by chunk,
//!   aborting as soon as the running total passes the limit. Memory use is
//!   bounded by the limit plus one chunk, whatever the provider sends.
//! * [`decode_horizon_page`] decodes `_embedded.records` element by element
//!   with a visitor (no intermediate `serde_json::Value` tree and no
//!   `#[serde(flatten)]` buffering) and caps the record count.
//! * Oversize responses surface as [`RpcError::ResponseTooLarge`]; paginated
//!   callers use [`next_page_size`] to retry with a smaller page.

use std::fmt;
use std::marker::PhantomData;

use serde::de::{
    self, DeserializeOwned, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor,
};

use crate::rpc::error::RpcError;

/// Per-call-type body limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyLimit {
    /// A single Horizon resource or small JSON-RPC result (1 MiB).
    Single,
    /// A Horizon collection page (`_embedded.records`) (8 MiB).
    Page,
    /// A JSON-RPC result that can be large, e.g. `getLedgers` / `getEvents` (16 MiB).
    RpcBatch,
    /// An error body that is only logged (64 KiB).
    ErrorBody,
}

impl BodyLimit {
    #[must_use]
    pub const fn bytes(self) -> usize {
        match self {
            Self::Single => 1024 * 1024,
            Self::Page => 8 * 1024 * 1024,
            Self::RpcBatch => 16 * 1024 * 1024,
            Self::ErrorBody => 64 * 1024,
        }
    }
}

/// Hard cap on records decoded from one page, independent of byte size.
pub const MAX_RECORDS_PER_PAGE: usize = 1_000;

/// Smallest page size adaptive reduction will go down to.
pub const MIN_ADAPTIVE_PAGE_SIZE: u32 = 10;

/// Page size to retry with after a [`RpcError::ResponseTooLarge`]: half the
/// current size, or `None` once it would drop below [`MIN_ADAPTIVE_PAGE_SIZE`].
#[must_use]
pub const fn next_page_size(current: u32) -> Option<u32> {
    let next = current / 2;
    if next < MIN_ADAPTIVE_PAGE_SIZE {
        None
    } else {
        Some(next)
    }
}

/// Run a paginated fetch, halving the page size each time the provider's
/// response is too large, until it fits or [`MIN_ADAPTIVE_PAGE_SIZE`] is hit.
pub async fn with_adaptive_page_size<T, F, Fut>(limit: u32, mut fetch: F) -> Result<T, RpcError>
where
    F: FnMut(u32) -> Fut,
    Fut: std::future::Future<Output = Result<T, RpcError>>,
{
    let mut page = limit;
    loop {
        match fetch(page).await {
            Err(err @ RpcError::ResponseTooLarge { .. }) => match next_page_size(page) {
                Some(smaller) => {
                    tracing::warn!(
                        page_size = page,
                        next_page_size = smaller,
                        "provider response too large; retrying with a smaller page"
                    );
                    page = smaller;
                }
                None => return Err(err),
            },
            other => return other,
        }
    }
}

/// Accumulates body chunks, failing once the total passes `limit`.
#[derive(Debug)]
pub struct BoundedBuffer {
    buf: Vec<u8>,
    limit: usize,
}

impl BoundedBuffer {
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            buf: Vec::new(),
            limit,
        }
    }

    /// Append a chunk. Errors (without growing the buffer) if it would exceed the limit.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), RpcError> {
        let total = self.buf.len().saturating_add(chunk.len());
        if total > self.limit {
            return Err(RpcError::ResponseTooLarge {
                limit: self.limit,
                observed: Some(total as u64),
            });
        }
        self.buf.extend_from_slice(chunk);
        Ok(())
    }

    #[must_use]
    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }
}

/// Read a response body, aborting as soon as it exceeds `limit`.
pub async fn read_bounded(
    mut response: reqwest::Response,
    limit: BodyLimit,
) -> Result<Vec<u8>, RpcError> {
    let max = limit.bytes();
    if let Some(len) = response.content_length() {
        if len > max as u64 {
            return Err(RpcError::ResponseTooLarge {
                limit: max,
                observed: Some(len),
            });
        }
    }
    let mut buffer = BoundedBuffer::new(max);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| RpcError::NetworkError(e.to_string()))?
    {
        buffer.push(&chunk)?;
    }
    Ok(buffer.into_inner())
}

/// Read an error body for logging, truncated at [`BodyLimit::ErrorBody`]
/// instead of failing.
pub async fn read_error_text(mut response: reqwest::Response) -> String {
    let max = BodyLimit::ErrorBody.bytes();
    let mut buf: Vec<u8> = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        let room = max.saturating_sub(buf.len());
        if chunk.len() >= room {
            buf.extend_from_slice(&chunk[..room]);
            break;
        }
        buf.extend_from_slice(&chunk);
    }
    if buf.is_empty() {
        return "Unknown error".to_string();
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Decode a bounded body as JSON.
pub fn decode_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, RpcError> {
    serde_json::from_slice(bytes).map_err(|e| RpcError::ParseError(e.to_string()))
}

/// Read a response with a limit and decode it as JSON.
pub async fn read_json<T: DeserializeOwned>(
    response: reqwest::Response,
    limit: BodyLimit,
) -> Result<T, RpcError> {
    let bytes = read_bounded(response, limit).await?;
    decode_json(&bytes)
}

/// Read a Horizon collection page and return its `_embedded.records`.
pub async fn read_horizon_records<T: DeserializeOwned>(
    response: reqwest::Response,
) -> Result<Vec<T>, RpcError> {
    let bytes = read_bounded(response, BodyLimit::Page).await?;
    decode_horizon_page(&bytes, MAX_RECORDS_PER_PAGE)
}

/// Decode `_embedded.records` from a Horizon page without materialising the
/// rest of the document. Missing `_embedded` gives an empty list. More than
/// `max_records` records is a [`RpcError::ResponseTooLarge`].
pub fn decode_horizon_page<T: DeserializeOwned>(
    bytes: &[u8],
    max_records: usize,
) -> Result<Vec<T>, RpcError> {
    let mut de = serde_json::Deserializer::from_slice(bytes);
    let outcome = PageSeed::<T> {
        max_records,
        _t: PhantomData,
    }
    .deserialize(&mut de);
    match outcome {
        Ok(records) => {
            de.end().map_err(|e| RpcError::ParseError(e.to_string()))?;
            Ok(records)
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.starts_with(TOO_MANY_RECORDS) {
                Err(RpcError::ResponseTooLarge {
                    limit: max_records,
                    observed: None,
                })
            } else {
                Err(RpcError::ParseError(msg))
            }
        }
    }
}

const TOO_MANY_RECORDS: &str = "too many records";

struct PageSeed<T> {
    max_records: usize,
    _t: PhantomData<T>,
}

impl<'de, T: DeserializeOwned> DeserializeSeed<'de> for PageSeed<T> {
    type Value = Vec<T>;

    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(PageVisitor::<T> {
            max_records: self.max_records,
            _t: PhantomData,
        })
    }
}

struct PageVisitor<T> {
    max_records: usize,
    _t: PhantomData<T>,
}

impl<'de, T: DeserializeOwned> Visitor<'de> for PageVisitor<T> {
    type Value = Vec<T>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a Horizon page object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut records: Option<Vec<T>> = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == "_embedded" && records.is_none() {
                records = Some(map.next_value_seed(EmbeddedSeed::<T> {
                    max_records: self.max_records,
                    _t: PhantomData,
                })?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(records.unwrap_or_default())
    }
}

struct EmbeddedSeed<T> {
    max_records: usize,
    _t: PhantomData<T>,
}

impl<'de, T: DeserializeOwned> DeserializeSeed<'de> for EmbeddedSeed<T> {
    type Value = Vec<T>;

    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(EmbeddedVisitor::<T> {
            max_records: self.max_records,
            _t: PhantomData,
        })
    }
}

struct EmbeddedVisitor<T> {
    max_records: usize,
    _t: PhantomData<T>,
}

impl<'de, T: DeserializeOwned> Visitor<'de> for EmbeddedVisitor<T> {
    type Value = Vec<T>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an _embedded object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut records: Option<Vec<T>> = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == "records" && records.is_none() {
                records = Some(map.next_value_seed(RecordsSeed::<T> {
                    max_records: self.max_records,
                    _t: PhantomData,
                })?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(records.unwrap_or_default())
    }
}

struct RecordsSeed<T> {
    max_records: usize,
    _t: PhantomData<T>,
}

impl<'de, T: DeserializeOwned> DeserializeSeed<'de> for RecordsSeed<T> {
    type Value = Vec<T>;

    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}

impl<'de, T: DeserializeOwned> Visitor<'de> for RecordsSeed<T> {
    type Value = Vec<T>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an array of records")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        // Never trust the size hint for preallocation beyond the cap.
        let hint = seq.size_hint().unwrap_or(0).min(self.max_records);
        let mut out = Vec::with_capacity(hint);
        while let Some(item) = seq.next_element::<T>()? {
            if out.len() == self.max_records {
                return Err(de::Error::custom(format!(
                    "{TOO_MANY_RECORDS} (max {})",
                    self.max_records
                )));
            }
            out.push(item);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Rec {
        id: String,
    }

    #[test]
    fn decodes_embedded_records_and_ignores_other_fields() {
        let body = br#"{"_links":{"self":{"href":"x"}},"_embedded":{"records":[{"id":"a","extra":1},{"id":"b"}]},"tail":[1,2,3]}"#;
        let recs: Vec<Rec> = decode_horizon_page(body, 10).unwrap();
        assert_eq!(recs, vec![Rec { id: "a".into() }, Rec { id: "b".into() }]);
    }

    #[test]
    fn missing_embedded_is_empty() {
        let recs: Vec<Rec> = decode_horizon_page(br#"{"_links":{}}"#, 10).unwrap();
        assert!(recs.is_empty());
    }

    #[test]
    fn too_many_records_is_response_too_large() {
        let body = br#"{"_embedded":{"records":[{"id":"a"},{"id":"b"},{"id":"c"}]}}"#;
        let err = decode_horizon_page::<Rec>(body, 2).unwrap_err();
        assert!(matches!(err, RpcError::ResponseTooLarge { limit: 2, .. }));
    }

    #[test]
    fn malformed_page_is_parse_error() {
        let bodies: [&[u8]; 6] = [
            b"",
            b"[]",
            b"{\"_embedded\":{\"records\":{}}}",
            b"{\"_embedded\":{\"records\":[{\"id\":1}]}}",
            b"{\"_embedded\":{\"records\":[{\"id\":\"a\"}]}} trailing",
            b"{\"_embedded\":",
        ];
        for body in bodies {
            let err = decode_horizon_page::<Rec>(body, 10).unwrap_err();
            assert!(
                matches!(err, RpcError::ParseError(_)),
                "{body:?} -> {err:?}"
            );
        }
    }

    #[test]
    fn bounded_buffer_rejects_overflow_without_growing() {
        let mut b = BoundedBuffer::new(8);
        b.push(b"12345").unwrap();
        let err = b.push(b"6789").unwrap_err();
        assert!(matches!(
            err,
            RpcError::ResponseTooLarge {
                limit: 8,
                observed: Some(9)
            }
        ));
        assert_eq!(b.into_inner(), b"12345");
    }

    #[test]
    fn next_page_size_halves_until_floor() {
        assert_eq!(next_page_size(200), Some(100));
        assert_eq!(next_page_size(25), Some(12));
        assert_eq!(next_page_size(19), None);
        assert_eq!(next_page_size(0), None);
    }

    #[tokio::test]
    async fn adaptive_page_size_shrinks_until_response_fits() {
        let mut seen = Vec::new();
        let out = with_adaptive_page_size(200, |page| {
            seen.push(page);
            async move {
                if page > 50 {
                    Err(RpcError::ResponseTooLarge {
                        limit: 1,
                        observed: None,
                    })
                } else {
                    Ok(page)
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(out, 50);
        assert_eq!(seen, vec![200, 100, 50]);
    }

    #[tokio::test]
    async fn adaptive_page_size_gives_up_at_floor() {
        let err = with_adaptive_page_size(20, |_| async {
            Err::<(), _>(RpcError::ResponseTooLarge {
                limit: 1,
                observed: None,
            })
        })
        .await
        .unwrap_err();
        assert!(matches!(err, RpcError::ResponseTooLarge { .. }));
    }

    #[tokio::test]
    async fn adaptive_page_size_does_not_retry_other_errors() {
        let mut calls = 0;
        let err = with_adaptive_page_size(200, |_| {
            calls += 1;
            async { Err::<(), _>(RpcError::ParseError("bad".into())) }
        })
        .await
        .unwrap_err();
        assert!(matches!(err, RpcError::ParseError(_)));
        assert_eq!(calls, 1);
    }

    proptest! {
        // Fuzz-style: arbitrary bytes never panic, and only produce the two
        // expected error kinds.
        #[test]
        fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            match decode_horizon_page::<Rec>(&bytes, 4) {
                Ok(v) => prop_assert!(v.len() <= 4),
                Err(RpcError::ParseError(_) | RpcError::ResponseTooLarge { .. }) => {}
                Err(other) => prop_assert!(false, "unexpected error {other:?}"),
            }
        }
    }
}
