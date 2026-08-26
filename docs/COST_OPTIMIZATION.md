# Infrastructure Cost Optimization Analysis

## Executive Summary

This document analyzes the Stellar Insights infrastructure configuration and identifies cost optimization opportunities across compute, database, caching, and logging services. Recommendations range from configuration-only changes (no operational risk) to operational improvements that require operator review.

**Key Findings:**
- Missing CloudWatch log retention lifecycle policies (potential cost driver)
- RDS backup retention could be optimized for non-production environments
- ElastiCache log retention can be more aggressive for development
- Multi-AZ deployment decision should be environment-aware
- Performance Insights retention can be optimized

**Estimated Impact:** 15-25% potential cost reduction with safe changes, 25-40% with operator-reviewed recommendations.

---

## 1. CloudWatch Logs and Log Aggregation

### Current State
- ECS logs: Configurable retention, currently varies by environment
- Redis logs: Retention set to 14/7/3 days (prod/staging/dev)
- No explicit S3 log retention or archival policies
- Vault audit logs retained indefinitely in database

### Cost Drivers
- **Ingestion cost**: CloudWatch charges per GB ingested
- **Storage cost**: $0.50 per GB-month for logs stored
- **Long tail**: Log queries and analysis can increase retention costs

### Recommendations

#### 🟢 **[HIGH IMPACT, SAFE]** Add CloudWatch Log Retention Defaults
- **Change**: Set explicit retention periods for all CloudWatch log groups
- **Why**: Prevents indefinite retention that can silently accumulate cost
- **Implementation**: Add `retention_in_days` parameter to all log groups
- **Estimated savings**: 10-15% of log costs (if logs were indefinitely retained)
- **Risk**: None (applies going forward; existing logs still subject to any explicit retention)

```hcl
# Add to all CloudWatch log group definitions
retention_in_days = var.environment == "production" ? 30 : (var.environment == "staging" ? 14 : 7)
```

#### 🟡 **[MEDIUM IMPACT, REVIEW REQUIRED]** Aggregate Logs to S3 with Lifecycle Policies
- **Change**: Export CloudWatch logs to S3, apply S3 Lifecycle policies
- **Why**: S3 Glacier is 80% cheaper than CloudWatch for long-term retention
- **Implementation**: 
  - Create S3 bucket for log archive
  - Add S3 Lifecycle policy: move to Glacier after 30 days, delete after 365 days
  - Configure CloudWatch -> S3 export (manual or via Lambda)
- **Estimated savings**: 20-30% of log storage costs
- **Risk**: Low (logs still queryable in CloudWatch for configured retention period, older logs in Glacier require restore)
- **Operator decision**: Acceptable for audit/compliance logs?

#### 🟡 **[MEDIUM IMPACT, REVIEW REQUIRED]** Reduce Redis Slow-Log Retention for Non-Production
- **Change**: Decrease retention from 7 days to 3 days (staging), 1 day (dev)
- **Why**: Development and staging databases rarely require week-long query logs
- **Estimated savings**: 5% of log costs
- **Risk**: Low (slower query debugging, but queries usually reproduced quickly)
- **Operator decision**: Acceptable trade-off?

---

## 2. RDS Database

### Current State
- Instance class: `db.t3.medium` (default, can be overridden)
- Storage: Configurable, default GP3 with 3000 IOPS
- Backups: 7-day retention (default)
- Multi-AZ: Disabled by default, can be enabled
- Performance Insights: Enabled for non-dev, 31-day retention (prod), 7-day (others)
- Enhanced Monitoring: Disabled by default

### Cost Drivers
- **Compute**: t3.medium ~$35/month (on-demand, varies by region)
- **Backups**: ~$0.095 per GB-month; 7 days of daily backups = 7x allocated storage
- **Performance Insights**: $0.02 per vCPU-day (t3.medium = 1 vCPU) = ~$0.60/day
- **Multi-AZ**: Doubles storage and compute costs

### Recommendations

#### 🟢 **[LOW IMPACT, SAFE]** Optimize Backup Retention for Non-Production
- **Change**: Reduce backup retention for dev/staging to 3 days (production: keep 7-14 days)
- **Why**: Dev/staging databases can be rebuilt; production needs longer retention
- **Estimated savings**: 5-10% of RDS costs
- **Risk**: None (production unchanged, dev/staging easily rebuilt)
- **Implementation**: 
  ```hcl
  backup_retention_period = var.environment == "production" ? 7 : (var.environment == "staging" ? 3 : 1)
  ```

#### 🟢 **[LOW IMPACT, SAFE]** Disable Performance Insights for Development
- **Change**: Disable Performance Insights for dev environment
- **Why**: Not needed for dev; reduces cost without impacting developers (staging/prod unaffected)
- **Estimated savings**: 2-3% of RDS costs
- **Risk**: None (dev performance debugging less critical)
- **Implementation**:
  ```hcl
  enable_performance_insights = var.environment != "dev"
  ```

#### 🟡 **[MEDIUM IMPACT, OPERATOR REVIEW]** Right-Size Database Instances
- **Current**: `db.t3.medium` suitable for small workloads
- **Recommendation**: Review actual CPU/memory usage metrics in CloudWatch
- **Guidance**: 
  - If CPU <20% sustained and memory <30%: consider `db.t3.small`
  - If CPU >70% sustained: scale up to `db.t3.large`
- **Estimated savings**: 30-50% if downsize is appropriate (none if rightsize is upsize)
- **Risk**: Medium (downsizing without sufficient headroom causes performance issues)
- **Operator decision**: Required (needs production monitoring data)

#### 🟡 **[HIGH IMPACT, OPERATOR REVIEW]** Evaluate Multi-AZ for Non-Production
- **Current**: Multi-AZ disabled by default; can be enabled
- **Question**: Is Multi-AZ needed for dev/staging?
- **Estimated cost**: Multi-AZ doubles RDS cost (compute + storage + backups)
- **Recommendation**: 
  - Production: Multi-AZ recommended for HA
  - Staging: Consider single-AZ for cost efficiency (can tolerate downtime)
  - Dev: Single-AZ sufficient (not production-critical)
- **Risk**: Medium (loss of HA in staging; requires operator accept downtime risk)
- **Implementation**:
  ```hcl
  multi_az = var.environment == "production" ? true : false
  ```

#### 🟡 **[MEDIUM IMPACT, OPERATOR REVIEW]** Configure Auto-Scaling for Storage
- **Current**: Fixed allocated storage; manual expansion required
- **Recommendation**: Enable RDS Auto-Scaling to dynamically expand storage as needed
- **Benefit**: Allocate conservatively (20GB), scale automatically up to 100GB
- **Estimated savings**: 20-30% (if allocation is conservative)
- **Risk**: Low (automatic scaling prevents out-of-disk emergency)
- **Note**: Requires Terraform AWS provider 5.0+ for auto-scaling support

---

## 3. ElastiCache (Redis)

### Current State
- Node type: Configurable (default `cache.t3.micro` for small deployments)
- Nodes: Single node (no cluster mode)
- Automatic failover: Disabled by default
- Snapshots: Retention configurable per environment
- Encryption: At-rest and in-transit enabled
- Slow-log: Sent to CloudWatch

### Cost Drivers
- **Compute**: `cache.t3.micro` ~$15/month; `cache.t3.small` ~$30/month
- **Snapshots**: Minimal cost if retention is short
- **Failover**: Disabled, reducing cost
- **CloudWatch logs**: See log retention section

### Recommendations

#### 🟢 **[LOW IMPACT, SAFE]** Disable Snapshots for Non-Production Cache
- **Change**: Set `snapshot_retention_limit = 0` for dev/staging
- **Why**: Cache is ephemeral; loss of snapshot acceptable for non-production
- **Estimated savings**: 2-3% of ElastiCache costs
- **Risk**: None (cache rebuilt on restart; no data loss)
- **Implementation**:
  ```hcl
  snapshot_retention_limit = var.environment == "production" ? 7 : 0
  ```

#### 🟢 **[LOW IMPACT, SAFE]** Reduce Production Snapshot Retention
- **Change**: Reduce from 7 days to 5 days (or to RPO matching backup strategy)
- **Why**: Aligns with actual recovery needs
- **Estimated savings**: 3-5% of snapshot costs
- **Risk**: None (still covers week-long recovery window)

#### 🟡 **[MEDIUM IMPACT, OPERATOR REVIEW]** Evaluate Single-Node vs. Multi-Node
- **Current**: Single node (no failover)
- **Question**: Does cache failover matter for production?
- **Guidance**:
  - If cache loss is acceptable: single-node (current)
  - If cache loss affects SLA: enable automatic failover (adds ~30% cost)
- **Risk**: Medium (reduced reliability if failover is disabled but needed)
- **Operator decision**: Required

---

## 4. S3 and Object Storage

### Current State
- No explicit S3 lifecycle policies for log archives or old data
- Likely indefinite retention of backups, logs, and exports
- No versioning policies to manage old versions

### Cost Drivers
- **Storage**: $0.023 per GB-month (Standard)
- **Long tail**: Old backup snapshots accumulate cost over years
- **Exports**: User data exports not cleaned up after download expiration

### Recommendations

#### 🟢 **[HIGH IMPACT, SAFE]** Add S3 Lifecycle Policies to All Buckets
- **Change**: Add lifecycle rules to transition old objects to cheaper storage classes
- **Policy Template**:
  ```
  - Keep current version: 30 days in Standard
  - Archive to Glacier: 31+ days
  - Delete: 365+ days
  ```
- **Estimated savings**: 15-25% of S3 costs (if significant old data exists)
- **Risk**: None (objects still accessible, but slower retrieval from Glacier)
- **Implementation**: Add to each S3 bucket:
  ```hcl
  lifecycle_rule {
    enabled = true
    
    transition {
      days          = 30
      storage_class = "GLACIER"
    }
    
    expiration {
      days = 365
    }
  }
  ```

#### 🟢 **[MEDIUM IMPACT, SAFE]** Set Expiration on Temporary Objects
- **Change**: Add lifecycle rule to delete user exports after expiration
- **Why**: GDPR exports should not persist beyond download window anyway
- **Estimated savings**: 5-10% (if exports accumulate)
- **Risk**: None (matches GDPR retention policy)
- **Implementation**:
  ```hcl
  # For export bucket
  lifecycle_rule {
    filter {
      prefix = "exports/"
    }
    expiration {
      days = 7  # Matches download TTL
    }
  }
  ```

---

## 5. Load Balancing and Data Transfer

### Current State
- Application Load Balancer (ALB) configured
- Data transfer costs (especially inter-region) not optimized

### Cost Drivers
- **ALB**: ~$16/month + $0.006 per LCU
- **Data transfer**: $0.02 per GB (outbound to internet), higher for inter-region

### Recommendations

#### 🟡 **[MEDIUM IMPACT, OPERATOR REVIEW]** Optimize Data Transfer Patterns
- **Action**: Monitor CloudFront cache hit ratio and ALB access logs
- **Opportunity**: If mostly static content served, use CloudFront (cheaper egress)
- **Estimated savings**: 20-30% if static content significant
- **Risk**: Low (CDN adds complexity but reduces cost)
- **Operator decision**: Is content distribution warranted?

---

## 6. Summary of Recommendations

### Applied in This Commit (Safe, No Operator Review Needed)

| Change | Impact | Implementation |
|--------|--------|-----------------|
| Add CloudWatch log retention defaults | 10-15% log costs | Set retention_in_days on all log groups |
| Optimize backup retention (dev/staging) | 5-10% RDS costs | Reduce retention to 1-3 days |
| Disable Performance Insights (dev) | 2-3% RDS costs | Set enable_performance_insights = false for dev |
| Disable snapshots (non-prod cache) | 2-3% ElastiCache costs | Set snapshot_retention_limit = 0 for dev/staging |
| Add S3 lifecycle policies | 15-25% S3 costs | Transition old objects to Glacier after 30 days |

**Total Safe Savings: 15-25% of infrastructure costs**

### Recommended for Operator Review (Implementation Not Included)

| Change | Impact | Decision Required |
|--------|--------|-------------------|
| Reduce log retention further (staging/dev) | 5% log costs | Acceptable for slower debug? |
| Right-size database instances | 30-50% RDS costs | What actual CPU/memory utilization? |
| Disable Multi-AZ (non-production) | 20-30% RDS costs | Acceptable downtime in staging? |
| Enable cache failover (production) | +30% ElastiCache cost | Does cache loss matter for SLA? |

**Potential Additional Savings: 25-40% (operator-dependent)**

---

## 7. Implementation Checklist

### Phase 1: Safe Configuration Changes (This Commit)
- [ ] Add CloudWatch log retention to all log group definitions
- [ ] Update RDS backup retention variables for dev/staging
- [ ] Disable Performance Insights for development environment
- [ ] Disable Redis snapshots for non-production environments
- [ ] Add S3 lifecycle policies to all buckets

### Phase 2: Operator Review & Decisions
- [ ] Review CloudWatch dashboards for actual RDS utilization (CPU, memory)
- [ ] Decide: Multi-AZ for non-production environments (cost vs. availability)
- [ ] Decide: Cache failover strategy (cost vs. reliability)
- [ ] Decide: CDN/CloudFront for static content (complexity vs. cost)
- [ ] Audit S3 buckets for orphaned objects (backups, old exports)

### Phase 3: Ongoing Cost Monitoring
- [ ] Set up AWS Budgets for monthly cost alerts
- [ ] Review AWS Cost Explorer monthly for anomalies
- [ ] Monitor Reserved Instance coverage (purchase RIs if >50% consistent usage)
- [ ] Quarterly: re-evaluate instance sizes based on usage trends

---

## 8. Monitoring & Cost Tracking

### Recommended CloudWatch Custom Metrics
```
- Infrastructure/RDS/BackupStorageSize
- Infrastructure/ElastiCache/EvictionRate
- Infrastructure/S3/TotalBytes
- Infrastructure/CloudWatch/LogsIngested
```

### AWS Cost Explorer Queries
1. **By service**: Identify which service is largest cost driver
2. **By environment**: Compare prod/staging/dev costs
3. **By tag**: Track costs by cost center or team
4. **Trending**: Identify month-over-month cost increases

### Reserved Instance Opportunity Analysis
- Review Reserved Instance recommendations in AWS Console monthly
- If RDS/ElastiCache consistently high utilization: purchase 1-year RIs for 25-30% discount
- If compute (ECS) consistent: evaluate Compute Savings Plans

---

## 9. Design Decisions & Trade-offs

### Backup Retention for Development
**Decision**: Reduce dev backup retention to 1 day (instead of 7)
**Rationale**: Dev databases easily rebuilt; long-term backups not needed
**Trade-off**: Slightly slower disaster recovery if needed in dev (acceptable risk)

### Performance Insights for Development
**Decision**: Disable entirely for dev
**Rationale**: Dev performance debugging less critical than production
**Trade-off**: Developers lose detailed performance data (can enable on-demand if needed)

### S3 Lifecycle: Balance Between Cost and Retrieval Speed
**Decision**: Transition to Glacier after 30 days, delete after 365 days
**Rationale**: Covers 99% of retrieval scenarios while minimizing long-tail storage cost
**Trade-off**: Objects >30 days old have 12-hour retrieval lag from Glacier (acceptable for archives)

---

## 10. Future Enhancements (Out of Scope)

- **Auto-scaling**: RDS Aurora with read replicas for scaling
- **Spot instances**: Use Spot pricing for ECS tasks (up to 70% discount)
- **Serverless alternatives**: Lambda + RDS Proxy for low-volume workloads
- **Reserved Instances**: Annual purchasing for consistent workloads
- **Multi-region optimization**: Data locality strategies
