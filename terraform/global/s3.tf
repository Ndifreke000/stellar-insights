# S3 bucket for Terraform remote state
# This bucket stores the Terraform state file for all infrastructure
# Security: versioning, encryption, block public access, MFA delete

# Generate unique bucket name
resource "random_string" "bucket_suffix" {
  length  = 8
  special = false
  lower   = true
}

locals {
  state_bucket_name = "${var.terraform_state_bucket_prefix}-terraform-state-${data.aws_caller_identity.current.account_id}"
}

resource "aws_s3_bucket" "terraform_state" {
  bucket = local.state_bucket_name

  tags = {
    Name      = "Terraform State"
    Purpose   = "Remote state for Terraform"
    Lifecycle = "Critical"
  }
}

# Enable versioning to recover previous states
resource "aws_s3_bucket_versioning" "terraform_state" {
  bucket = aws_s3_bucket.terraform_state.id

  versioning_configuration {
    status     = var.enable_versioning ? "Enabled" : "Suspended"
    mfa_delete = var.enable_mfa_delete ? "Enabled" : "Disabled"
  }
}

# Enable server-side encryption
resource "aws_s3_bucket_server_side_encryption_configuration" "terraform_state" {
  bucket = aws_s3_bucket.terraform_state.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

# Block all public access
resource "aws_s3_bucket_public_access_block" "terraform_state" {
  bucket = aws_s3_bucket.terraform_state.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Enable access logging
resource "aws_s3_bucket_logging" "terraform_state" {
  bucket = aws_s3_bucket.terraform_state.id

  target_bucket = aws_s3_bucket.terraform_state_logs.id
  target_prefix = "terraform-state-logs/"
}

# Separate bucket for access logs
resource "aws_s3_bucket" "terraform_state_logs" {
  bucket = "${local.state_bucket_name}-logs"

  tags = {
    Name    = "Terraform State Logs"
    Purpose = "Access logs for state bucket"
  }
}

resource "aws_s3_bucket_public_access_block" "terraform_state_logs" {
  bucket = aws_s3_bucket.terraform_state_logs.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Lifecycle policy to delete old logs after 90 days
resource "aws_s3_bucket_lifecycle_configuration" "terraform_state_logs" {
  bucket = aws_s3_bucket.terraform_state_logs.id

  rule {
    id     = "delete-old-logs"
    status = "Enabled"

    expiration {
      days = 90
    }
  }
}

# Data source for AWS account ID (used in bucket naming)
data "aws_caller_identity" "current" {}

# Output bucket information
output "state_bucket_name" {
  description = "Name of the S3 bucket for Terraform state"
  value       = aws_s3_bucket.terraform_state.id
}

output "state_bucket_arn" {
  description = "ARN of the S3 bucket for Terraform state"
  value       = aws_s3_bucket.terraform_state.arn
}

output "state_bucket_region" {
  description = "Region where state bucket is located"
  value       = aws_s3_bucket.terraform_state.region
}
