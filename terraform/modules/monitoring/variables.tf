variable "cluster_name" {
  description = "ECS cluster name for dashboard"
  type        = string
}

variable "log_group_names" {
  description = "Map of log group names (ecs, rds, alb)"
  type        = map(string)
  default     = {}
}

variable "alarm_email" {
  description = "Email address for SNS alarm notifications"
  type        = string

  validation {
    condition     = can(regex("^[\\w.+-]+@[\\w.-]+\\.\\w+$", var.alarm_email))
    error_message = "Alarm email must be a valid email address"
  }
}

variable "enable_dashboard" {
  description = "Enable CloudWatch dashboard creation"
  type        = bool
  default     = true
}

variable "enable_alarms" {
  description = "Enable CloudWatch alarms"
  type        = bool
  default     = true
}

variable "environment" {
  description = "Environment name (dev, staging, production)"
  type        = string

  validation {
    condition     = contains(["dev", "staging", "production"], var.environment)
    error_message = "Environment must be one of: dev, staging, production"
  }
}

variable "project" {
  description = "Project name for tagging"
  type        = string
  default     = "stellar-insights"
}

variable "enable_cost_alerts" {
  description = "Enable AWS Budgets cost alerts"
  type        = bool
  default     = true
}

variable "budget_limit" {
  description = "Monthly budget limit in USD"
  type        = string
  default     = "500"

  validation {
    condition     = can(regex("^[0-9]+$", var.budget_limit))
    error_message = "Budget limit must be a positive number"
  }
}
