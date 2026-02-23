variable "name" {
  description = "Name of the ALB"
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9-]{1,32}$", var.name))
    error_message = "ALB name must contain only lowercase letters, numbers, and hyphens (1-32 chars)"
  }
}

variable "internal" {
  description = "Whether ALB is internal"
  type        = bool
  default     = false
}

variable "load_balancer_type" {
  description = "Type of load balancer (application, network)"
  type        = string
  default     = "application"

  validation {
    condition     = contains(["application", "network"], var.load_balancer_type)
    error_message = "Load balancer type must be application or network"
  }
}

variable "subnets" {
  description = "Public subnet IDs for ALB"
  type        = list(string)

  validation {
    condition     = length(var.subnets) >= 2
    error_message = "At least 2 subnets must be provided for ALB"
  }
}

variable "security_groups" {
  description = "Security group IDs for ALB"
  type        = list(string)

  validation {
    condition     = length(var.security_groups) >= 1
    error_message = "At least one security group must be provided"
  }
}

variable "target_group_name" {
  description = "Name of the target group"
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9-]{1,32}$", var.target_group_name))
    error_message = "Target group name must contain only lowercase letters, numbers, and hyphens (1-32 chars)"
  }
}

variable "target_port" {
  description = "Target port (backend port)"
  type        = number
  default     = 8080

  validation {
    condition     = var.target_port >= 1 && var.target_port <= 65535
    error_message = "Target port must be between 1 and 65535"
  }
}

variable "target_type" {
  description = "Target type (instance, ip, lambda)"
  type        = string
  default     = "instance"

  validation {
    condition     = contains(["instance", "ip", "lambda"], var.target_type)
    error_message = "Target type must be instance, ip, or lambda"
  }
}

variable "health_check_path" {
  description = "Health check endpoint path"
  type        = string
  default     = "/health"

  validation {
    condition     = can(regex("^/", var.health_check_path))
    error_message = "Health check path must start with /"
  }
}

variable "health_check_interval" {
  description = "Health check interval in seconds"
  type        = number
  default     = 30

  validation {
    condition     = var.health_check_interval >= 5 && var.health_check_interval <= 300
    error_message = "Health check interval must be between 5 and 300 seconds"
  }
}

variable "health_check_timeout" {
  description = "Health check timeout in seconds"
  type        = number
  default     = 5

  validation {
    condition     = var.health_check_timeout >= 2 && var.health_check_timeout <= 120
    error_message = "Health check timeout must be between 2 and 120 seconds"
  }
}

variable "health_check_healthy_threshold" {
  description = "Healthy check count threshold"
  type        = number
  default     = 2

  validation {
    condition     = var.health_check_healthy_threshold >= 2 && var.health_check_healthy_threshold <= 10
    error_message = "Healthy threshold must be between 2 and 10"
  }
}

variable "health_check_unhealthy_threshold" {
  description = "Unhealthy check count threshold"
  type        = number
  default     = 3

  validation {
    condition     = var.health_check_unhealthy_threshold >= 2 && var.health_check_unhealthy_threshold <= 10
    error_message = "Unhealthy threshold must be between 2 and 10"
  }
}

variable "certificate_arn" {
  description = "ACM certificate ARN for HTTPS"
  type        = string

  validation {
    condition     = can(regex("arn:aws:acm:", var.certificate_arn))
    error_message = "Certificate ARN must be a valid ACM certificate ARN"
  }
}

variable "domain_name" {
  description = "Domain name for HTTPS"
  type        = string

  validation {
    condition     = can(regex("^([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\\.)+[a-z]{2,}$", var.domain_name))
    error_message = "Domain name must be a valid domain"
  }
}

variable "enable_logs" {
  description = "Enable ALB request logging to S3"
  type        = bool
  default     = true
}

variable "logs_bucket" {
  description = "S3 bucket name for ALB logs (required if enable_logs=true)"
  type        = string
  default     = ""

  validation {
    condition     = var.logs_bucket == "" || can(regex("^[a-z0-9-]{3,63}$", var.logs_bucket))
    error_message = "Bucket name must be a valid S3 bucket name"
  }
}

variable "enable_waf" {
  description = "Enable WAF integration"
  type        = bool
  default     = false
}

variable "waf_arn" {
  description = "WAF Web ACL ARN (required if enable_waf=true)"
  type        = string
  default     = ""

  validation {
    condition     = var.waf_arn == "" || can(regex("arn:aws:wafv2:", var.waf_arn))
    error_message = "WAF ARN must be a valid WAFv2 ARN"
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

variable "enable_blue_green" {
  description = "Enable blue-green deployment with CodeDeploy"
  type        = bool
  default     = false
}

variable "test_listener_port" {
  description = "Port for test traffic listener (blue-green pre-validation)"
  type        = number
  default     = 8443

  validation {
    condition     = var.test_listener_port >= 1 && var.test_listener_port <= 65535
    error_message = "Test listener port must be between 1 and 65535"
  }
}
