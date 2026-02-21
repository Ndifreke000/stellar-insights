output "rds_endpoint" {
  description = "RDS instance endpoint address (hostname:port)"
  value       = aws_db_instance.postgresql.endpoint
  sensitive   = false
}

output "rds_address" {
  description = "RDS instance address (hostname only)"
  value       = aws_db_instance.postgresql.address
  sensitive   = false
}

output "rds_port" {
  description = "RDS instance port"
  value       = aws_db_instance.postgresql.port
  sensitive   = false
}

output "rds_resource_id" {
  description = "RDS resource ID (for IAM policies)"
  value       = aws_db_instance.postgresql.resource_id
  sensitive   = false
}

output "rds_arn" {
  description = "RDS instance ARN"
  value       = aws_db_instance.postgresql.arn
  sensitive   = false
}

output "db_subnet_group_id" {
  description = "DB subnet group ID"
  value       = aws_db_subnet_group.database.id
  sensitive   = false
}

output "kms_key_id" {
  description = "KMS key ID for RDS encryption"
  value       = aws_kms_key.rds.id
  sensitive   = false
}

output "rds_identifier" {
  description = "RDS instance identifier"
  value       = aws_db_instance.postgresql.identifier
  sensitive   = false
}
