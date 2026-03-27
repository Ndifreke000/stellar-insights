# ECR (Elastic Container Registry) repositories for Docker images
# Stores: backend, frontend Docker images for deployment

resource "aws_ecr_repository" "backend" {
  name                 = "stellar-insights-backend"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }

  encryption_configuration {
    encryption_type = "AES256"
  }

  tags = {
    Name        = "Backend Docker Repository"
    Service     = "backend"
    Description = "Rust backend service image"
  }
}

resource "aws_ecr_repository" "frontend" {
  name                 = "stellar-insights-frontend"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }

  encryption_configuration {
    encryption_type = "AES256"
  }

  tags = {
    Name        = "Frontend Docker Repository"
    Service     = "frontend"
    Description = "Next.js frontend application image"
  }
}

# ECR lifecycle policy: keep only N images (auto-delete old versions)
resource "aws_ecr_lifecycle_policy" "backend" {
  repository = aws_ecr_repository.backend.name

  policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last N images"
        selection = {
          tagStatus             = "tagged"
          tagPrefixList         = ["v"]
          countType             = "imageCountMoreThan"
          countNumber           = var.ecr_image_retention_count
        }
        action = {
          type = "expire"
        }
      },
      {
        rulePriority = 2
        description  = "Delete untagged images after 7 days"
        selection = {
          tagStatus     = "untagged"
          countType     = "sinceImagePushed"
          countUnit     = "days"
          countNumber   = 7
        }
        action = {
          type = "expire"
        }
      }
    ]
  })
}

resource "aws_ecr_lifecycle_policy" "frontend" {
  repository = aws_ecr_repository.frontend.name

  policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last N images"
        selection = {
          tagStatus             = "tagged"
          tagPrefixList         = ["v"]
          countType             = "imageCountMoreThan"
          countNumber           = var.ecr_image_retention_count
        }
        action = {
          type = "expire"
        }
      },
      {
        rulePriority = 2
        description  = "Delete untagged images after 7 days"
        selection = {
          tagStatus     = "untagged"
          countType     = "sinceImagePushed"
          countUnit     = "days"
          countNumber   = 7
        }
        action = {
          type = "expire"
        }
      }
    ]
  })
}

output "ecr_backend_repository_url" {
  description = "ECR URL for backend Docker image"
  value       = aws_ecr_repository.backend.repository_url
}

output "ecr_backend_repository_arn" {
  description = "ARN of backend ECR repository"
  value       = aws_ecr_repository.backend.arn
}

output "ecr_frontend_repository_url" {
  description = "ECR URL for frontend Docker image"
  value       = aws_ecr_repository.frontend.repository_url
}

output "ecr_frontend_repository_arn" {
  description = "ARN of frontend ECR repository"
  value       = aws_ecr_repository.frontend.arn
}
