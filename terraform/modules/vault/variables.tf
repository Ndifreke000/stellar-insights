variable "vault_addr" {
  description = "Vault server address (HCP endpoint)"
  type        = string

  validation {
    condition     = can(regex("^https://", var.vault_addr))
    error_message = "Vault address must be an HTTPS URL"
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

variable "github_repo_owner" {
  description = "GitHub repository owner"
  type        = string
  default     = "Ndifreke000"
}

variable "github_repo_name" {
  description = "GitHub repository name"
  type        = string
  default     = "stellar-insights"
}

variable "subnet_ids" {
  description = "Private subnet IDs for Vault cluster nodes"
  type        = list(string)
  default     = []
}

variable "cluster_size" {
  description = "Number of Vault nodes in the HA cluster"
  type        = number
  default     = 3

  validation {
    condition     = var.cluster_size >= 1 && var.cluster_size <= 5
    error_message = "cluster_size must be between 1 and 5"
  }
}

variable "ha_enabled" {
  description = "Enable Vault high availability mode"
  type        = bool
  default     = true
}

variable "storage_backend" {
  description = "Vault storage backend (raft for HA)"
  type        = string
  default     = "raft"

  validation {
    condition     = contains(["raft", "consul"], var.storage_backend)
    error_message = "storage_backend must be raft or consul"
  }
}
