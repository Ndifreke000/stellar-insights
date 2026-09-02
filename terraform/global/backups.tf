# S3 bucket for continuous SQLite backups (Litestream replication target).
#
# Addresses the last unchecked item in ADR 0001's hardening checklist:
# "Durability story. SQLite's backup is a file copy, but a *consistent*
# one needs care under WAL. Litestream (or an equivalent) gives continuous
# replication; backup.rs currently schedules snapshots without it."
#
# One bucket shared across environments (mirrors the ECR pattern in
# ecr.tf), partitioned by key prefix per environment:
#   s3://<bucket>/<environment>/payraider.db

resource "aws_s3_bucket" "db_backups" {
  bucket = "payraider-db-backups-${data.aws_caller_identity.current.account_id}"

  tags = {
    Name      = "PayRaider DB Backups"
    Purpose   = "Litestream continuous replication target for the SQLite database"
    Lifecycle = "Critical"
  }
}

resource "aws_s3_bucket_versioning" "db_backups" {
  bucket = aws_s3_bucket.db_backups.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "db_backups" {
  bucket = aws_s3_bucket.db_backups.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "db_backups" {
  bucket = aws_s3_bucket.db_backups.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Litestream's own generation/snapshot files accumulate; expire old
# generations well past any realistic point-in-time-recovery need.
resource "aws_s3_bucket_lifecycle_configuration" "db_backups" {
  bucket = aws_s3_bucket.db_backups.id

  rule {
    id     = "expire-old-litestream-generations"
    status = "Enabled"

    noncurrent_version_expiration {
      noncurrent_days = 90
    }
  }
}

output "db_backups_bucket_name" {
  description = "S3 bucket name for Litestream SQLite replication"
  value       = aws_s3_bucket.db_backups.id
}

output "db_backups_bucket_arn" {
  description = "S3 bucket ARN for Litestream SQLite replication"
  value       = aws_s3_bucket.db_backups.arn
}
