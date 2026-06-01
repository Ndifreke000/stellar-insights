# ============================================================================
# SNS Topic for Alarms
# ============================================================================

resource "aws_sns_topic" "alarms" {
  count = var.enable_alarms ? 1 : 0
  name  = "stellar-insights-alarms-${var.environment}"

  tags = {
    Name = "stellar-insights-alarms-${var.environment}"
  }
}

resource "aws_sns_topic_subscription" "alarm_email" {
  count     = var.enable_alarms ? 1 : 0
  topic_arn = aws_sns_topic.alarms[0].arn
  protocol  = "email"
  endpoint  = var.alarm_email
}

# ============================================================================
# CloudWatch Dashboard
# ============================================================================

resource "aws_cloudwatch_dashboard" "main" {
  count          = var.enable_dashboard ? 1 : 0
  dashboard_name = "stellar-insights-${var.environment}"

  dashboard_body = jsonencode({
    widgets = [
      {
        type = "metric"
        properties = {
          metrics = [
            ["AWS/ECS", "CPUUtilization", { stat = "Average" }],
            [".", "MemoryUtilization", { stat = "Average" }]
          ]
          period = 300
          stat   = "Average"
          region = data.aws_region.current.name
          title  = "ECS Cluster Health"
        }
      },
      {
        type = "metric"
        properties = {
          metrics = [
            ["AWS/RDS", "CPUUtilization", { stat = "Average" }],
            [".", "DatabaseConnections", { stat = "Average" }]
          ]
          period = 300
          stat   = "Average"
          region = data.aws_region.current.name
          title  = "RDS Performance"
        }
      },
      {
        type = "metric"
        properties = {
          metrics = [
            ["AWS/ApplicationELB", "TargetResponseTime", { stat = "Average" }],
            [".", "HTTPCode_Target_5XX_Count", { stat = "Sum" }]
          ]
          period = 60
          stat   = "Average"
          region = data.aws_region.current.name
          title  = "ALB Health"
        }
      },
      {
        type = "log"
        properties = {
          query   = "fields @timestamp, @message | stats count() by bin(300s)"
          region  = data.aws_region.current.name
          title   = "Log Volume"
          logGroupNames = values(var.log_group_names)
        }
      }
    ]
  })
}

# ============================================================================
# Data Sources
# ============================================================================

data "aws_region" "current" {}

# ============================================================================
# Outputs
# ============================================================================

output "sns_topic_arn" {
  description = "SNS topic ARN for alarms"
  value       = try(aws_sns_topic.alarms[0].arn, null)
}

output "dashboard_url" {
  description = "CloudWatch dashboard URL"
  value = try(
    "https://${data.aws_region.current.name}.console.aws.amazon.com/cloudwatch/home?region=${data.aws_region.current.name}#dashboards:name=stellar-insights-${var.environment}",
    null
  )
}

# ============================================================================
# AWS Budget for Cost Alerts
# ============================================================================

resource "aws_budgets_budget" "monthly" {
  count = var.enable_cost_alerts ? 1 : 0
  name  = "stellar-insights-monthly-${var.environment}"

  budget_type = "COST"
  limit_amount = var.budget_limit
  limit_unit = "USD"
  time_period_start = "2024-01-01"
  time_unit = "MONTHLY"

  # Cost filters
  cost_filter {
    name = "Environment"
    values = [var.environment]
  }

  notification {
    comparison_operator = "GREATER_THAN"
    threshold = 80
    notification_type = "FORECASTED"
    subscriber_sns_topic_arn = try(aws_sns_topic.alarms[0].arn, null)
  }

  notification {
    comparison_operator = "GREATER_THAN"
    threshold = 100
    notification_type = "ACTUAL"
    subscriber_sns_topic_arn = try(aws_sns_topic.alarms[0].arn, null)
  }
}

resource "aws_budgets_budget" "annual" {
  count = var.enable_cost_alerts ? 1 : 0
  name  = "stellar-insights-annual-${var.environment}"

  budget_type = "COST"
  limit_amount = var.budget_limit * 12
  limit_unit = "USD"
  time_period_start = "2024-01-01"
  time_unit = "YEARLY"

  # Cost filters
  cost_filter {
    name = "Environment"
    values = [var.environment]
  }

  notification {
    comparison_operator = "GREATER_THAN"
    threshold = 90
    notification_type = "FORECASTED"
    subscriber_sns_topic_arn = try(aws_sns_topic.alarms[0].arn, null)
  }
}
