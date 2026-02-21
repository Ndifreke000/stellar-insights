# Monitoring Module

Manages AWS CloudWatch logs, metrics, alarms, and dashboards for Stellar Insights.

## Features

- CloudWatch Log Groups for all services
- CloudWatch Alarms for infrastructure health
- SNS topics for alert notifications
- CloudWatch Dashboard for operational visibility
- Log retention policies (7-14-30 days based on environment)
- Metric aggregation and custom metrics

## Architecture

```
ECS Logs → CloudWatch Log Group
RDS Logs → CloudWatch Log Group
ALB Logs → CloudWatch Log Group
       ↓
   CloudWatch Metrics
       ↓
   CloudWatch Alarms → SNS → Email/Slack
       ↓
   CloudWatch Dashboard (ops view)
```

## Usage

```hcl
module "monitoring" {
  source = "../../modules/monitoring"

  cluster_name = module.compute.cluster_name
  log_group_names = {
    ecs = module.compute.log_group_name
    rds = module.database.log_group_name
    alb = module.load_balancing.log_group_name
  }
  
  alarm_email = "ops@stellar-insights.com"
  environment = var.environment
}
```

## Inputs

| Name | Description | Type | Required |
|------|-------------|------|----------|
| cluster_name | ECS cluster name | `string` | Yes |
| log_group_names | Map of log group names | `map(string)` | Yes |
| alarm_email | Email for SNS notifications | `string` | Yes |
| enable_dashboard | Enable CloudWatch dashboard | `bool` | No (default: `true`) |
| environment | Environment name | `string` | Yes |

## Outputs

| Name | Description |
|------|-------------|
| sns_topic_arn | SNS topic ARN for alarms |
| dashboard_url | CloudWatch dashboard URL |

## Monitoring Scope

**ECS Service Metrics:**
- CPU utilization (target: <70%)
- Memory utilization (target: <80%)
- Task count vs desired count
- Connection count
- Requests per second

**RDS Metrics:**
- CPU utilization (target: <70%)
- Database connections (target: <400)
- Free storage space (alert at <10%)
- Read/write latency
- Replication lag (Multi-AZ)

**ALB Metrics:**
- Request count
- Target response time (target: <1s)
- HTTP 4XX/5XX error rates
- Unhealthy host count

**Application Metrics:**
- Custom metrics from app (via stdout logs)
- Request duration percentiles
- Error rates by endpoint

## Notes

- Log retention: 7 days (dev), 14 days (staging), 30 days (production)
- Alarms use SNS for notifications (configure in AWS)
- Dashboard auto-refreshes every 5 minutes
- For Slack integration: use SNS → Lambda → Slack webhook
