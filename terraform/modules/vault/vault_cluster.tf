locals {
  vault_node_count = var.ha_enabled ? var.cluster_size : 1

  vault_server_config = {
    storage_backend = var.storage_backend
    ha_enabled      = var.ha_enabled
    cluster_size    = local.vault_node_count
    listener = {
      address     = "0.0.0.0:8200"
      tls_disable = 0
    }
    raft = var.storage_backend == "raft" ? {
      path    = "/vault/data"
      node_id = "vault-${var.environment}"
    } : null
  }
}

resource "aws_security_group" "vault" {
  count = length(var.subnet_ids) > 0 ? 1 : 0

  name        = "${var.project}-vault-${var.environment}"
  description = "Vault HA cluster (${var.storage_backend})"
  vpc_id      = data.aws_subnet.vault[0].vpc_id

  ingress {
    description = "Vault API"
    from_port   = 8200
    to_port     = 8200
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/8"]
  }

  ingress {
    description = "Vault Raft cluster traffic"
    from_port   = 8201
    to_port     = 8201
    protocol    = "tcp"
    self        = true
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${var.project}-vault-${var.environment}"
  }
}

data "aws_subnet" "vault" {
  count = length(var.subnet_ids) > 0 ? 1 : 0

  id = var.subnet_ids[0]
}

resource "aws_ecs_cluster" "vault" {
  count = length(var.subnet_ids) > 0 ? 1 : 0

  name = "${var.project}-vault-${var.environment}"

  setting {
    name  = "containerInsights"
    value = "enabled"
  }

  tags = {
    Name = "${var.project}-vault-${var.environment}"
  }
}

resource "aws_ecs_service" "vault" {
  count = length(var.subnet_ids) > 0 ? 1 : 0

  name            = "${var.project}-vault-${var.environment}"
  cluster         = aws_ecs_cluster.vault[0].id
  task_definition = aws_ecs_task_definition.vault[0].arn
  desired_count   = local.vault_node_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = var.subnet_ids
    security_groups  = [aws_security_group.vault[0].id]
    assign_public_ip = false
  }

  deployment_minimum_healthy_percent = var.ha_enabled ? 67 : 100
  deployment_maximum_percent         = 200

  tags = {
    Name          = "${var.project}-vault-${var.environment}"
    ha_enabled    = tostring(var.ha_enabled)
    storage_backend = var.storage_backend
    cluster_size  = tostring(local.vault_node_count)
  }
}

resource "aws_ecs_task_definition" "vault" {
  count = length(var.subnet_ids) > 0 ? 1 : 0

  family                   = "${var.project}-vault-${var.environment}"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = "512"
  memory                   = "1024"
  execution_role_arn       = aws_iam_role.vault_task_execution[0].arn
  task_role_arn            = aws_iam_role.vault_task[0].arn

  container_definitions = jsonencode([
    {
      name      = "vault"
      image     = "hashicorp/vault:1.15"
      essential = true
      portMappings = [
        { containerPort = 8200, protocol = "tcp" },
        { containerPort = 8201, protocol = "tcp" },
      ]
      environment = [
        { name = "VAULT_ADDR", value = "https://127.0.0.1:8200" },
        { name = "VAULT_API_ADDR", value = "https://127.0.0.1:8200" },
        { name = "VAULT_CLUSTER_ADDR", value = "https://127.0.0.1:8201" },
        { name = "VAULT_LOCAL_CONFIG", value = jsonencode(local.vault_server_config) },
      ]
      mountPoints = [
        {
          sourceVolume  = "vault-data"
          containerPath = "/vault/data"
          readOnly      = false
        },
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          awslogs-group         = aws_cloudwatch_log_group.vault[0].name
          awslogs-region        = data.aws_region.current.name
          awslogs-stream-prefix = "vault"
        }
      }
    },
  ])

  volume {
    name = "vault-data"

    efs_volume_configuration {
      file_system_id = aws_efs_file_system.vault[0].id
      root_directory = "/"
    }
  }
}

data "aws_region" "current" {}

resource "aws_cloudwatch_log_group" "vault" {
  count = length(var.subnet_ids) > 0 ? 1 : 0

  name              = "/ecs/${var.project}/vault/${var.environment}"
  retention_in_days = 30
}

resource "aws_efs_file_system" "vault" {
  count = length(var.subnet_ids) > 0 && var.storage_backend == "raft" ? 1 : 0

  encrypted = true

  tags = {
    Name = "${var.project}-vault-raft-${var.environment}"
  }
}

resource "aws_iam_role" "vault_task_execution" {
  count = length(var.subnet_ids) > 0 ? 1 : 0

  name = "${var.project}-vault-task-exec-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "ecs-tasks.amazonaws.com" }
    }]
  })
}

resource "aws_iam_role_policy_attachment" "vault_task_execution" {
  count = length(var.subnet_ids) > 0 ? 1 : 0

  role       = aws_iam_role.vault_task_execution[0].name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

resource "aws_iam_role" "vault_task" {
  count = length(var.subnet_ids) > 0 ? 1 : 0

  name = "${var.project}-vault-task-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "ecs-tasks.amazonaws.com" }
    }]
  })
}
