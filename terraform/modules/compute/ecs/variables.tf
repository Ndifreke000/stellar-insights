variable "cluster_name" {
  description = "ECS cluster name"
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9-]{1,255}$", var.cluster_name))
    error_message = "Cluster name must contain only lowercase letters, numbers, and hyphens"
  }
}

variable "container_image" {
  description = "ECR image URI with tag (e.g., account.dkr.ecr.region.amazonaws.com/repo:tag)"
  type        = string

  validation {
    condition     = can(regex(".dkr.ecr.", var.container_image))
    error_message = "Container image must be an ECR image URI"
  }
}

variable "container_port" {
  description = "Container port (Rust backend default: 8080)"
  type        = number
  default     = 8080

  validation {
    condition     = var.container_port >= 1024 && var.container_port <= 65535
    error_message = "Container port must be between 1024 and 65535"
  }
}

variable "container_cpu" {
  description = "Task CPU units (256, 512, 1024, 2048, 4096)"
  type        = number
  default     = 512

  validation {
    condition     = contains([256, 512, 1024, 2048, 4096], var.container_cpu)
    error_message = "Container CPU must be one of: 256, 512, 1024, 2048, 4096"
  }
}

variable "container_memory" {
  description = "Task memory in MB (512, 1024, 2048, 3072, 4096)"
  type        = number
  default     = 1024

  validation {
    condition     = contains([512, 1024, 2048, 3072, 4096], var.container_memory)
    error_message = "Container memory must be one of: 512, 1024, 2048, 3072, 4096"
  }
}

variable "desired_count" {
  description = "Desired number of running tasks"
  type        = number
  default     = 2

  validation {
    condition     = var.desired_count >= 1 && var.desired_count <= 10
    error_message = "Desired count must be between 1 and 10"
  }
}

variable "min_size" {
  description = "Minimum number of EC2 instances (ASG)"
  type        = number
  default     = 1

  validation {
    condition     = var.min_size >= 1 && var.min_size <= 10
    error_message = "Min size must be between 1 and 10"
  }
}

variable "max_size" {
  description = "Maximum number of EC2 instances (ASG)"
  type        = number
  default     = 4

  validation {
    condition     = var.max_size >= var.min_size && var.max_size <= 20
    error_message = "Max size must be >= min_size and <= 20"
  }
}

variable "instance_type" {
  description = "EC2 instance type (t3.micro, t3.small, t3.medium, t3.large)"
  type        = string

  validation {
    condition     = can(regex("^t3\\.(micro|small|medium|large)$", var.instance_type))
    error_message = "Instance type must be a valid t3 type (e.g., t3.micro, t3.small)"
  }
}

variable "subnets" {
  description = "Private subnet IDs for ECS tasks"
  type        = list(string)

  validation {
    condition     = length(var.subnets) >= 1
    error_message = "At least one subnet must be provided"
  }
}

variable "security_groups" {
  description = "Security group IDs for ECS tasks"
  type        = list(string)

  validation {
    condition     = length(var.security_groups) >= 1
    error_message = "At least one security group must be provided"
  }
}

variable "target_group_arn" {
  description = "ALB target group ARN"
  type        = string

  validation {
    condition     = can(regex("arn:aws:elasticloadbalancing:", var.target_group_arn))
    error_message = "Target group ARN must be a valid ELB ARN"
  }
}

variable "vault_addr" {
  description = "Vault server address (passed to container)"
  type        = string

  validation {
    condition     = can(regex("^https://", var.vault_addr))
    error_message = "Vault address must be an HTTPS URL"
  }
}

variable "db_url" {
  description = "Database connection URL (postgresql://user:pass@host:5432/db)"
  type        = string
  sensitive   = true
}

variable "redis_url" {
  description = "Redis connection URL (redis://:auth@host:6379)"
  type        = string
  sensitive   = true
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

variable "log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 7
}

variable "health_check_path" {
  description = "Health check endpoint path"
  type        = string
  default     = "/health"
}

variable "health_check_interval" {
  description = "Health check interval in seconds"
  type        = number
  default     = 30
}

variable "health_check_timeout" {
  description = "Health check timeout in seconds"
  type        = number
  default     = 5
}

variable "health_check_healthy_threshold" {
  description = "Healthy check count threshold"
  type        = number
  default     = 2
}

variable "health_check_unhealthy_threshold" {
  description = "Unhealthy check count threshold"
  type        = number
  default     = 3
}

variable "enable_auto_scaling" {
  description = "Enable target tracking auto-scaling"
  type        = bool
  default     = true
}

variable "cpu_target_percentage" {
  description = "CPU target percentage for auto-scaling (70% default)"
  type        = number
  default     = 70

  validation {
    condition     = var.cpu_target_percentage >= 20 && var.cpu_target_percentage <= 90
    error_message = "CPU target must be between 20 and 90 percent"
  }
}

variable "enable_blue_green" {
  description = "Enable blue-green deployment with CodeDeploy"
  type        = bool
  default     = false
}
