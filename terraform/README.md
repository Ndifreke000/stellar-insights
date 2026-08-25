# Terraform Infrastructure as Code

This directory contains Terraform modules and configurations for provisioning and managing the Stellar Insights infrastructure on AWS.

## Overview

The Stellar Insights infrastructure is designed for **high availability, scalability, and disaster recovery** across three AWS availability zones (AZs). All components are deployed with Multi-AZ redundancy in production and staging environments.

### Architecture Components

- **Networking**: VPC with public/private subnets across 3 AZs, security groups, NAT gateways, route tables
- **Database**: RDS PostgreSQL (Multi-AZ) with automated backups, enhanced monitoring, and encryption
- **Caching**: ElastiCache Redis cluster (Multi-AZ with automatic failover)
- **Compute**: ECS Fargate for containerized backend workloads with auto-scaling
- **Load Balancing**: Application Load Balancer (ALB) with HTTPS/TLS and blue-green deployment support
- **Deployment**: AWS CodeDeploy for automated blue-green deployments
- **Monitoring**: CloudWatch logs, metrics, dashboards, and alarms
- **Secrets Management**: Vault integration for credential management

## Directory Structure

```
terraform/
├── README.md                          # This file
├── global/                            # Shared infrastructure (all environments)
│   ├── versions.tf                    # Terraform version and provider constraints
│   ├── variables.tf                   # Global variables (AWS account, ECR repos)
│   ├── s3.tf                          # S3 buckets (state, ALB logs, backups)
│   ├── dynamodb.tf                    # DynamoDB for Terraform state locking
│   ├── ecr.tf                         # ECR repositories for container images
│   ├── iam.tf                         # IAM roles and policies
│   └── outputs.tf                     # Outputs (bucket names, ECR endpoints)
│
├── environments/                      # Environment-specific configurations
│   ├── dev/                           # Development environment
│   │   ├── main.tf                    # Root module configuration
│   │   ├── variables.tf               # Environment-specific variables
│   │   ├── terraform.tfvars           # Variable values (git-ignored secrets)
│   │   └── terraform.tfvars.example   # Example values template
│   ├── staging/                       # Staging environment
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   ├── terraform.tfvars
│   │   └── terraform.tfvars.example
│   ├── production/                    # Production environment
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   ├── terraform.tfvars
│   │   └── terraform.tfvars.example
│   └── mainnet/                       # Mainnet environment
│       ├── main.tf
│       ├── variables.tf
│       ├── terraform.tfvars
│       └── terraform.tfvars.example
│
└── modules/                           # Reusable Terraform modules
    ├── networking/                    # VPC, subnets, security groups, NAT
    │   ├── main.tf
    │   ├── variables.tf
    │   ├── outputs.tf
    │   ├── security_groups.tf
    │   ├── route_tables.tf
    │   ├── nat.tf
    │   └── versions.tf
    ├── database/                      # RDS PostgreSQL
    │   ├── main.tf
    │   ├── variables.tf
    │   ├── outputs.tf
    │   └── versions.tf
    ├── caching/                       # ElastiCache Redis
    │   ├── main.tf
    │   ├── variables.tf
    │   ├── outputs.tf
    │   └── versions.tf
    ├── compute/ecs/                   # ECS Fargate cluster
    │   ├── main.tf
    │   ├── variables.tf
    │   ├── outputs.tf
    │   └── versions.tf
    ├── load_balancing/                # ALB and target groups
    │   ├── main.tf
    │   ├── variables.tf
    │   ├── outputs.tf
    │   └── versions.tf
    ├── monitoring/                    # CloudWatch dashboards and alarms
    │   ├── main.tf
    │   ├── variables.tf
    │   ├── outputs.tf
    │   └── versions.tf
    ├── codedeploy/                    # CodeDeploy for blue-green deployments
    │   ├── main.tf
    │   ├── variables.tf
    │   ├── outputs.tf
    │   └── versions.tf
    └── vault/                         # Vault integration for secrets
        ├── main.tf
        ├── variables.tf
        ├── vault_cluster.tf
        ├── outputs.tf
        └── versions.tf
```

## Prerequisites

Before deploying infrastructure, ensure you have:

1. **AWS Account** with appropriate permissions (see IAM roles in `global/iam.tf`)
2. **AWS CLI** configured: `aws configure`
3. **Terraform** >= 1.5 installed
4. **Docker** for building and pushing container images to ECR
5. **Vault** access (HCP or self-hosted) for secrets management

## Getting Started

### 1. Initialize Terraform Backend

Terraform uses remote state stored in S3 with DynamoDB locking for team collaboration.

```bash
# First, deploy the global infrastructure (one-time, per AWS account)
cd terraform/global
terraform init
terraform plan
terraform apply

# The global module creates:
# - S3 bucket: stellar-insights-terraform-state-ACCOUNT_ID
# - DynamoDB table: terraform-locks
# - ECR repositories: stellar-insights-backend
# - IAM roles: terraform-executor, github-actions-iam-role
```

### 2. Deploy an Environment

```bash
# Enter environment directory (e.g., production)
cd terraform/environments/production

# Copy example variables and customize for your environment
cp terraform.tfvars.example terraform.tfvars
# IMPORTANT: Edit terraform.tfvars with your actual values (never commit this file)
# - aws_region: AWS region for deployment
# - vpc_cidr: VPC CIDR block (e.g., 10.2.0.0/16)
# - vault_addr: Vault server URL
# - alarm_email: Email for CloudWatch alarms

# Initialize backend
terraform init \
  -backend-config="bucket=stellar-insights-terraform-state-$(aws sts get-caller-identity --query Account --output text)" \
  -backend-config="key=production/terraform.tfstate" \
  -backend-config="region=us-east-1" \
  -backend-config="dynamodb_table=terraform-locks"

# Review planned changes
terraform plan -out=tfplan

# Apply changes (requires approval for production)
terraform apply tfplan
```

### 3. Verify Deployment

```bash
# Check infrastructure status
terraform show

# Output key values (ALB DNS, RDS endpoint, etc.)
terraform output

# Monitor deployment progress
aws ecs describe-services --cluster stellar-insights-production --services stellar-insights-service
```

## Environment-Specific Configurations

### Development (`terraform/environments/dev/`)

- **Instance sizes**: Minimal (t3.small compute, t3.micro database)
- **Replicas**: 1 (cost optimized)
- **Backup retention**: 7 days
- **Multi-AZ**: Disabled (single AZ, cost optimized)
- **Cost estimate**: ~$50/month

```bash
cd terraform/environments/dev
terraform init -backend-config="key=dev/terraform.tfstate"
terraform plan
terraform apply
```

### Staging (`terraform/environments/staging/`)

- **Instance sizes**: Medium (t3.medium compute, t3.medium database)
- **Replicas**: 2
- **Backup retention**: 14 days
- **Multi-AZ**: Enabled
- **Cost estimate**: ~$150/month

### Production (`terraform/environments/production/`)

- **Instance sizes**: Medium (t3.medium compute, t3.medium database)
- **Replicas**: 3
- **Backup retention**: 30 days
- **Multi-AZ**: Enabled, across 3 AZs
- **Auto-scaling**: 3-10 tasks based on CPU/memory
- **Cost estimate**: ~$330/month

### Mainnet (`terraform/environments/mainnet/`)

- **Instance sizes**: Large (t3.large compute, t3.medium database)
- **Replicas**: 3 per AZ
- **Backup retention**: 30 days
- **Multi-AZ**: Enabled, across 3 AZs
- **Cost estimate**: ~$400-500/month

## Common Operations

### Scaling Compute

```bash
# Update desired task count in terraform.tfvars
desired_count = 5

# Apply changes
terraform apply
```

### Database Backup & Restore

Database backups are automatically created per the RDS backup retention policy (30 days for production).

#### Manual Backup

```bash
# Create manual snapshot
aws rds create-db-snapshot \
  --db-instance-identifier stellar-insights-production \
  --db-snapshot-identifier stellar-insights-production-manual-$(date +%Y%m%d)
```

#### Restore from Backup

```bash
# List available snapshots
aws rds describe-db-snapshots --db-instance-identifier stellar-insights-production

# Restore to new database
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier stellar-insights-production-restored \
  --db-snapshot-identifier stellar-insights-production-manual-20240101
```

### Updating Images

Container images are pushed to ECR and deployed via CodeDeploy:

```bash
# Build and push image to ECR
docker build -t stellar-insights-backend:TAG backend/
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin ECR_REPOSITORY_URL
docker tag stellar-insights-backend:TAG ECR_REPOSITORY_URL/stellar-insights-backend:TAG
docker push ECR_REPOSITORY_URL/stellar-insights-backend:TAG

# CodeDeploy CI/CD workflow will automatically:
# 1. Register new ECS task definition
# 2. Create blue-green deployment
# 3. Route traffic to new deployment
# 4. Rollback on failure
```

### Monitoring & Alarms

CloudWatch dashboards and alarms are managed by the `monitoring` module:

```bash
# View available dashboards
aws cloudwatch list-dashboards

# Check alarm status
aws cloudwatch describe-alarms --query 'MetricAlarms[].{Name:AlarmName,State:StateValue}'

# Update alarm email (if using SNS)
terraform apply -var="alarm_email=new-email@example.com"
```

## Module Documentation

Each module has its own `README` (or is documented in the main.tf):

- **networking**: VPC topology, security groups, routing
- **database**: RDS configuration, backup retention, encryption
- **caching**: Redis cluster, auto-failover, backup snapshots
- **compute/ecs**: ECS task definitions, auto-scaling, health checks
- **load_balancing**: ALB listener rules, target groups, SSL/TLS
- **monitoring**: CloudWatch log groups, dashboards, alarm thresholds
- **codedeploy**: Blue-green deployment strategy, health checks, rollback
- **vault**: Secrets engine configuration, IAM auth, access policies

## State Management

Terraform state is stored in S3 with DynamoDB locking to prevent concurrent modifications:

```bash
# View state (read-only)
terraform state show

# Refresh state from AWS
terraform refresh

# Manual state operations (use with caution!)
terraform state list
terraform state show aws_rds_cluster_instance.primary
terraform state mv old_name new_name
terraform state rm resource_name  # Only if removing from management
```

**IMPORTANT**: Never commit `terraform.tfvars` or local state files to git. State files contain sensitive data (database passwords, API keys).

## Disaster Recovery

All infrastructure is designed for disaster recovery:

1. **Database backups** are retained for 30 days (production) with automated snapshots
2. **Multi-AZ failover** is automatic for RDS and Redis
3. **Infrastructure as Code** allows entire environment to be re-provisioned from git
4. **Blue-green deployments** allow instant rollback to previous version

See [Disaster Recovery Plan](../docs/disaster-recovery.md) for detailed runbooks.

## Security Considerations

- **Encryption**: RDS encryption at rest (KMS), in-transit (SSL/TLS)
- **Network isolation**: Private subnets for database/cache, NACLs for ingress control
- **Secrets management**: All credentials stored in Vault, not in Terraform
- **IAM policies**: Least-privilege roles for ECS tasks, CodeDeploy, monitoring
- **Audit logging**: VPC Flow Logs, CloudTrail for all AWS API calls

## CI/CD Integration

Terraform validation and formatting are automatically checked in CI:

```yaml
# .github/workflows/terraform-validate.yml
- Run terraform validate on all configurations
- Run terraform fmt to check formatting
- Fail if issues found
```

```bash
# Local validation before committing
terraform validate
terraform fmt -recursive terraform/
```

## Troubleshooting

### Backend Initialization Fails

```bash
# Ensure S3 bucket and DynamoDB table exist
aws s3 ls s3://stellar-insights-terraform-state-ACCOUNT_ID/
aws dynamodb describe-table --table-name terraform-locks

# If missing, run global module first
cd terraform/global && terraform apply
```

### Plan Shows Unexpected Changes

```bash
# Refresh state to match AWS reality
terraform refresh

# Re-run plan
terraform plan
```

### Deployment Stuck or Failing

```bash
# Check ECS service status
aws ecs describe-services --cluster stellar-insights-production --services stellar-insights-service

# View ECS task logs
aws logs tail /ecs/stellar-insights-production --follow

# Check CodeDeploy deployment status
aws deploy get-deployment --deployment-id deployment-id
```

### Network Connectivity Issues

```bash
# Verify security groups
aws ec2 describe-security-groups --filters "Name=group-name,Values=stellar-insights-*"

# Check route tables
aws ec2 describe-route-tables --filters "Name=vpc-id,Values=VPC_ID"

# Test database connectivity (from ECS task)
psql -h RDS_ENDPOINT -U postgres -d stellar_insights -c "SELECT 1;"
```

## Cost Optimization

The infrastructure is designed for cost-efficiency:

- **Fargate auto-scaling**: Scale down to 1 task during off-hours
- **RDS**: t3.medium for most workloads, but right-size based on workload
- **Single NAT**: Using one NAT gateway instead of per-AZ saves ~$30/month
- **Spot instances**: Can be enabled for non-critical environments (dev)

```bash
# Estimate costs
terraform plan | grep -E "^  ~|^  \+|^  -"

# Use AWS Cost Explorer for historical analysis
aws ce get-cost-and-usage --time-period Start=2024-01-01,End=2024-01-31 --granularity MONTHLY --metrics "BlendedCost"
```

## Contributing

- All module changes require `terraform fmt` and `terraform validate`
- Follow naming conventions: `${project}-${component}-${environment}`
- Document changes in commit messages with issue references
- Test changes in dev environment before staging/production
- Use `terraform plan` to review all changes before apply

## Support & Documentation

- **AWS Documentation**: https://docs.aws.amazon.com/
- **Terraform Registry**: https://registry.terraform.io/
- **Stellar Insights Docs**: See [../docs/](../docs/)
- **Disaster Recovery**: [../docs/disaster-recovery.md](../docs/disaster-recovery.md)
- **Backup System**: [../docs/backup-system.md](../docs/backup-system.md)

## License

This Terraform configuration is part of the Stellar Insights project and follows the same license terms.
