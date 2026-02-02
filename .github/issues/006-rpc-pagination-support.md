---
title: "[ENHANCEMENT] No Pagination Support for RPC Data Fetching"
labels: enhancement, high, rpc, performance, backend
assignees: ''
---

## 💡 Description

The RPC client currently fetches fixed limits (200 payments, 200 trades) without proper pagination support. This limits data completeness and prevents fetching historical data beyond the initial batch.

## 📊 Current Behavior

```rust
let payments = rpc_client.fetch_payments(200, None).await?;
let trades = rpc_client.fetch_trades(200, None).await?;
```

**Problems:**
- Only fetches first 200 records
- No cursor-based pagination
- Cannot retrieve complete historical data
- Misses older transactions
- Limited data for analysis

## ✅ Expected Behavior

- Implement cursor-based pagination
- Fetch all available data or up to configurable limit
- Support time-range queries
- Handle pagination automatically in background
- Respect rate limits during pagination

## 📁 Affected Files

- `backend/src/rpc/stellar.rs` (all fetch methods)
- `backend/src/api/corridors_cached.rs` (uses fetch_payments)
- `backend/src/api/anchors_cached.rs` (uses fetch_account_payments)

## 📋 Acceptance Criteria

- [ ] Add pagination support to all fetch methods
- [ ] Implement cursor tracking and extraction
- [ ] Add configurable max records limit
- [ ] Support time-range filtering
- [ ] Add background pagination for large datasets
- [ ] Handle rate limiting during pagination
- [ ] Add progress logging for large fetches
- [ ] Add tests for pagination logic
- [ ] Document pagination behavior

## 🚀 Implementation Steps

1. **Update fetch method signatures**
   ```rust
   pub async fn fetch_payments_paginated(
       &self,
       max_records: Option<usize>,
       time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
   ) -> Result<Vec<Payment>>
   ```

2. **Implement cursor extraction from responses**
   ```rust
   fn extract_cursor(response: &HorizonResponse<Payment>) -> Option<String> {
       response.embedded
           .as_ref()?
           .records
           .last()?
           .paging_token
           .clone()
   }
   ```

3. **Create pagination loop with rate limiting**
   ```rust
   let mut all_payments = Vec::new();
   let mut cursor = None;
   
   loop {
       self.rate_limiter.acquire().await?;
       
       let batch = self.fetch_payments(200, cursor.as_deref()).await?;
       if batch.is_empty() {
           break;
       }
       
       cursor = batch.last().map(|p| p.paging_token.clone());
       all_payments.extend(batch);
       
       if let Some(max) = max_records {
           if all_payments.len() >= max {
               all_payments.truncate(max);
               break;
           }
       }
       
       tokio::time::sleep(Duration::from_millis(100)).await;
   }
   ```

4. **Add progress logging**
5. **Implement time-range filters**
6. **Add configuration for pagination limits**
7. **Test with large datasets**

## ⚙️ Configuration

```toml
# backend/.env
RPC_MAX_PAYMENTS_PER_REQUEST=200
RPC_MAX_TOTAL_PAYMENTS=10000
RPC_PAGINATION_DELAY_MS=100
RPC_ENABLE_AUTO_PAGINATION=true
```

## 💻 Usage Example

```rust
// Fetch all payments from last 7 days
let start = Utc::now() - Duration::days(7);
let end = Utc::now();
let payments = rpc_client
    .fetch_payments_paginated(Some(5000), Some((start, end)))
    .await?;

// Fetch all available payments (up to max)
let all_payments = rpc_client
    .fetch_payments_paginated(Some(10000), None)
    .await?;
```

## 🧪 Test Cases

- [ ] Pagination with small dataset (< 200 records)
- [ ] Pagination with large dataset (> 1000 records)
- [ ] Pagination with max_records limit
- [ ] Pagination with time range filter
- [ ] Pagination with rate limiting
- [ ] Pagination error handling (network failure mid-pagination)
- [ ] Cursor extraction and usage

## 📊 Performance Considerations

- Add delay between requests to avoid rate limits
- Use streaming for very large datasets
- Consider background jobs for historical data sync
- Monitor memory usage with large result sets

## 🏷️ Priority

**High** - Limits data completeness and historical analysis

## 🔗 Related Issues

- Related to #9 (Rate Limiting)
- Improves #2 (Corridor Detail) data quality
