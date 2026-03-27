variable "aws_region" {
  description = "AWS region for dev environment"
  type        = string
  default     = "us-east-1"

  validation {
    condition     = contains(["us-east-1", "us-west-2", "eu-west-1", "eu-west-3"], var.aws_region)
    error_message = "Region must be one of: us-east-1, us-west-2, eu-west-1, eu-west-3"
  }
}

variable "environment" {
  description = "Environment name (dev)"
  type        = string
  default     = "dev"

  validation {
    condition     = var.environment == "dev"
    error_message = "This directory is for dev environment only"
  }
}

variable "vpc_cidr" {
  description = "VPC CIDR block (dev)"
  type        = string
  default     = "10.0.0.0/16"

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
  description = "Email for alarms (dev: not used)"
  type        = string
  default     = "noreply@example.com"
}
