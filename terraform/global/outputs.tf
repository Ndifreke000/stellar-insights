# Outputs for terraform/global
# These values are referenced by environment-specific modules

output "account_id" {
  description = "AWS account ID"
  value       = data.aws_caller_identity.current.account_id
}

output "aws_region" {
  description = "AWS region"
  value       = var.aws_region
}

output "environment" {
  description = "Environment name"
  value       = var.environment
}

# S3 state bucket outputs
output "terraform_state_bucket" {
  description = "S3 bucket for Terraform state"
  value       = aws_s3_bucket.terraform_state.id
}

output "terraform_state_bucket_arn" {
  description = "ARN of Terraform state bucket"
  value       = aws_s3_bucket.terraform_state.arn
}

# DynamoDB lock table outputs
output "terraform_lock_table" {
  description = "DynamoDB table for state locking"
  value       = aws_dynamodb_table.terraform_locks.name
}

output "terraform_lock_table_arn" {
  description = "ARN of DynamoDB lock table"
  value       = aws_dynamodb_table.terraform_locks.arn
}

# ECR outputs
output "ecr_backend_url" {
  description = "ECR URL for backend image"
  value       = aws_ecr_repository.backend.repository_url
}

output "ecr_frontend_url" {
  description = "ECR URL for frontend image"
  value       = aws_ecr_repository.frontend.repository_url
}

# IAM outputs
output "terraform_executor_role_arn" {
  description = "ARN for Terraform executor IAM role"
  value       = aws_iam_role.terraform_executor.arn
}
