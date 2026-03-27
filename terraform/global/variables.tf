variable "aws_region" {
  description = "AWS region for all resources"
  type        = string
  default     = "us-east-1"

  validation {
    condition     = can(regex("^[a-z]{2}-[a-z]+-\\d{1}$", var.aws_region))
    error_message = "AWS region must be valid (e.g., us-east-1)"
  }
}

variable "environment" {
  description = "Environment name (dev/staging/production)"
  type        = string
  default     = "dev"

  validation {
    condition     = contains(["dev", "staging", "production"], var.environment)
    error_message = "Environment must be dev, staging, or production"
  }
}

variable "enable_mfa_delete" {
  description = "Enable MFA delete on S3 bucket (for production only)"
  type        = bool
  default     = false
}

variable "terraform_state_bucket_prefix" {
  description = "Prefix for Terraform state bucket name"
  type        = string
  default     = "stellar-insights"
}

variable "enable_versioning" {
  description = "Enable S3 versioning for state recovery"
  type        = bool
  default     = true
}

variable "state_lock_table_name" {
  description = "Name of DynamoDB table for Terraform locks"
  type        = string
  default     = "terraform-locks"
}

variable "ecr_image_retention_count" {
  description = "Number of images to retain in ECR (older images auto-deleted)"
  type        = number
  default     = 10

  validation {
    condition     = var.ecr_image_retention_count > 0 && var.ecr_image_retention_count <= 100
    error_message = "ECR image retention must be between 1-100"
  }
}

variable "tags" {
  description = "Common tags for all resources"
  type        = map(string)
  default = {
    Owner       = "DevOps"
    CostCenter  = "Infrastructure"
    Terraform   = "true"
  }
}
