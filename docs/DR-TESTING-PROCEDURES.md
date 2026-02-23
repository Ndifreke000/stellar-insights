# Disaster Recovery Testing Procedures

**Last Updated:** 2024  
**Document Owner:** DevOps Team

---

## Testing Schedule

| Test Type | Frequency | Duration | Participants | Next Test |
|-----------|-----------|----------|--------------|-----------|
| Backup Verification | Daily | 30 min | Automated | Daily |
| Database Restore | Weekly | 2 hours | DevOps | [Date] |
| Application Failover | Monthly | 4 hours | Tech Team | [Date] |
| Full DR Exercise | Quarterly | 8 hours | All Teams | [Date] |
| Tabletop Exercise | Bi-annually | 4 hours | Leadership | [Date] |

---

## Test 1: Daily Backup Verification

**Objective:** Verify backups are completing successfully

**Automated Script:**
```bash
#!/bin/bash
# /opt/scripts/daily-backup-check.sh

# Check database backup
LATEST_BACKUP=$(wal-g backup-list | tail -1)
if [ -z "$LATEST_BACKUP" ]; then
    echo "FAIL: No backups found"
    exit 1
fi

# Check backup age
BACKUP_TIME=$(echo $LATEST_BACKUP | awk '{print $2}')
AGE=$(( $(date +%s) - $(date -d "$BACKUP_TIME" +%s) ))

if [ $AGE -gt 86400 ]; then
    echo "FAIL: Latest backup older than 24 hours"
    exit 1
fi

# Check S3 connectivity
aws s3 ls s3://stellar-insights-backups/ > /dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "FAIL: Cannot access S3"
    exit 1
fi

echo "PASS: Backup verification successful"
exit 0
```

**Success Criteria:**
- [ ] Backup completed within last 24 hours
- [ ] S3 bucket accessible
- [ ] No errors in backup logs

---

## Test 2: Weekly Database Restore

**Objective:** Verify database can be restored from backup

**Procedure:**

### Step 1: Preparation (10 minutes)

```bash
# Create test environment
TEST_DIR="/tmp/restore-test-$(date +%Y%m%d)"
mkdir -p $TEST_DIR

# Document test start
echo "Test started: $(date)" > $TEST_DIR/test-log.txt
```

### Step 2: Restore Database (30 minutes)

```bash
# Restore latest backup
wal-g backup-fetch $TEST_DIR LATEST

# Start test PostgreSQL instance
docker run -d \
  --name postgres-restore-test \
  -v $TEST_DIR:/var/lib/postgresql/data \
  -e POSTGRES_PASSWORD=test \
  -p 5433:5432 \
  postgres:14

# Wait for startup
sleep 30
```

### Step 3: Validation (20 minutes)

```bash
# Test connectivity
docker exec postgres-restore-test psql -U postgres -c "SELECT version();"

# Verify databases exist
docker exec postgres-restore-test psql -U postgres -c "\l"

# Check table counts
docker exec postgres-restore-test psql -U postgres -d stellar_insights <<EOF
SELECT 'anchors' as table_name, COUNT(*) as count FROM anchors
UNION ALL
SELECT 'corridors', COUNT(*) FROM corridors
UNION ALL
SELECT 'metrics', COUNT(*) FROM metrics;
EOF

# Verify data integrity
docker exec postgres-restore-test psql -U postgres -d stellar_insights -c "
  SELECT 
    MAX(created_at) as latest_record,
    COUNT(*) as total_records
  FROM metrics;
"
```

### Step 4: Cleanup (10 minutes)

```bash
# Stop and remove test container
docker stop postgres-restore-test
docker rm postgres-restore-test

# Clean up test directory
rm -rf $TEST_DIR

# Document results
echo "Test completed: $(date)" >> /var/log/dr-tests.log
```

**Success Criteria:**
- [ ] Restore completes without errors
- [ ] Database starts successfully
- [ ] All tables accessible
- [ ] Data integrity checks pass
- [ ] Restore time < 30 minutes

**Documentation:**
```bash
# Record test results
cat >> /var/log/dr-tests.log <<EOF
---
Test: Weekly Database Restore
Date: $(date)
Backup: $(wal-g backup-list | tail -1)
Duration: [X minutes]
Status: [PASS/FAIL]
Notes: [Any observations]
---
EOF
```

---

## Test 3: Monthly Application Failover

**Objective:** Verify application can failover to backup instance

**Procedure:**

### Step 1: Pre-Test Validation (15 minutes)

```bash
# Verify both instances are healthy
curl -f http://primary:8080/health
curl -f http://backup:8080/health

# Check load balancer configuration
curl http://load-balancer/health

# Document baseline metrics
curl http://primary:8080/metrics > /tmp/baseline-metrics.txt
```

### Step 2: Simulate Primary Failure (5 minutes)

```bash
# Stop primary instance
docker stop stellar-insights-backend-primary

# Or if using systemd
sudo systemctl stop stellar-insights-backend@primary
```

### Step 3: Verify Failover (10 minutes)

```bash
# Check load balancer redirects to backup
for i in {1..10}; do
  curl -s http://load-balancer/health | jq '.instance'
  sleep 1
done

# Verify no errors
curl -f http://load-balancer/api/anchors

# Check response times
ab -n 100 -c 10 http://load-balancer/api/anchors
```

### Step 4: Restore Primary (10 minutes)

```bash
# Start primary instance
docker start stellar-insights-backend-primary

# Wait for health check
sleep 30

# Verify both instances healthy
curl -f http://primary:8080/health
curl -f http://backup:8080/health
```

**Success Criteria:**
- [ ] Failover completes within 5 minutes
- [ ] No user-facing errors
- [ ] Response times acceptable
- [ ] All services operational
- [ ] Primary restored successfully

---

## Test 4: Quarterly Full DR Exercise

**Objective:** Test complete disaster recovery process

**Participants:**
- Incident Commander
- Technical Lead
- DevOps Team
- Backend Engineers
- Frontend Engineers
- QA Team

**Duration:** 8 hours

### Phase 1: Planning (1 hour)

**Pre-Exercise Checklist:**
- [ ] Schedule maintenance window
- [ ] Notify stakeholders
- [ ] Prepare test environment
- [ ] Review runbooks
- [ ] Assign roles
- [ ] Set up communication channels

### Phase 2: Disaster Simulation (30 minutes)

**Scenario:** Complete infrastructure loss

```bash
# Simulate disaster (in test environment)
# 1. Stop all services
docker-compose down

# 2. Delete data directories
sudo rm -rf /var/lib/postgresql/14/main
sudo rm -rf /var/lib/redis

# 3. Remove application files
sudo rm -rf /opt/stellar-insights
```

### Phase 3: Recovery Execution (4 hours)

**Follow DR Runbooks:**

1. **Database Recovery** (1 hour)
   - Execute [DR-RUNBOOK-DATABASE.md](./DR-RUNBOOK-DATABASE.md)
   - Document time taken
   - Note any issues

2. **Application Recovery** (1 hour)
   - Execute [DR-RUNBOOK-APPLICATION.md](./DR-RUNBOOK-APPLICATION.md)
   - Document time taken
   - Note any issues

3. **Infrastructure Recovery** (2 hours)
   - Provision new infrastructure
   - Configure networking
   - Deploy applications
   - Verify connectivity

### Phase 4: Validation (1.5 hours)

**Validation Checklist:**
- [ ] All services running
- [ ] Database accessible
- [ ] API endpoints responding
- [ ] Frontend accessible
- [ ] Monitoring active
- [ ] Backups resuming
- [ ] No data loss beyond RPO
- [ ] Performance acceptable

**Test Scenarios:**
```bash
# 1. Test API endpoints
curl http://localhost:8080/api/anchors
curl http://localhost:8080/api/corridors
curl http://localhost:8080/api/metrics

# 2. Test database queries
psql -U postgres -d stellar_insights -c "
  SELECT COUNT(*) FROM anchors;
  SELECT COUNT(*) FROM corridors;
  SELECT MAX(created_at) FROM metrics;
"

# 3. Test frontend
curl http://localhost:3000/
curl http://localhost:3000/anchors
curl http://localhost:3000/corridors

# 4. Load testing
ab -n 1000 -c 50 http://localhost:8080/api/anchors
```

### Phase 5: Debrief (1 hour)

**Discussion Points:**
- What went well?
- What went wrong?
- Were RTOs met?
- Were RPOs met?
- Documentation gaps?
- Process improvements?

**Action Items:**
- Update runbooks
- Fix identified issues
- Improve automation
- Schedule follow-up

---

## Test 5: Bi-Annual Tabletop Exercise

**Objective:** Test incident response procedures without actual system changes

**Participants:**
- Executive Team
- Engineering Leadership
- DevOps Team
- Security Team
- Communications Team

**Duration:** 4 hours

### Exercise Format

**Scenario Presentation:** (30 minutes)
- Present disaster scenario
- Describe initial symptoms
- Provide timeline of events

**Response Simulation:** (2 hours)
- Teams discuss response
- Walk through procedures
- Identify decision points
- Document actions

**Evaluation:** (1 hour)
- Review response
- Identify gaps
- Discuss improvements
- Assign action items

**Follow-up:** (30 minutes)
- Document lessons learned
- Update procedures
- Schedule next exercise

### Sample Scenarios

1. **Database Corruption**
   - Symptoms: Query failures, checksum errors
   - Impact: Complete service outage
   - Response: PITR recovery

2. **Security Breach**
   - Symptoms: Unauthorized access detected
   - Impact: Data compromise
   - Response: Incident response procedures

3. **Cloud Provider Outage**
   - Symptoms: All services unreachable
   - Impact: Complete platform down
   - Response: Failover to backup region

---

## Test Documentation

### Test Report Template

```markdown
# DR Test Report

**Test ID:** DR-TEST-YYYY-MM-DD-NN
**Test Type:** [Type]
**Date:** YYYY-MM-DD
**Duration:** [X hours]
**Participants:** [Names]

## Objectives
- [Objective 1]
- [Objective 2]

## Procedure
[Steps taken]

## Results
- RTO Target: [X hours]
- RTO Actual: [Y hours]
- RPO Target: [X minutes]
- RPO Actual: [Y minutes]

## Issues Identified
1. [Issue 1]
2. [Issue 2]

## Action Items
1. [Action 1] - Owner: [Name] - Due: [Date]
2. [Action 2] - Owner: [Name] - Due: [Date]

## Recommendations
- [Recommendation 1]
- [Recommendation 2]

## Conclusion
[Pass/Fail] - [Summary]
```

### Test Results Tracking

**Location:** `/var/log/dr-tests/`

**Format:**
```
dr-tests/
├── 2024-01-15-database-restore.md
├── 2024-01-22-application-failover.md
├── 2024-02-01-full-dr-exercise.md
└── summary.csv
```

---

## Continuous Improvement

### After Each Test

1. **Document Results**
   - Record metrics
   - Note issues
   - Capture lessons learned

2. **Update Procedures**
   - Fix documentation gaps
   - Improve automation
   - Enhance monitoring

3. **Share Learnings**
   - Team meeting presentation
   - Update wiki
   - Training materials

### Quarterly Review

- Analyze test trends
- Review RTO/RPO compliance
- Update DR strategy
- Adjust testing schedule

---

## Automation

### Automated Testing Scripts

**Location:** `/opt/scripts/dr-tests/`

```bash
dr-tests/
├── daily-backup-check.sh
├── weekly-restore-test.sh
├── monthly-failover-test.sh
└── quarterly-full-test.sh
```

### Cron Schedule

```cron
# Daily backup verification
0 3 * * * /opt/scripts/dr-tests/daily-backup-check.sh

# Weekly restore test
0 2 * * 0 /opt/scripts/dr-tests/weekly-restore-test.sh

# Monthly failover test
0 2 1 * * /opt/scripts/dr-tests/monthly-failover-test.sh
```

---

## Success Metrics

### Key Performance Indicators

| Metric | Target | Current |
|--------|--------|---------|
| Backup Success Rate | 100% | [X%] |
| Restore Success Rate | 100% | [X%] |
| RTO Compliance | 100% | [X%] |
| RPO Compliance | 100% | [X%] |
| Test Completion Rate | 100% | [X%] |

### Trend Analysis

Track over time:
- Average restore time
- Test failure rate
- Issues identified
- Time to resolution

---

**Last Updated:** [Date]  
**Next Review:** [Date]  
**Contact:** devops@stellar-insights.com
