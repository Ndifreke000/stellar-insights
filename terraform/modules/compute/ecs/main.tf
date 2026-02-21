# ============================================================================
# CloudWatch Log Group
# ============================================================================

resource "aws_cloudwatch_log_group" "ecs" {
  name              = "/ecs/${var.cluster_name}"
  retention_in_days = var.log_retention_days

  tags = {
    Name = "stellar-insights-ecs-logs-${var.environment}"
  }
}

# ============================================================================
# IAM Roles
# ============================================================================

# Task Execution Role (used by ECS agent to pull image and write logs)
resource "aws_iam_role" "ecs_task_execution_role" {
  name = "stellar-insights-ecs-task-execution-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ecs-tasks.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name = "stellar-insights-ecs-task-execution-${var.environment}"
  }
}

resource "aws_iam_role_policy_attachment" "ecs_task_execution_role_policy" {
  role       = aws_iam_role.ecs_task_execution_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

# Task Role (used by the application inside the container)
resource "aws_iam_role" "ecs_task_role" {
  name = "stellar-insights-ecs-task-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ecs-tasks.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name = "stellar-insights-ecs-task-${var.environment}"
  }
}

# Task role policy: allow reading from Vault and writing to S3 for artifacts
resource "aws_iam_role_policy" "ecs_task_policy" {
  name = "stellar-insights-ecs-task-policy"
  role = aws_iam_role.ecs_task_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "vault:ReadSecret",
          "vault:GetDatabaseCredentials"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:ListBucket"
        ]
        Resource = "*"
        Condition = {
          StringLike = {
            "s3:prefix" = "artifacts/*"
          }
        }
      }
    ]
  })
}

# EC2 Instance Profile Role
resource "aws_iam_role" "ecs_instance_role" {
  name = "stellar-insights-ecs-instance-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name = "stellar-insights-ecs-instance-${var.environment}"
  }
}

resource "aws_iam_role_policy_attachment" "ecs_instance_role_policy" {
  role       = aws_iam_role.ecs_instance_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonEC2ContainerServiceforEC2Role"
}

resource "aws_iam_instance_profile" "ecs" {
  name = "stellar-insights-ecs-instance-profile-${var.environment}"
  role = aws_iam_role.ecs_instance_role.name
}

# ============================================================================
# ECS Cluster
# ============================================================================

resource "aws_ecs_cluster" "main" {
  name = var.cluster_name

  setting {
    name  = "containerInsights"
    value = var.environment == "production" ? "enabled" : "disabled"
  }

  tags = {
    Name = "stellar-insights-ecs-cluster-${var.environment}"
  }
}

# ============================================================================
# EC2 Launch Template
# ============================================================================

data "aws_ami" "ecs_optimized" {
  most_recent = true
  owners      = ["amazon"]

  filter {
    name   = "name"
    values = ["amzn2-ami-ecs-hvm-*-x86_64-ebs"]
  }
}

resource "aws_launch_template" "ecs" {
  name_prefix   = "stellar-insights-ecs-"
  image_id      = data.aws_ami.ecs_optimized.id
  instance_type = var.instance_type

  iam_instance_profile {
    arn = aws_iam_instance_profile.ecs.arn
  }

  # ECS cluster initialization
  user_data = base64encode(<<-EOF
              #!/bin/bash
              echo "ECS_CLUSTER=${aws_ecs_cluster.main.name}" >> /etc/ecs/ecs.config
              echo "ECS_ENABLE_CONTAINER_METADATA=true" >> /etc/ecs/ecs.config
              echo "ECS_ENABLE_TASK_CPU_MEM_LIMIT=true" >> /etc/ecs/ecs.config
              echo "ECS_ENABLE_TASK_IAM_ROLE=true" >> /etc/ecs/ecs.config
              echo "ECS_ENABLE_TASK_IAM_ROLE_NETWORK_HOST=true" >> /etc/ecs/ecs.config
              EOF
  )

  monitoring {
    enabled = var.environment != "dev"
  }

  metadata_options {
    http_endpoint               = "enabled"
    http_tokens                 = "required"
    http_put_response_hop_limit = 1
  }

  monitoring {
    enabled = true
  }

  tag_specifications {
    resource_type = "instance"
    tags = {
      Name = "stellar-insights-ecs-${var.environment}"
    }
  }

  lifecycle {
    create_before_destroy = true
  }
}

# ============================================================================
# Auto Scaling Group
# ============================================================================

resource "aws_autoscaling_group" "ecs" {
  name                = "${var.cluster_name}-asg"
  vpc_zone_identifier = var.subnets
  min_size            = var.min_size
  max_size            = var.max_size
  desired_capacity    = var.desired_count
  health_check_type   = "ELB"
  health_check_grace_period = 300

  launch_template {
    id      = aws_launch_template.ecs.id
    version = "$Latest"
  }

  tag {
    key                 = "Name"
    value               = "stellar-insights-ecs-${var.environment}"
    propagate_at_launch = true
  }

  tag {
    key                 = "Cluster"
    value               = aws_ecs_cluster.main.name
    propagate_at_launch = true
  }

  lifecycle {
    create_before_destroy = true
  }
}

# ============================================================================
# ECS Task Definition
# ============================================================================

resource "aws_ecs_task_definition" "app" {
  family                   = "stellar-insights-${var.environment}"
  network_mode             = "awsvpc"
  requires_compatibilities = ["EC2"]
  cpu                      = var.container_cpu
  memory                   = var.container_memory
  execution_role_arn       = aws_iam_role.ecs_task_execution_role.arn
  task_role_arn            = aws_iam_role.ecs_task_role.arn

  container_definitions = jsonencode([
    {
      name      = "stellar-insights"
      image     = var.container_image
      essential = true
      portMappings = [
        {
          containerPort = var.container_port
          hostPort      = var.container_port
          protocol      = "tcp"
        }
      ]

      # Logging
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs.name
          "awslogs-region"        = data.aws_caller_identity.current.account
          "awslogs-stream-prefix" = "ecs"
        }
      }

      # Environment variables (non-sensitive)
      environment = [
        {
          name  = "ENVIRONMENT"
          value = var.environment
        },
        {
          name  = "VAULT_ADDR"
          value = var.vault_addr
        }
      ]

      # Secrets from Vault (via parameter store/secrets manager in production)
      secrets = [
        {
          name      = "DATABASE_URL"
          valueFrom = aws_secretsmanager_secret.database_url.arn
        },
        {
          name      = "REDIS_URL"
          valueFrom = aws_secretsmanager_secret.redis_url.arn
        }
      ]

      # Health check
      healthCheck = {
        command     = ["CMD-SHELL", "curl -f http://localhost:${var.container_port}${var.health_check_path} || exit 1"]
        interval    = var.health_check_interval
        timeout     = var.health_check_timeout
        retries     = 3
        startPeriod = 60
      }
    }
  ])

  tags = {
    Name = "stellar-insights-task-${var.environment}"
  }
}

# ============================================================================
# ECS Service
# ============================================================================

resource "aws_ecs_service" "app" {
  name            = "stellar-insights-service"
  cluster         = aws_ecs_cluster.main.id
  task_definition = aws_ecs_task_definition.app.arn
  desired_count   = var.desired_count
  launch_type     = "EC2"

  network_configuration {
    subnets          = var.subnets
    security_groups  = var.security_groups
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = var.target_group_arn
    container_name   = "stellar-insights"
    container_port   = var.container_port
  }

  dynamic "deployment_controller" {
    for_each = var.enable_blue_green ? [1] : []
    content {
      type = "CODE_DEPLOY"
    }
  }

  # Graceful shutdown: wait up to 30 seconds for container to stop
  depends_on = [
    aws_ecs_cluster.main,
    aws_ecs_task_definition.app
  ]

  tags = {
    Name = "stellar-insights-service-${var.environment}"
  }

  lifecycle {
    ignore_changes = [desired_count, task_definition, load_balancer]
  }
}

# ============================================================================
# Auto-Scaling
# ============================================================================

resource "aws_appautoscaling_target" "ecs_target" {
  count              = var.enable_auto_scaling ? 1 : 0
  max_capacity       = var.max_size
  min_capacity       = var.min_size
  resource_id        = "service/${aws_ecs_cluster.main.name}/${aws_ecs_service.app.name}"
  scalable_dimension = "ecs:service:DesiredCount"
  service_namespace  = "ecs"
}

resource "aws_appautoscaling_policy" "ecs_target_cpu" {
  count              = var.enable_auto_scaling ? 1 : 0
  name               = "stellar-insights-cpu-scaling-${var.environment}"
  policy_type        = "TargetTrackingScaling"
  resource_id        = aws_appautoscaling_target.ecs_target[0].resource_id
  scalable_dimension = aws_appautoscaling_target.ecs_target[0].scalable_dimension
  service_namespace  = aws_appautoscaling_target.ecs_target[0].service_namespace

  target_tracking_scaling_policy_configuration {
    predefined_metric_specification {
      predefined_metric_type = "ECSServiceAverageCPUUtilization"
    }
    target_value = var.cpu_target_percentage / 100.0
  }
}

# ============================================================================
# Secrets in AWS Secrets Manager
# ============================================================================

resource "aws_secretsmanager_secret" "database_url" {
  name                    = "stellar-insights/database-url-${var.environment}"
  recovery_window_in_days = 7

  tags = {
    Name = "stellar-insights-db-url-${var.environment}"
  }
}

resource "aws_secretsmanager_secret_version" "database_url" {
  secret_id       = aws_secretsmanager_secret.database_url.id
  secret_string   = var.db_url
  version_stages = ["AWSCURRENT"]
}

resource "aws_secretsmanager_secret" "redis_url" {
  name                    = "stellar-insights/redis-url-${var.environment}"
  recovery_window_in_days = 7

  tags = {
    Name = "stellar-insights-redis-url-${var.environment}"
  }
}

resource "aws_secretsmanager_secret_version" "redis_url" {
  secret_id       = aws_secretsmanager_secret.redis_url.id
  secret_string   = var.redis_url
  version_stages = ["AWSCURRENT"]
}

# Allow ECS task execution role to read secrets
resource "aws_secretsmanager_secret_policy" "database_url" {
  secret_arn = aws_secretsmanager_secret.database_url.arn

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "AllowECStaskExecutionRole"
        Effect = "Allow"
        Principal = {
          AWS = aws_iam_role.ecs_task_execution_role.arn
        }
        Action   = "secretsmanager:GetSecretValue"
        Resource = "*"
      }
    ]
  })
}

resource "aws_secretsmanager_secret_policy" "redis_url" {
  secret_arn = aws_secretsmanager_secret.redis_url.arn

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "AllowECStaskExecutionRole"
        Effect = "Allow"
        Principal = {
          AWS = aws_iam_role.ecs_task_execution_role.arn
        }
        Action   = "secretsmanager:GetSecretValue"
        Resource = "*"
      }
    ]
  })
}

# ============================================================================
# Data Sources
# ============================================================================

data "aws_caller_identity" "current" {}
