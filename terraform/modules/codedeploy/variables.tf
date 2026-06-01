variable "cluster_name" {
  description = "ECS cluster name"
  type        = string
}

variable "service_name" {
  description = "ECS service name"
  type        = string
}

variable "listener_arn" {
  description = "Production HTTPS listener ARN"
  type        = string
}

variable "test_listener_arn" {
  description = "Test listener ARN (port 8443)"
  type        = string
}

variable "target_group_blue_name" {
  description = "Blue target group name"
  type        = string
}

variable "target_group_green_name" {
  description = "Green target group name"
  type        = string
}

variable "deployment_config_name" {
  description = "Deployment strategy"
  type        = string
  default     = "CodeDeployDefault.ECSCanary10Percent5Minutes"
}

variable "alb_arn_suffix" {
  description = "ALB ARN suffix for CloudWatch dimensions"
  type        = string
}

variable "target_group_blue_arn_suffix" {
  description = "Blue TG ARN suffix"
  type        = string
}

variable "target_group_green_arn_suffix" {
  description = "Green TG ARN suffix"
  type        = string
}

variable "environment" {
  description = "Environment name"
  type        = string

  validation {
    condition     = contains(["dev", "staging", "production"], var.environment)
    error_message = "Environment must be one of: dev, staging, production"
  }
}

variable "project" {
  description = "Project name"
  type        = string
  default     = "stellar-insights"
}

variable "termination_wait_time" {
  description = "Minutes to wait before terminating original tasks"
  type        = number
  default     = 5
}
