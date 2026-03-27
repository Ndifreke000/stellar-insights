# VPC and core networking infrastructure

# Get available AZs in the region
data "aws_availability_zones" "available" {
  state = "available"
}

# Determine which AZs to use (all available or specified)
locals {
  azs = length(var.availability_zones) > 0 ? var.availability_zones : slice(data.aws_availability_zones.available.names, 0, var.environment == "production" ? 3 : 2)
  
  # Resource naming convention
  vpc_name = "${var.project_name}-${var.environment}-vpc"
  
  # Tags for all resources
  common_tags = merge(
    {
      Environment = var.environment
      Project     = var.project_name
    },
    var.tags
  )
}

# VPC
resource "aws_vpc" "main" {
  cidr_block           = var.vpc_cidr
  enable_dns_hostnames = var.enable_dns_hostnames
  enable_dns_support   = var.enable_dns_support

  tags = merge(
    local.common_tags,
    {
      Name = local.vpc_name
    }
  )
}

# Internet Gateway (for public subnets)
resource "aws_internet_gateway" "main" {
  vpc_id = aws_vpc.main.id

  tags = merge(
    local.common_tags,
    {
      Name = "${var.project_name}-${var.environment}-igw"
    }
  )
}

# Public Subnets (for ALB/NAT)
resource "aws_subnet" "public" {
  count                   = length(local.azs)
  vpc_id                  = aws_vpc.main.id
  cidr_block              = var.public_subnet_cidrs[count.index]
  availability_zone       = local.azs[count.index]
  map_public_ip_on_launch = true

  tags = merge(
    local.common_tags,
    {
      Name = "${var.project_name}-${var.environment}-public-${local.azs[count.index]}"
      Tier = "public"
    }
  )
}

# Private App Subnets (for ECS)
resource "aws_subnet" "private_app" {
  count              = length(local.azs)
  vpc_id             = aws_vpc.main.id
  cidr_block         = var.private_app_subnet_cidrs[count.index]
  availability_zone  = local.azs[count.index]

  tags = merge(
    local.common_tags,
    {
      Name = "${var.project_name}-${var.environment}-private-app-${local.azs[count.index]}"
      Tier = "private-app"
    }
  )
}

# Private Database Subnets (for RDS, ElastiCache)
resource "aws_subnet" "private_db" {
  count              = length(local.azs)
  vpc_id             = aws_vpc.main.id
  cidr_block         = var.private_db_subnet_cidrs[count.index]
  availability_zone  = local.azs[count.index]

  tags = merge(
    local.common_tags,
    {
      Name = "${var.project_name}-${var.environment}-private-db-${local.azs[count.index]}"
      Tier = "private-db"
    }
  )
}

# VPC Flow Logs (for production monitoring)
resource "aws_flow_log" "vpc" {
  count                   = var.enable_vpc_flow_logs ? 1 : 0
  iam_role_arn            = aws_iam_role.vpc_flow_logs[0].arn
  log_destination         = aws_cloudwatch_log_group.vpc_flow_logs[0].arn
  traffic_type            = "ALL"
  vpc_id                  = aws_vpc.main.id

  tags = merge(
    local.common_tags,
    {
      Name = "${var.project_name}-${var.environment}-flow-logs"
    }
  )
}

# CloudWatch Log Group for VPC Flow Logs
resource "aws_cloudwatch_log_group" "vpc_flow_logs" {
  count             = var.enable_vpc_flow_logs ? 1 : 0
  name              = "/aws/vpc/flow-logs/${var.project_name}-${var.environment}"
  retention_in_days = var.vpc_flow_log_retention_days

  tags = local.common_tags
}

# IAM role for VPC Flow Logs
resource "aws_iam_role" "vpc_flow_logs" {
  count = var.enable_vpc_flow_logs ? 1 : 0
  name  = "${var.project_name}-${var.environment}-vpc-flow-logs"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "vpc-flow-logs.amazonaws.com"
        }
      }
    ]
  })
}

resource "aws_iam_role_policy" "vpc_flow_logs" {
  count = var.enable_vpc_flow_logs ? 1 : 0
  name  = "${var.project_name}-${var.environment}-vpc-flow-logs"
  role  = aws_iam_role.vpc_flow_logs[0].id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents",
          "logs:DescribeLogGroups",
          "logs:DescribeLogStreams"
        ]
        Effect   = "Allow"
        Resource = "${aws_cloudwatch_log_group.vpc_flow_logs[0].arn}:*"
      }
    ]
  })
}
