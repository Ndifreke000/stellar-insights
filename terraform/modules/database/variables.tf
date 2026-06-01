variable "db_subnet_group_name" {
  description = "RDS DB subnet group name for Multi-AZ"
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9-]{1,255}$", var.db_subnet_group_name))
    error_message = "Subnet group name must contain only lowercase letters, numbers, and hyphens (1-255 chars)"
  }
}

variable "vpc_security_group_ids" {
  description = "List of security group IDs to attach to RDS instance"
  type        = list(string)

  validation {
    condition     = length(var.vpc_security_group_ids) > 0
    error_message = "At least one security group ID must be provided"
  }
}

variable "db_subnet_ids" {
  description = "List of database subnet IDs for Multi-AZ deployment"
  type        = list(string)

  validation {
    condition     = length(var.db_subnet_ids) >= 2
    error_message = "At least 2 subnet IDs must be provided (across different AZs)"
  }
}

variable "identifier" {
  description = "RDS instance resource identifier (must be unique, lowercase, alphanumeric and hyphens only)"
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9-]{1,63}$", var.identifier))
    error_message = "Identifier must contain only lowercase letters, numbers, and hyphens (1-63 chars)"
  }
}

variable "instance_class" {
  description = "RDS instance type (e.g., db.t3.micro, db.t3.small, db.t3.medium)"
  type        = string

  validation {
    condition     = can(regex("^db\\.t3\\.(micro|small|medium|large|xlarge|2xlarge)$", var.instance_class))
    error_message = "Instance class must be a valid t3 type (e.g., db.t3.micro, db.t3.small)"
  }
}

variable "allocated_storage" {
  description = "Allocated storage in GB (20-65536 for GP3)"
  type        = number

  validation {
    condition     = var.allocated_storage >= 20 && var.allocated_storage <= 65536
    error_message = "Allocated storage must be between 20 and 65536 GB"
  }
}

variable "storage_type" {
  description = "Storage type (gp3, gp2, io1, io2)"
  type        = string
  default     = "gp3"

  validation {
    condition     = contains(["gp3", "gp2", "io1", "io2"], var.storage_type)
    error_message = "Storage type must be one of: gp3, gp2, io1, io2"
  }
}

variable "engine_version" {
  description = "PostgreSQL engine version (14.x preferred for Stellar Insights)"
  type        = string
  default     = "14.8"

  validation {
    condition     = can(regex("^14\\.[0-9]+$", var.engine_version))
    error_message = "Engine version must be a valid PostgreSQL 14.x version (e.g., 14.8)"
  }
}

variable "db_name" {
  description = "Name of the initial database to create"
  type        = string
  default     = "stellar_insights"

  validation {
    condition     = can(regex("^[a-z][a-z0-9_]*$", var.db_name))
    error_message = "Database name must start with a letter and contain only lowercase letters, numbers, and underscores"
  }
}

variable "username" {
  description = "Master database username"
  type        = string
  default     = "postgres"

  validation {
    condition     = can(regex("^[a-z][a-z0-9_]*$", var.username))
    error_message = "Username must start with a letter and contain only lowercase letters, numbers, and underscores"
  }
}

variable "password" {
  description = "Master database password (strongly prefer Vault over plaintext)"
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.password) >= 8
    error_message = "Password must be at least 8 characters long"
  }
}

variable "multi_az" {
  description = "Enable Multi-AZ RDS deployment for high availability"
  type        = bool
  default     = false
}

variable "backup_retention_period" {
  description = "Number of days to retain automated backups (1-35, default 7)"
  type        = number
  default     = 7

  validation {
    condition     = var.backup_retention_period >= 1 && var.backup_retention_period <= 35
    error_message = "Backup retention must be between 1 and 35 days"
  }
}

variable "backup_window" {
  description = "Preferred backup window (HH:MM-HH:MM UTC)"
  type        = string
  default     = "03:00-04:00"

  validation {
    condition     = can(regex("^[0-2][0-9]:[0-5][0-9]-[0-2][0-9]:[0-5][0-9]$", var.backup_window))
    error_message = "Backup window must be in format HH:MM-HH:MM (UTC)"
  }
}

variable "skip_final_snapshot" {
  description = "Skip final snapshot on database destruction (use with caution!)"
  type        = bool
  default     = false
}

variable "enable_cloudwatch_logs_exports" {
  description = "List of log types to export to CloudWatch (e.g., postgresql)"
  type        = list(string)
  default     = []

  validation {
    condition     = alltrue([for log in var.enable_cloudwatch_logs_exports : contains(["postgresql"], log)])
    error_message = "Only 'postgresql' is valid for exports"
  }
}

variable "enable_enhanced_monitoring" {
  description = "Enable RDS Enhanced Monitoring (additional cost)"
  type        = bool
  default     = false
}

variable "monitoring_interval" {
  description = "Enhanced monitoring granularity in seconds (60 or 0 to disable)"
  type        = number
  default     = 60

  validation {
    condition     = contains([0, 60], var.monitoring_interval)
    error_message = "Monitoring interval must be 0 (disabled) or 60 seconds"
  }
}

variable "auto_minor_version_upgrade" {
  description = "Enable automatic minor version upgrades"
  type        = bool
  default     = true
}

variable "iops" {
  description = "IOPS for GP3 storage (3000-16000, only for GP3)"
  type        = number
  default     = 3000

  validation {
    condition     = var.iops >= 3000 && var.iops <= 16000
    error_message = "IOPS must be between 3000 and 16000 for GP3 storage"
  }
}

variable "storage_throughput" {
  description = "Storage throughput for GP3 (125-1000 MB/s, only for GP3)"
  type        = number
  default     = 125

  validation {
    condition     = var.storage_throughput >= 125 && var.storage_throughput <= 1000
    error_message = "Storage throughput must be between 125 and 1000 MB/s"
  }
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
