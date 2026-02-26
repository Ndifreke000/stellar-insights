# IAM roles and policies for Terraform execution
# These roles are used by CI/CD (GitHub Actions) and local Terraform runs

# Role for Terraform execution (CI/CD)
resource "aws_iam_role" "terraform_executor" {
  name               = "terraform-executor"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
      },
      {
        Action = "sts:AssumeRoleWithWebIdentity"
        Effect = "Allow"
        Principal = {
          Federated = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:oidc-provider/token.actions.githubusercontent.com"
        }
        Condition = {
          StringLike = {
            "token.actions.githubusercontent.com:sub" = "repo:Ndifreke000/stellar-insights:*"
          }
        }
      }
    ]
  })

  tags = {
    Name        = "Terraform Executor Role"
    Purpose     = "Allow Terraform to provision AWS resources"
  }
}

# Attach policy for state management
resource "aws_iam_role_policy" "terraform_state" {
  name   = "terraform-state-access"
  role   = aws_iam_role.terraform_executor.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "S3StateAccess"
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:DeleteObject",
          "s3:ListBucket"
        ]
        Resource = [
          aws_s3_bucket.terraform_state.arn,
          "${aws_s3_bucket.terraform_state.arn}/*"
        ]
      },
      {
        Sid    = "DynamoDBLocking"
        Effect = "Allow"
        Action = [
          "dynamodb:GetItem",
          "dynamodb:PutItem",
          "dynamodb:DeleteItem",
          "dynamodb:Query"
        ]
        Resource = aws_dynamodb_table.terraform_locks.arn
      }
    ]
  })
}

# Attach policy for infrastructure provisioning
# This is scoped to common infrastructure resources
resource "aws_iam_role_policy" "terraform_infrastructure" {
  name   = "terraform-infrastructure"
  role   = aws_iam_role.terraform_executor.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "EC2"
        Effect = "Allow"
        Action = [
          "ec2:*"
        ]
        Resource = "*"
      },
      {
        Sid    = "RDS"
        Effect = "Allow"
        Action = [
          "rds:*"
        ]
        Resource = "*"
      },
      {
        Sid    = "ElastiCache"
        Effect = "Allow"
        Action = [
          "elasticache:*"
        ]
        Resource = "*"
      },
      {
        Sid    = "ECS"
        Effect = "Allow"
        Action = [
          "ecs:*",
          "ec2:*"
        ]
        Resource = "*"
      },
      {
        Sid    = "ALB"
        Effect = "Allow"
        Action = [
          "elasticloadbalancing:*"
        ]
        Resource = "*"
      },
      {
        Sid    = "ECR"
        Effect = "Allow"
        Action = [
          "ecr:*"
        ]
        Resource = "*"
      },
      {
        Sid    = "IAM"
        Effect = "Allow"
        Action = [
          "iam:CreateRole",
          "iam:DeleteRole",
          "iam:GetRole",
          "iam:PutRolePolicy",
          "iam:DeleteRolePolicy",
          "iam:AttachRolePolicy",
          "iam:DetachRolePolicy",
          "iam:CreateInstanceProfile",
          "iam:DeleteInstanceProfile",
          "iam:AddRoleToInstanceProfile",
          "iam:RemoveRoleFromInstanceProfile",
          "iam:PassRole",
          "iam:ListRolePolicies"
        ]
        Resource = "*"
      },
      {
        Sid    = "KMS"
        Effect = "Allow"
        Action = [
          "kms:CreateKey",
          "kms:CreateAlias",
          "kms:DeleteAlias",
          "kms:DescribeKey",
          "kms:GetKeyPolicy",
          "kms:PutKeyPolicy",
          "kms:ListAliases"
        ]
        Resource = "*"
      },
      {
        Sid    = "CloudWatch"
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents",
          "logs:DeleteLogGroup",
          "cloudwatch:PutMetricAlarm",
          "cloudwatch:DeleteAlarms"
        ]
        Resource = "*"
      },
      {
        Sid    = "Route53"
        Effect = "Allow"
        Action = [
          "route53:*"
        ]
        Resource = "*"
      },
      {
        Sid    = "ACM"
        Effect = "Allow"
        Action = [
          "acm:RequestCertificate",
          "acm:DescribeCertificate",
          "acm:ListCertificates",
          "acm:DeleteCertificate"
        ]
        Resource = "*"
      },
      {
        Sid    = "CodeDeploy"
        Effect = "Allow"
        Action = [
          "codedeploy:*"
        ]
        Resource = "*"
      }
    ]
  })
}

output "terraform_executor_role_arn" {
  description = "ARN of Terraform executor role"
  value       = aws_iam_role.terraform_executor.arn
}

output "terraform_executor_role_name" {
  description = "Name of Terraform executor role"
  value       = aws_iam_role.terraform_executor.name
}
