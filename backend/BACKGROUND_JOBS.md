# Background Job System

## Overview

The background job system provides automated, periodic execution of maintenance tasks to keep data fresh and caches warm without requiring manual intervention or on-demand requests.

## Features

- ✅ Configurable job schedules via environment variables
- ✅ Individual job enable/disable controls
- ✅ Automatic error handling and logging
- ✅ Non-blocking execution
- ✅ Graceful shutdown support

## Available Jobs

### 1. Corridor Refresh Job
**Purpose:** Periodically refresh corridor metrics and invalidate stale caches

**Default Schedule:** Every 5 minutes (300 seconds)

**Configuration:**
```bash
JOB_CORRIDOR_REFRESH_ENABLED=true
JOB_CORRIDOR_REFRESH_INTERVAL_SECONDS=300
```

**What it does:**
- Syncs all corridor metrics from Stellar network
- Invalidates corridor-related caches
- Ensures fresh data for corridor endpoints

### 2. Anchor Refresh Job
**Purpose:** Invalidate anchor caches to ensure fresh data

**Default Schedule:** Every 10 minutes (600 seconds)

**Configuration:**
```bash
JOB_ANCHOR_REFRESH_ENABLED=true
JOB_ANCHOR_REFRESH_INTERVAL_SECONDS=600
```

**What it does:**
- Invalidates anchor-related caches
- Forces fresh data fetch on next request

### 3. Price Feed Update Job
**Purpose:** Warm price feed cache with latest prices

**Default Schedule:** Every 15 minutes (900 seconds)

**Configuration:**
```bash
JOB_PRICE_FEED_UPDATE_ENABLED=true
JOB_PRICE_FEED_UPDATE_INTERVAL_SECONDS=900
```

**What it does:**
- Fetches prices for all mapped assets
- Warms cache to prevent cold starts
- Reduces API calls during peak usage

### 4. Cache Cleanup Job
**Purpose:** Clean up expired cache entries

**Default Schedule:** Every 1 hour (3600 seconds)

**Configuration:**
```bash
JOB_CACHE_CLEANUP_ENABLED=true
JOB_CACHE_CLEANUP_INTERVAL_SECONDS=3600
```

**What it does:**
- Triggers cache cleanup (Redis auto-expires keys)
- Useful for monitoring and logging

## Configuration

All jobs are configured via environment variables in `.env`:

```bash
# Enable/disable a job
JOB_<NAME>_ENABLED=true|false

# Set job interval in seconds
JOB_<NAME>_INTERVAL_SECONDS=<seconds>
```

Job names (uppercase, underscores):
- `CORRIDOR_REFRESH`
- `ANCHOR_REFRESH`
- `PRICE_FEED_UPDATE`
- `CACHE_CLEANUP`

## Monitoring

Jobs log their execution status:

```
INFO Running job 'corridor-refresh'
INFO Job 'corridor-refresh' completed successfully
```

Errors are logged but don't stop the scheduler:

```
ERROR Job 'price-feed-update' failed: Network timeout
```

## Architecture

```
JobScheduler
├── Job: corridor-refresh (5min)
├── Job: anchor-refresh (10min)
├── Job: price-feed-update (15min)
└── Job: cache-cleanup (1hr)
```

Each job runs in its own tokio task with independent intervals.

## Adding New Jobs

To add a new job:

1. Define job configuration in `scheduler.rs`:
```rust
let config = JobConfig::from_env("my-job", 300);
```

2. Add job logic:
```rust
scheduler.add_job(config, move || {
    Box::pin(async move {
        // Your job logic here
        Ok(())
    })
});
```

3. Add environment variables to `.env.example`:
```bash
JOB_MY_JOB_ENABLED=true
JOB_MY_JOB_INTERVAL_SECONDS=300
```

## Best Practices

1. **Keep jobs idempotent** - Jobs should be safe to run multiple times
2. **Handle errors gracefully** - Don't panic, log and continue
3. **Use appropriate intervals** - Balance freshness vs. resource usage
4. **Monitor job execution** - Check logs for failures
5. **Test job logic** - Ensure jobs work correctly before deploying

## Performance Impact

- Jobs run asynchronously and don't block API requests
- Cache warming reduces cold start latency
- Periodic refreshes distribute load over time
- Configurable intervals allow tuning for your workload

## Troubleshooting

**Job not running:**
- Check `JOB_<NAME>_ENABLED=true` in `.env`
- Verify logs for startup messages
- Ensure no syntax errors in configuration

**Job failing:**
- Check error logs for specific failure reason
- Verify network connectivity to external services
- Check Redis connection for cache operations

**High resource usage:**
- Increase job intervals to reduce frequency
- Disable non-critical jobs
- Monitor system resources during job execution
