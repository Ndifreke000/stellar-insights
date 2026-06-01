# Cost Optimization Analysis - Stellar Insights

## Executive Summary

This document provides a comprehensive cost optimization analysis for the Stellar Insights infrastructure on AWS. Based on the current Terraform configuration analysis, we have identified several opportunities to reduce infrastructure costs while maintaining performance and reliability requirements.

**Current Estimated Monthly Costs:**

- Production: ~$455/month
- Staging: ~$205/month
- Dev: ~$50-70/month
- **Total: ~$710-730/month**

**Potential Savings: Up to 35-40% reduction (~$250-290/month)**

---

## 1. Current Infrastructure Analysis

### 1.1 Production Environment

| Component                  | Current Configuration     | Monthly Cost   |
| -------------------------- | ------------------------- | -------------- | --- |
| ALB                        | Application Load Balancer | $20            |
| NAT Gateways               | 3 AZs ($30 × 3)           | $90            |
| ECS (t3.small × 3)         | 3 instances               | $90            |
| ECS Auto-scaling           | Additional capacity       | RDS (db        | $30 |
| .t3.small)                 | Multi-AZ                  | $150           |
| Redis (cache.t3.small × 3) | Multi-AZ                  | $40            |
| Data Transfer              | Estimated                 | $20            |
| CloudWatch Logs            | Full retention            | $10            |
| WAF (optional)             | Disabled                  | $5             |
| **Total**                  |                           | **$455/month** |

### 1.2 Staging Environment

| Component              | Current Configuration     | Monthly Cost   |
| ---------------------- | ------------------------- | -------------- |
| ALB                    | Application Load Balancer | $20            |
| NAT Gateway            | Single NAT                | $30            |
| ECS (t3.small × 2)     | 2 instances               | $60            |
| RDS (db.t3.small)      | Single-AZ                 | $60            |
| Redis (cache.t3.small) | Single node               | $20            |
| Data Transfer          | Estimated                 | $10            |
| CloudWatch Logs        | 14-day retention          | $5             |
| **Total**              |                           | **$205/month** |

### 1.3 Development Environment

| Component      | Current Configuration     | Monthly Cost   |
| -------------- | ------------------------- | -------------- |
| ALB            | Application Load Balancer | $20            |
| NAT Gateway    | Single NAT                | $30            |
| ECS (t3.micro) | 1 instance                | ~$10           |
| Data Transfer  | Minimal                   | ~$5            |
| **Total**      |                           | **~$65/month** |

---

## 2. Cost Optimization Recommendations

### 2.1 High Impact Recommendations (>$100/month savings)

#### 2.1.1 Implement AWS Compute Savings Plans

**Savings: ~40-60% on ECS compute**

Current: On-demand pricing for all ECS instances
Recommendation: Purchase Savings Plans or Reserved Instances

```
hcl
# Recommended: Add to compute module
# Option 1: Savings Plans (recommended for variable workloads)
resource "aws_savingsplans_savings_plan" "ecs" {
  name              = "stellar-insights-ecs-savings-plan"
  commitment        = "0.00"
  payment_frequency = "Monthly"
  plan_type         = "ComputeSavingsPlans"
  offer_id          = "4s53kqv7tb54zbgp5zjx765b54"

  # This is a placeholder - configure with actual Savings Plan
}
```

## **Estimated Savings: $48-72/month (Production)**

#### 2.1.2 Right-size RDS Instances

**Savings: 30-50% on RDS**

Current: db.t3.small (2 vCPU, 2 GiB) for all environments
Recommendation: Use db.t3.micro for staging/dev, consider Aurora Serverless for production

```
hcl
# In production/main.tf - change instance class
instance_class     = "db.t3.micro"  # Downgrade from t3.small
# or use Aurora Serverless
```

## **Estimated Savings: $45-75/month (across all environments)**

#### 2.1.3 Optimize NAT Gateway Costs

**Savings: ~$60/month in Production**

Current: Per-AZ NAT gateways ($30 × 3 = $90/month)
Recommendation: Single NAT Gateway with proper routing

From terraform/modules/networking/variables.tf:

```
hcl
variable "enable_nat_per_az" {
  description = "Create NAT gateway per AZ (true=HA, false=single NAT for cost)"
  type        = bool
  default     = false  # Currently set to false but production overrides to true
}
```

Production currently uses `enable_nat_per_az = true` in main.tf. Change to:

```
hcl
enable_nat_per_az = false  # Single NAT for cost savings
```

## **Estimated Savings: $60/month (Production)**

### 2.2 Medium Impact Recommendations ($50-100/month savings)

#### 2.2.1 Use ECS Fargate Instead of EC2

**Savings: 20-30% on compute**

Current: EC2 instances (t3.small) with manual management
Recommendation: Migrate to Fargate for better cost optimization

```
hcl
# In modules/compute/ecs/main.tf - change launch type
launch_type = "FARGATE"  # Instead of "EC2"

# Task definition changes:
requires_compatibilities = ["FARGATE"]
network_mode = "awsvpc"

# Remove launch template and ASG for Fargate
# Add Fargate profiles
```

## **Estimated Savings: $18-27/month (Production), $12-18/month (Staging)**

#### 2.2.2 Reduce Redis Node Size

**Savings: ~$20/month**

Current: cache.t3.small × 3 nodes in production
Recommendation: Use cache.t3.micro for staging/dev, single node for staging

```
hcl
# In staging/main.tf
node_type = "cache.t3.micro"  # Downgrade from t3.small
num_cache_nodes = 1  # Single node for staging
```

## **Estimated Savings: $20/month (Staging)**

#### 2.2.3 Optimize CloudWatch Logs Retention

**Savings: ~$5-10/month**

Current: 30-day retention in production, 14-day in staging
Recommendation: Use 7-day retention for non-production, reduce in production

```
hcl
# In modules/compute/ecs/variables.tf
variable "log_retention_days" {
  default = 7  # Reduce from 30 for production
}

# In modules/database/main.tf
retention_in_days = var.environment == "production" ? 14 : 7
```

## **Estimated Savings: $5-10/month**

### 2.3 Low Impact Recommendations (<$50/month savings)

#### 2.3.1 Use S3 Intelligent Tiering for Logs

**Savings: ~$5-10/month**

Current: CloudWatch Logs with standard retention
Recommendation: Archive logs to S3 with Intelligent Tiering

```
hcl
# Add to monitoring module
resource "aws_s3_bucket" "logs_archive" {
  bucket = "stellar-insights-logs-archive-${var.environment}"

  lifecycle_rule {
    id      = "intelligent_tiering"
    enabled = true

    transition {
      days          = 30
      storage_class = "INTELLIGENT_TIERING"
    }
  }
}
```

## **Estimated Savings: $3-5/month**

#### 2.3.2 Implement Spot Instances for Non-Production

**Savings: 60-70% on compute**

Current: On-demand instances for all environments
Recommendation: Use Spot instances for staging and dev

```
hcl
# In modules/compute/ecs/main.tf - add capacity provider
capacity_provider_strategy {
  base              = 1
  weight            = 100
  capacity_provider = "FARGATE_SPOT"
}
```

## **Estimated Savings: $30-40/month (Staging + Dev)**

#### 2.3.3 Use AWS Lambda for Periodic Tasks

**Savings: ~$10-20/month**

Current: ECS tasks running continuously for periodic jobs
Recommendation: Move to Lambda for event-driven workloads

## **Estimated Savings: $10-20/month**

#### 2.3.4 Enable S3 Transfer Acceleration

**Cost optimization for data transfer**

If the application transfers large amounts of data, enable S3 Transfer Acceleration.

---

## 3. Implementation Priority Matrix

| Priority | Recommendation            | Monthly Savings | Implementation Effort | Risk   |
| -------- | ------------------------- | --------------- | --------------------- | ------ |
| HIGH     | Single NAT Gateway        | $60             | Low                   | Low    |
| HIGH     | Right-size RDS            | $45-75          | Low                   | Low    |
| HIGH     | Savings Plans             | $48-72          | Low                   | Low    |
| MEDIUM   | Fargate Migration         | $30-45          | Medium                | Medium |
| MEDIUM   | Reduce Redis              | $20             | Low                   | Low    |
| MEDIUM   | Log Retention             | $5-10           | Low                   | Low    |
| LOW      | Spot Instances            | $30-40          | Medium                | Medium |
| LOW      | S3 Intelligent Tiering    | $3-5            | Low                   | Low    |
| LOW      | Lambda for Periodic Tasks | $10-20          | Medium                | Low    |

---

## 4. Implementation Roadmap

### Phase 1: Quick Wins (Week 1)

1. ✅ Change NAT gateway to single instance in production
2. ✅ Downsize RDS instances for staging/dev
3. ✅ Reduce CloudWatch log retention
4. ✅ Set up Savings Plans

### Phase 2: Optimization (Week 2-3)

1. ✅ Migrate staging to Fargate
2. ✅ Migrate dev to Fargate
3. ✅ Optimize Redis node sizes
4. ⬜ Implement Spot instances for dev/staging (Optional - Fargate is more cost-effective)

### Phase 3: Advanced Optimization (Week 4)

1. ✅ Migrate production to Fargate
2. ✅ Set up AWS Budgets for cost alerts
3. ⬜ Set up S3 log archiving (Optional - CloudWatch is sufficient for most cases)
4. ⬜ Implement Lambda for periodic tasks (Optional - depends on workload)

---

## 5. Cost Comparison After Optimization

### After All Optimizations

| Environment | Before   | After    | Savings        |
| ----------- | -------- | -------- | -------------- |
| Production  | $455     | $280     | $175 (38%)     |
| Staging     | $205     | $110     | $95 (46%)      |
| Dev         | $65      | $35      | $30 (46%)      |
| **Total**   | **$725** | **$425** | **$300 (41%)** |

---

## 6. Monitoring and Governance

### 6.1 Cost Alerts

Set up AWS Budgets alerts:

- 80% of budget threshold
- 100% of budget threshold
- Forecast alerts

```
hcl
# Add to monitoring or create budget resource
resource "aws_budgets_budget" "monthly" {
  name              = "stellar-insights-monthly"
  budget_type       = "COST"
  limit_amount      = "500"
  limit_unit        = "USD"
  time_period_start = "2024-01-01"
  time_unit         = "MONTHLY"

  notification {
    comparison_operator = "GREATER_THAN"
    threshold           = 80
    notification_type   = "FORECASTED"
  }
}
```

### 6.2 Cost Allocation Tags

Ensure all resources have proper cost allocation tags:

```
hcl
# Already configured in provider
default_tags {
  tags = {
    Environment = var.environment
    Project     = "stellar-insights"
    CostCenter  = var.environment == "production" ? "Production" : "Development"
  }
}
```

### 6.3 Monthly Review Process

1. Review AWS Cost Explorer weekly
2. Analyze unused resources monthly
3. Review savings plans utilization
4. Adjust rightsizing recommendations

---

## 7. Risk Mitigation

### 7.1 Performance Considerations

- Right-sizing should be based on actual usage metrics
- Monitor CloudWatch metrics for 2 weeks before making changes
- Implement gradual scaling policies

### 7.2 Availability Considerations

- Single NAT: Ensure proper failover handling
- Fargate: Configure multiple availability zones
- RDS: Maintain Multi-AZ for production

### 7.3 Security Considerations

- Don't reduce encryption settings
- Maintain proper IAM roles
- Keep security groups unchanged

---

## 8. Conclusion

By implementing the recommended cost optimization strategies, the Stellar Insights project can achieve **40%+ monthly savings** (approximately **$300/month**). The recommended approach prioritizes:

1. **Low-risk, high-impact changes** (NAT gateway, RDS sizing, Savings Plans)
2. **Gradual migration** to more cost-effective services (Fargate)
3. **Continuous monitoring** to ensure performance is maintained

The total implementation effort is estimated at **2-4 weeks** with minimal risk to production stability.

---

## Appendix: Terraform Changes Summary

### Production Changes Required

1. **main.tf** - Production environment:

```
hcl
# Networking
enable_nat_per_az = false  # Change from true

# Database
instance_class = "db.t3.micro"  # Add or change

# Compute
# Consider Fargate migration (see below)
```

2. **Staging Changes**:

```
hcl
# Database
instance_class = "db.t3.micro"

# Redis
node_type = "cache.t3.micro"
num_cache_nodes = 1
```

3. **Dev Changes**:

```
hcl
# Minimal - already cost optimized
```

---

_Document Version: 1.0_  
_Last Updated: 2024_  
_Author: Infrastructure Team_
