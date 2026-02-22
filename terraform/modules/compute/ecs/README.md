# Compute Module (ECS)

Manages AWS ECS cluster, EC2 instances, and task definitions for Stellar Insights backend.

## Features

- ECS cluster with EC2 launch type
- Auto-scaling group for EC2 instances
- CloudWatch Log Group for container logs
- IAM roles for ECS task execution and EC2 instance profile
- CloudWatch monitoring and alarms
- Task definitions with environment variables and secrets from Vault
- Service configuration with load balancer integration
- Graceful shutdown handling (SIGTERM)
- Health check configuration

## Architecture

```
ALB → Target Group → ECS Service → ECS Tasks (EC2 instances)
                                      ↓
                         (pulls secrets from Vault)
```

## Usage

```hcl
module "compute" {
  source = "../../modules/compute/ecs"

  # Cluster configuration
  cluster_name        = "stellar-insights-${var.environment}"
  container_image     = "${aws_ecr_repository.backend.repository_url}:latest"
  
  # Capacity
  desired_count       = 2
  min_size           = 1
  max_size           = 4
  instance_type      = "t3.small"  # t3.micro for dev
  
  # Networking
  subnets             = module.networking.private_app_subnet_ids
  security_groups     = [module.networking.security_group_backend_id]
  target_group_arn    = module.load_balancing.target_group_arn
  
  # Secrets and configuration
  vault_addr          = var.vault_addr
  db_url              = "postgresql://user:pass@${module.database.rds_endpoint}/stellar_insights"
  redis_url           = "redis://:${module.caching.auth_token}@${module.caching.primary_endpoint_address}"
  
  environment         = var.environment
  project            = "stellar-insights"
}
```

## Inputs

| Name | Description | Type | Required |
|------|-------------|------|----------|
| cluster_name | ECS cluster name | `string` | Yes |
| container_image | ECR image URI with tag | `string` | Yes |
| container_port | Container port | `number` | No (default: `8080`) |
| desired_count | Desired number of tasks | `number` | No (default: `2`) |
| min_size | Minimum number of EC2 instances | `number` | No (default: `1`) |
| max_size | Maximum number of EC2 instances | `number` | No (default: `4`) |
| instance_type | EC2 instance type | `string` | Yes |
| subnets | Private subnet IDs for ECS tasks | `list(string)` | Yes |
| security_groups | Security group IDs for ECS tasks | `list(string)` | Yes |
| target_group_arn | ALB target group ARN | `string` | Yes |
| vault_addr | Vault server address | `string` | Yes |
| db_url | Database connection URL | `string` | Yes |
| redis_url | Redis connection URL | `string` | Yes |
| environment | Environment name | `string` | Yes |
| project | Project name | `string` | No (default: `stellar-insights`) |

## Outputs

| Name | Description |
|------|-------------|
| cluster_name | ECS cluster name |
| cluster_arn | ECS cluster ARN |
| service_name | ECS service name |
| service_arn | ECS service ARN |
| asg_name | Auto Scaling Group name |

## Cost Estimates

**Development:**
- EC2 (t3.micro): ~$7/month
- Data transfer: ~$2/month
- CloudWatch Logs: <$1/month
- Monthly: ~$10/month (in addition to infrastructure)

**Staging:**
- EC2 (t3.small × 2): ~$60/month
- Data transfer: ~$5/month
- CloudWatch Logs: ~$1/month
- Monthly: ~$66/month

**Production:**
- EC2 (t3.small × 3): ~$90/month
- Auto-scaling (up to 5): +$30/month
- Data transfer: ~$10/month
- CloudWatch Logs: ~$2/month
- Monthly: ~$132/month

## Task Definition

The task definition includes:
- Container: Rust Tokio server on port 8080
- Memory: 512MB (dev), 1024MB (staging/prod)
- CPU: 256 (dev), 512 (staging/prod)
- Log driver: awslogs (CloudWatch)
- Environment variables: VAULT_ADDR, DATABASE_URL, REDIS_URL, ENVIRONMENT
- Health check: GET /health every 30s

## Auto-Scaling

- Target tracking: 70% CPU utilization (production)
- Scale up: within 2 minutes
- Scale down: within 5 minutes
- Minimum: 1 instance (dev), 2 (staging), 3 (production)

## Notes

- Uses EC2 launch type (not Fargate) for cost optimization
- Graceful shutdown: sends SIGTERM to container (30s timeout)
- Health checks: ALB + ECS both monitor task health
- Logging: all container output → CloudWatch
- See [main.rs](../../backend/src/main.rs) for shutdown signal handling
