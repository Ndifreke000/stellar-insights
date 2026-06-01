variable "cache_subnet_group_name" {
  description = "ElastiCache subnet group name"
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9-]{1,255}$", var.cache_subnet_group_name))
    error_message = "Subnet group name must contain only lowercase letters, numbers, and hyphens"
  }
}

variable "cache_subnet_ids" {
  description = "List of cache subnet IDs"
  type        = list(string)

  validation {
    condition     = length(var.cache_subnet_ids) >= 1
    error_message = "At least one subnet ID must be provided"
  }
}

variable "security_group_ids" {
  description = "List of security group IDs"
  type        = list(string)

  validation {
    condition     = length(var.security_group_ids) > 0
    error_message = "At least one security group ID must be provided"
  }
}

variable "cluster_id" {
  description = "ElastiCache cluster identifier"
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9-]{1,50}$", var.cluster_id))
    error_message = "Cluster ID must contain only lowercase letters, numbers, and hyphens (1-50 chars)"
  }
}

variable "node_type" {
  description = "ElastiCache node type (e.g., cache.t3.micro, cache.t3.small)"
  type        = string

  validation {
    condition     = can(regex("^cache\\.t3\\.(micro|small|medium|large)$", var.node_type))
    error_message = "Node type must be a valid t3 cache type (e.g., cache.t3.micro, cache.t3.small)"
  }
}

variable "num_cache_nodes" {
  description = "Number of cache nodes (1 for dev, 1-3 for staging/prod)"
  type        = number
  default     = 1

  validation {
    condition     = var.num_cache_nodes >= 1 && var.num_cache_nodes <= 3
    error_message = "Number of cache nodes must be between 1 and 3"
  }
}

variable "engine_version" {
  description = "Redis engine version"
  type        = string
  default     = "7.0"

  validation {
    condition     = can(regex("^7\\.[0-9]+$", var.engine_version))
    error_message = "Engine version must be Redis 7.x (e.g., 7.0, 7.1)"
  }
}

variable "parameter_group_family" {
  description = "Parameter group family"
  type        = string
  default     = "redis7"

  validation {
    condition     = can(regex("^redis[0-9]$", var.parameter_group_family))
    error_message = "Parameter group family must be redis7 or similar"
  }
}

variable "automatic_failover_enabled" {
  description = "Enable automatic failover (Multi-AZ)"
  type        = bool
  default     = false
}

variable "snapshot_retention_limit" {
  description = "Number of days to retain snapshots (0-35)"
  type        = number
  default     = 0

  validation {
    condition     = var.snapshot_retention_limit >= 0 && var.snapshot_retention_limit <= 35
    error_message = "Snapshot retention must be between 0 and 35 days"
  }
}

variable "snapshot_window" {
  description = "Preferred snapshot window (HH:MM-HH:MM UTC)"
  type        = string
  default     = "03:00-04:00"

  validation {
    condition     = can(regex("^[0-2][0-9]:[0-5][0-9]-[0-2][0-9]:[0-5][0-9]$", var.snapshot_window))
    error_message = "Snapshot window must be in format HH:MM-HH:MM (UTC)"
  }
}

variable "auto_minor_version_upgrade" {
  description = "Enable automatic minor version upgrades"
  type        = bool
  default     = true
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
