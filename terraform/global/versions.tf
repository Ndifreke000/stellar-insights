terraform {
  required_version = ">= 1.5"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }

  # NOTE: This module MUST be applied with local state first
  # Other modules will use remote S3 backend after this
  # Do NOT configure backend here
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "stellar-insights"
      Environment = var.environment
      ManagedBy   = "Terraform"
      CreatedAt   = timestamp()
    }
  }
}
