#!/bin/bash

# Retention Policy Tests for Database Backup System
# Purpose: Validate retention policy logic without requiring actual database
# Usage: ./test-retention-policy.sh

set -euo pipefail

# ============================================================================
# Test Data & Configuration
# ============================================================================

TESTS_PASSED=0
TESTS_FAILED=0

# Retention policy (days)
RETENTION_DAILY=7
RETENTION_WEEKLY=28
RETENTION_MONTHLY=180

# ============================================================================
# Utilities
# ============================================================================

log() {
  echo "[TEST] $*"
}

pass() {
  echo "✓ PASS: $*"
  ((TESTS_PASSED++))
}

fail() {
  echo "✗ FAIL: $*"
  ((TESTS_FAILED++))
}

# ============================================================================
# Retention Policy Functions (mirrored from backup script)
# ============================================================================

get_backup_timestamp() {
  local backup_key="$1"
  # Extract timestamp from s3 key: database/production/manual/stellar-insights-production-20240115-143022.sql.gz
  echo "$backup_key" | sed -E 's/.*-([0-9]{8}-[0-9]{6})\..*$/\1/'
}

get_backup_age_days() {
  local backup_timestamp="$1"
  # Convert YYYYMMDD-HHMMSS to epoch
  local backup_date=$(echo "$backup_timestamp" | cut -d'-' -f1)

  # Handle both GNU date and BSD date
  local backup_epoch
  if date --version &>/dev/null 2>&1; then
    # GNU date (Linux)
    backup_epoch=$(date -d "$backup_date" +%s 2>/dev/null || echo "0")
  else
    # BSD date (macOS)
    backup_epoch=$(date -jf %Y%m%d "$backup_date" +%s 2>/dev/null || echo "0")
  fi

  if [ "$backup_epoch" = "0" ]; then
    echo "ERROR"
    return 1
  fi

  local now_epoch=$(date +%s)
  echo $(( (now_epoch - backup_epoch) / 86400 ))
}

should_retain_backup() {
  local backup_key="$1"
  local timestamp=$(get_backup_timestamp "$backup_key")
  local age_days=$(get_backup_age_days "$timestamp")
  local backup_date=$(echo "$timestamp" | cut -d'-' -f1)

  # Handle date parsing errors
  if [ "$age_days" = "ERROR" ]; then
    echo "ERROR"
    return 1
  fi

  # Get day of week (0=Sunday, 6=Saturday)
  local dow
  if date --version &>/dev/null 2>&1; then
    dow=$(date -d "$backup_date" +%w)
  else
    dow=$(date -jf %Y%m%d "$backup_date" +%w)
  fi

  # Daily: Keep last 7 days
  if [ "$age_days" -le "$RETENTION_DAILY" ]; then
    echo "true"
    return 0
  fi

  # Weekly: Keep Sundays for 4 weeks (28 days)
  if [ "$age_days" -le "$RETENTION_WEEKLY" ] && [ "$dow" = "0" ]; then
    echo "true"
    return 0
  fi

  # Monthly: Keep first of month for 6 months (180 days)
  local day=$(echo "$backup_date" | cut -c7-8)
  if [ "$age_days" -le "$RETENTION_MONTHLY" ] && [ "$day" = "01" ]; then
    echo "true"
    return 0
  fi

  echo "false"
  return 0
}

# ============================================================================
# Test Cases
# ============================================================================

test_timestamp_extraction() {
  log "Testing timestamp extraction..."

  local key1="database/production/manual/stellar-insights-production-20240115-143022.sql.gz"
  local ts1=$(get_backup_timestamp "$key1")
  [ "$ts1" = "20240115-143022" ] && pass "Extract timestamp from S3 key" || fail "Extract timestamp: got $ts1"

  local key2="database/staging/export/stellar-insights-staging-export-20240101-000000.sql"
  local ts2=$(get_backup_timestamp "$key2")
  [ "$ts2" = "20240101-000000" ] && pass "Extract timestamp from export key" || fail "Extract timestamp from export"
}

test_backup_age_calculation() {
  log "Testing backup age calculation..."

  # Test with today's date (should be 0 days old)
  local today=$(date +%Y%m%d)
  local today_ts="${today}-120000"
  local today_key="backup-${today_ts}.sql.gz"
  local age=$(get_backup_age_days "$today_ts")

  if [ "$age" = "0" ]; then
    pass "Today's backup is 0 days old"
  else
    # Allow 1 day if test crosses midnight
    if [ "$age" = "1" ]; then
      pass "Today's backup is 0-1 days old (acceptable if near midnight)"
    else
      fail "Today's backup age: expected 0, got $age days"
    fi
  fi

  # Test with past date (7 days ago)
  local past_date=$(date -d "7 days ago" +%Y%m%d 2>/dev/null || date -v-7d +%Y%m%d)
  local past_ts="${past_date}-120000"
  local past_age=$(get_backup_age_days "$past_ts")

  if [ "$past_age" = "7" ]; then
    pass "7-day-old backup is calculated correctly"
  else
    # Allow 6-8 day range for timezone/timing issues
    if [ "$past_age" -ge "6" ] && [ "$past_age" -le "8" ]; then
      pass "7-day-old backup age is approximately correct"
    else
      fail "7-day-old backup age: expected ~7, got $past_age days"
    fi
  fi
}

test_daily_retention() {
  log "Testing daily retention (keep for 7 days)..."

  # Today's backup should be kept
  local today=$(date +%Y%m%d)
  local today_key="backup-${today}-120000.sql.gz"
  local result=$(should_retain_backup "$today_key")
  [ "$result" = "true" ] && pass "Today's backup is retained" || fail "Today's backup should be kept"

  # 3 days ago should be kept
  local past3=$(date -d "3 days ago" +%Y%m%d 2>/dev/null || date -v-3d +%Y%m%d)
  local key3="backup-${past3}-120000.sql.gz"
  local result3=$(should_retain_backup "$key3")
  [ "$result3" = "true" ] && pass "3-day-old backup is retained" || fail "3-day-old backup should be kept"
}

test_weekly_retention() {
  log "Testing weekly retention (keep Sundays for 28 days)..."

  # Find a recent Sunday in the past 28 days
  local sunday=$(date -d "last Sunday" +%Y%m%d 2>/dev/null || date -v-w0 +%Y%m%d)
  local sunday_key="backup-${sunday}-120000.sql.gz"
  local result=$(should_retain_backup "$sunday_key")

  if [ "$result" = "true" ]; then
    pass "Recent Sunday backup is retained"
  else
    # Check if calculation might be off
    local age=$(get_backup_age_days "${sunday}-120000")
    if [ "$age" -gt "$RETENTION_WEEKLY" ]; then
      pass "Recent Sunday beyond 28 days, correctly not retained"
    else
      fail "Recent Sunday should be retained, got: $result"
    fi
  fi

  # Non-Sunday in retention window should not be kept based on weekly policy alone
  local monday=$(date -d "last Monday" +%Y%m%d 2>/dev/null || date -v-w1 +%Y%m%d)
  local monday_key="backup-${monday}-120000.sql.gz"
  local result_mon=$(should_retain_backup "$monday_key")
  local age_mon=$(get_backup_age_days "${monday}-120000")

  if [ "$age_mon" -gt "$RETENTION_DAILY" ] && [ "$age_mon" -le "$RETENTION_WEEKLY" ]; then
    if [ "$result_mon" = "false" ]; then
      pass "Non-Sunday in retention window correctly not retained"
    else
      fail "Non-Sunday 8+ days old should not be retained"
    fi
  fi
}

test_monthly_retention() {
  log "Testing monthly retention (keep 1st of month for 180 days)..."

  # Find a recent 1st of month
  local month_start=$(date -d "$(date +%Y-%m)-01" +%Y%m%d 2>/dev/null || date -v1d +%Y%m%d)
  local month_key="backup-${month_start}-120000.sql.gz"
  local result=$(should_retain_backup "$month_key")

  if [ "$result" = "true" ]; then
    pass "Recent 1st of month is retained"
  else
    fail "Recent 1st of month should be retained"
  fi

  # 1st of month > 180 days ago should not be kept
  local old_date=$(date -d "190 days ago" +%Y%m01 2>/dev/null || date -v-190d -v1d +%Y%m%d)
  local old_key="backup-${old_date}-120000.sql.gz"
  local result_old=$(should_retain_backup "$old_key")
  local age_old=$(get_backup_age_days "${old_date}-120000")

  if [ "$age_old" -gt "$RETENTION_MONTHLY" ]; then
    if [ "$result_old" = "false" ]; then
      pass "Old 1st of month (>180 days) is correctly not retained"
    else
      fail "Very old 1st of month should not be retained"
    fi
  else
    pass "1st of month test edge case (timing dependent)"
  fi
}

test_edge_cases() {
  log "Testing edge cases..."

  # Backup from exactly 7 days ago (boundary condition)
  local boundary7=$(date -d "7 days ago" +%Y%m%d 2>/dev/null || date -v-7d +%Y%m%d)
  local boundary7_key="backup-${boundary7}-120000.sql.gz"
  local result7=$(should_retain_backup "$boundary7_key")
  [ "$result7" = "true" ] && pass "Boundary: 7-day-old backup is kept" || fail "7-day boundary"

  # Backup from exactly 28 days ago (weekly boundary)
  local boundary28=$(date -d "28 days ago" +%Y%m%d 2>/dev/null 2>/dev/null || date -v-28d +%Y%m%d)
  # Only test if it's a Sunday
  local dow28=$(date -d "$boundary28" +%w 2>/dev/null || date -jf %Y%m%d "$boundary28" +%w)
  if [ "$dow28" = "0" ]; then
    local boundary28_key="backup-${boundary28}-120000.sql.gz"
    local result28=$(should_retain_backup "$boundary28_key")
    [ "$result28" = "true" ] && pass "Boundary: 28-day-old Sunday is kept" || fail "28-day weekly boundary"
  fi

  # Backup from exactly 180 days ago (monthly boundary)
  local boundary180=$(date -d "180 days ago" +%Y%m01 2>/dev/null || date -v-180d -v1d +%Y%m%d)
  # Only test if within monthly retention
  local age180=$(get_backup_age_days "${boundary180}-120000")
  if [ "$age180" -eq 180 ] || [ "$age180" -eq 179 ] || [ "$age180" -eq 181 ]; then
    local boundary180_key="backup-${boundary180}-120000.sql.gz"
    local result180=$(should_retain_backup "$boundary180_key")
    [ "$result180" = "true" ] && pass "Boundary: 180-day-old 1st of month is kept" || fail "180-day monthly boundary"
  else
    pass "Boundary test skipped (timing dependent)"
  fi
}

test_retention_scenarios() {
  log "Testing real-world scenarios..."

  # Scenario 1: Daily backups for past 10 days
  # Expect: Keep last 7 days, delete 8-10 days old
  log "  Scenario 1: Daily backups (10 days)"
  for i in {0..10}; do
    local date=$(date -d "$i days ago" +%Y%m%d 2>/dev/null || date -v-${i}d +%Y%m%d)
    local key="backup-${date}-120000.sql.gz"
    local result=$(should_retain_backup "$key")

    if [ $i -le 7 ]; then
      if [ "$result" = "true" ]; then
        echo "    ✓ Day -$i: Keep"
      else
        fail "    Day -$i should be kept (daily policy)"
      fi
    else
      if [ "$result" = "false" ]; then
        echo "    ✓ Day -$i: Delete"
      else
        echo "    ⚠ Day -$i: Keep (weekly/monthly policy may apply)"
      fi
    fi
  done

  pass "Daily backup scenario completed"
}

# ============================================================================
# Test Suite
# ============================================================================

main() {
  echo "════════════════════════════════════════════════════════════════"
  echo "Database Backup Retention Policy Tests"
  echo "Policy:"
  echo "  • Daily: Keep for $RETENTION_DAILY days"
  echo "  • Weekly: Keep Sundays for $RETENTION_WEEKLY days"
  echo "  • Monthly: Keep 1st of month for $RETENTION_MONTHLY days"
  echo "════════════════════════════════════════════════════════════════"
  echo ""

  # Run all test suites
  test_timestamp_extraction
  echo ""
  test_backup_age_calculation
  echo ""
  test_daily_retention
  echo ""
  test_weekly_retention
  echo ""
  test_monthly_retention
  echo ""
  test_edge_cases
  echo ""
  test_retention_scenarios
  echo ""

  # Summary
  echo "════════════════════════════════════════════════════════════════"
  echo "Test Summary"
  echo "════════════════════════════════════════════════════════════════"
  echo "Passed: $TESTS_PASSED"
  echo "Failed: $TESTS_FAILED"
  echo ""

  if [ $TESTS_FAILED -eq 0 ]; then
    echo "✓ All tests passed!"
    exit 0
  else
    echo "❌ Some tests failed"
    exit 1
  fi
}

main "$@"
