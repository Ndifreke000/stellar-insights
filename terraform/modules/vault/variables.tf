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
