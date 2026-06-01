# DynamoDB table for Terraform state locking
# Prevents concurrent modifications when multiple people run terraform apply
# One lock per state file (key = path/to/terraform.tfstate)

resource "aws_dynamodb_table" "terraform_locks" {
  name           = var.state_lock_table_name
  billing_mode   = "PAY_PER_REQUEST"  # No minimum cost, scales automatically
  hash_key       = "LockID"

  attribute {
    name = "LockID"
    type = "S"
  }

  point_in_time_recovery {
    enabled = true
  }

  server_side_encryption {
    enabled = true
  }

  ttl {
    attribute_name = "Expiration"
    enabled        = true
  }

  tags = {
    Name    = "Terraform State Locks"
    Purpose = "Prevent concurrent terraform applies"
  }
}

output "dynamodb_table_name" {
  description = "Name of DynamoDB table for state locking"
  value       = aws_dynamodb_table.terraform_locks.name
}

output "dynamodb_table_arn" {
  description = "ARN of DynamoDB lock table"
  value       = aws_dynamodb_table.terraform_locks.arn
}
