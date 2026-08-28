# Blue-Green Deployment Strategy

This guide covers the zero-downtime blue-green deployment strategy for Stellar Insights backend.

## Overview

Blue-green deployments enable zero-downtime releases by running two identical production environments:

- **Blue (Current)**: The live production environment serving all traffic
- **Green (New)**: The new release candidate, tested before receiving traffic

After green passes health checks, traffic is switched. If issues arise, traffic reverts to blue instantly.

## Architecture

```
┌─────────────────────────────────────┐
│     Internet Traffic (HTTPS)         │
└──────────────────┬──────────────────┘
                   │
          ┌────────▼────────┐
          │ Application     │
          │ Load Balancer   │
          │ (Port 443)      │
          └────────┬────────┘
                   │
        ┌──────────┴──────────┐
        │                     │
  ┌─────▼──────┐        ┌─────▼──────┐
  │  Blue      │        │  Green     │
  │  Target    │        │  Target    │
  │  Group     │        │  Group     │
  └─────┬──────┘        └─────┬──────┘
        │                     │
  ┌─────▼──────────────────────▼─────┐
  │   ECS Tasks (Service)            │
  │  Blue  │  Blue  │ Green │ Green  │
  └────────┴────────┴───────┴────────┘

Test Traffic (Port 8443) → Green Target Group
Production Traffic (Port 443) → Blue Target Group
```

## Deployment Workflow

### 1. Pre-Deployment (Manual or Automated)

```bash
# Verify database migrations are compatible
# See: docs/DATABASE_MIGRATIONS.md - Expand-Contract Pattern
```

### 2. Build & Register (Automated via GitHub Actions)

1. Code pushed to main branch
2. Backend changes trigger the deploy workflow
3. Docker image built and pushed to ECR
4. New task definition registered with ECS

### 3. Green Environment Launch (CodeDeploy)

```
CodeDeploy creates new ECS tasks in the green target group:
  - Tasks launched with new image
  - Initial count = current blue task count
  - Migration code runs during container startup
  - Waits for all green tasks to report healthy
```

### 4. Health Check Validation

```
ALB Health Checks (continuous):
  - Interval: 5 seconds
  - Healthy threshold: 2 consecutive checks
  - Timeout: 3 seconds
  - Success codes: 200
  
Green tasks must pass health checks before traffic cutover
If health checks fail:
  - Traffic cutover is blocked
  - Green tasks remain running for debugging
  - Auto-rollback triggers based on CloudWatch alarms
  - Blue continues serving 100% of traffic
```

### 5. Traffic Cutover (Automatic)

Once green passes health checks:

```
Phase 1: Test Traffic (Optional Manual Validation)
  - ALB listener on port 8443 routes to green
  - Test team can validate via https://prod.example.com:8443
  - No impact to production traffic

Phase 2: Production Traffic Switch
  - ALB listener on port 443 transitions from blue → green
  - Sticky sessions drain gracefully (30s deregistration delay)
  - Blue tasks continue handling in-flight requests
  - All new requests go to green
```

### 6. Monitoring & Auto-Rollback

During and after cutover, CloudWatch alarms monitor:

| Alarm                          | Threshold | Action                  |
| ------------------------------ | --------- | ----------------------- |
| Green 5XX Error Rate           | > 5/min   | Trigger rollback        |
| Green Response Time            | > 2 sec   | Trigger rollback        |
| Green Unhealthy Host Count     | > 0       | Trigger rollback        |
| ALB Response Time              | > 1 sec   | Alert (informational)   |
| ALB Unhealthy Hosts            | > 0       | Alert (informational)   |

If an alarm fires during deployment:

```
Auto-Rollback:
  1. Traffic immediately reverts to blue
  2. Green tasks continue running (for debugging)
  3. On-call team is paged (via CloudWatch)
  4. Manual intervention required to investigate or retry
```

### 7. Blue Termination

```
After deployment succeeds:
  - Monitoring grace period: 5 minutes (configurable)
  - No errors or unhealthy instances reported
  - Blue tasks are terminated gracefully
  - Database connection pooling drains
  - Green becomes the new blue for next deployment
```

## Database Migration Constraints

Migrations must be **backward-compatible** during the cutover window when both blue and green run simultaneously against the same database.

### ✅ Safe Migration Pattern (Expand-Contract)

**Deployment 1 (expand):**
```sql
-- Add new column (backward-compatible: old code ignores, new code will use)
ALTER TABLE users ADD COLUMN email_v2 TEXT;
```
- Old (blue) code: continues using `email` column
- New (green) code: starts using `email_v2` column
- Backfill job: gradually populates `email_v2` from `email`

**Deployment 2 (contract):**
```sql
-- Remove old column after code fully migrated
ALTER TABLE users DROP COLUMN email;
```

### ❌ Unsafe Migration Pattern (Immediate Replace)

```sql
-- Direct replacement breaks blue if it tries to use old column
ALTER TABLE users DROP COLUMN email;
ALTER TABLE users ADD COLUMN email_v2 TEXT;
```

This causes blue to fail with "column not found" if traffic doesn't cutover instantly.

## Manual Rollback Procedure

### If Auto-Rollback Fails or Manual Intervention Needed

**Prerequisites:**
- AWS credentials configured
- On-call database access

**Steps:**

```bash
# 1. Identify the deployment
DEPLOYMENT_ID="d-XXXXXXXXX"  # From GitHub Actions or AWS Console
aws deploy get-deployment --deployment-id $DEPLOYMENT_ID

# 2. Stop the deployment (if still in progress)
aws deploy stop-deployment \
  --deployment-id $DEPLOYMENT_ID \
  --auto-rollback-enabled

# 3. Manually revert traffic to blue (if auto-revert didn't work)
aws elbv2 modify-listener \
  --listener-arn arn:aws:elasticloadbalancing:... \
  --default-actions Type=forward,TargetGroupArn=<BLUE_TG_ARN>

# 4. Terminate green tasks (optional)
aws ecs update-service \
  --cluster stellar-insights-production \
  --service stellar-insights-service \
  --desired-count 3  # Back to blue count

# 5. Verify rollback
curl -I https://api.stellar-insights.example.com/health
```

### Rollback from Database Issue

If the issue is a bad migration:

```bash
# 1. Stop application deployments
# 2. Connect to database
psql $DB_URL

# 3. Run rollback migration
\i backend/migrations/NNN_description.down.sql

# 4. Verify schema
\d users

# 5. Restart services
# 6. Investigate root cause of migration
```

See [DATABASE_MIGRATIONS.md](./DATABASE_MIGRATIONS.md#rollback-procedures) for detailed migration rollback steps.

## Monitoring & Debugging

### During Deployment

```bash
# Watch ECS service state
aws ecs describe-services \
  --cluster stellar-insights-production \
  --services stellar-insights-service

# Check ALB target health
aws elbv2 describe-target-health \
  --target-group-arn arn:aws:elasticloadbalancing:...

# View CloudWatch logs
aws logs tail /ecs/stellar-insights-production --follow
```

### Green Task Logs

```bash
# If green is unhealthy, check container logs
aws logs tail /ecs/stellar-insights-production --follow \
  --filter-pattern "ERROR"
```

### Deployment Status

```bash
# Check deployment progress
watch -n 5 'aws deploy get-deployment \
  --deployment-id d-XXXXXXXXX \
  --query "deploymentInfo.[status,creator,createTime]"'
```

## Post-Deployment Validation

After successful traffic cutover:

1. **Error Rate**: Confirm no spike in 5XX errors
2. **Latency**: Confirm response times within baseline
3. **Database**: Confirm migrations applied successfully
4. **Logs**: Spot-check application logs for errors
5. **Metrics**: Verify key business metrics stable

If any issue detected within the grace period, rollback is automatic. Manual rollback only if monitoring grace period has passed.

## Configuration

Blue-green deployment is controlled by:

**Terraform variables** (`terraform/environments/production/terraform.tfvars`):
```hcl
enable_blue_green = true
test_listener_port = 8443
termination_wait_time = 5
```

**GitHub Actions secrets** (`.github/workflows/deploy.yml`):
```yaml
AWS_ACCOUNT_ID          # AWS account for assume-role
AWS_REGION              # Deployment region (default: us-east-1)
```

**CodeDeploy configuration** (terraform/modules/codedeploy):
```hcl
deployment_config_name = "CodeDeployDefault.ECSAllAtOnce"  # Can be "CodeDeployDefault.ECSCanary10Percent5Minutes"
auto_rollback_events = ["DEPLOYMENT_FAILURE", "DEPLOYMENT_STOP_ON_ALARM"]
```

## Best Practices

1. **Always test in staging first** — Deploy to staging environment before production
2. **Use expand-contract for schema changes** — Ensures both blue & green work with same schema
3. **Monitor actively** — Watch CloudWatch dashboards during cutover (usually 2-5 minutes)
4. **Plan maintenance windows** — If rollback is needed, have on-call ready
5. **Document deployment events** — Log who deployed what and when in a deployment log
6. **Validate migrations locally** — Test migrations before committing (see DATABASE_MIGRATIONS.md)

## Troubleshooting

### Green Tasks Stuck in Pending

**Cause**: Not enough ECS cluster capacity or security group restrictions

**Fix**:
```bash
aws ecs describe-services \
  --cluster stellar-insights-production \
  --services stellar-insights-service \
  --query 'services[0].events[0:3]'
```

### Health Check Failing

**Cause**: App not responding to `GET /health` or slow startup

**Fix**:
```bash
# SSH to a green task and curl health endpoint
curl http://localhost:8080/health

# Check logs for startup errors
docker logs <CONTAINER_ID>
```

### Traffic Not Switching

**Cause**: ALB listener misconfiguration or target group empty

**Fix**:
```bash
# Verify target group registration
aws elbv2 describe-target-health \
  --target-group-arn <TG_ARN> | jq '.TargetHealthDescriptions'

# Verify listener routing
aws elbv2 describe-listeners \
  --load-balancer-arn <LB_ARN> \
  --query 'Listeners[?Port==443]'
```

## References

- [AWS Blue-Green Deployments](https://docs.aws.amazon.com/whitepapers/latest/blue-green-deployments/)
- [CodeDeploy ECS Deployments](https://docs.aws.amazon.com/codedeploy/latest/userguide/deployments-create-ecs)
- [ALB Target Groups](https://docs.aws.amazon.com/elasticloadbalancing/latest/application/application-load-balancers.html)
- [Database Migration Strategy](./DATABASE_MIGRATIONS.md)
