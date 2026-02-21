variable "aws_region" {
  description = "AWS region for staging environment"
  type        = string
  default     = "us-east-1"

  validation {
    condition     = contains(["us-east-1", "us-west-2", "eu-west-1", "eu-west-3"], var.aws_region)
    error_message = "Region must be one of: us-east-1, us-west-2, eu-west-1, eu-west-3"
  }
}

variable "environment" {
  description = "Environment name (staging)"
  type        = string
  default     = "staging"

  validation {
    condition     = var.environment == "staging"
    error_message = "This directory is for staging environment only"
  }
}

variable "vpc_cidr" {
  description = "CIDR block for VPC (staging)"
  type        = string
  default     = "10.1.0.0/16"

  validation {
    condition     = can(cidrhost(var.vpc_cidr, 0))
    error_message = "VPC CIDR must be a valid CIDR block"
  }
}

variable "vault_addr" {
  description = "Vault server address (HCP endpoint)"
  type        = string
  default     = "https://vault-cluster.vault.11eb.aws.hashicorp.cloud:8200"

  validation {
    condition     = can(regex("^https://", var.vault_addr))
    error_message = "Vault address must be an HTTPS URL"
  }
}

variable "alarm_email" {
  description = "Email for CloudWatch alarm notifications"
  type        = string
  default     = "ops-team@stellar-insights.com"

  validation {
    condition     = can(regex("^[\\w.+-]+@[\\w.-]+\\.\\w+$", var.alarm_email))
    error_message = "Alarm email must be a valid email address"
  }
}
