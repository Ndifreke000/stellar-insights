# Terraform Infrastructure as Code

This directory contains Terraform modules and configurations for provisioning and managing the PayRaider infrastructure on AWS.

## Overview

The PayRaider infrastructure is designed for **high availability, scalability, and disaster recovery** across three AWS availability zones (AZs). All components are deployed with Multi-AZ redundancy in production and staging environments.

### Architecture Components

- **Networking**: VPC with public/private subnets across 3 AZs, security groups, NAT gateways, route tables
- **Database**: SQLite, on an EFS volume mounted into the single (pinned
  `desired_count = 1`) backend task. There is no RDS instance -- see
  `docs/adr/0001-sqlite-vs-postgres.md` for why, and `docs/backup-system.md`
  for how the database is backed up (Litestream continuous S3 replication +
  periodic local snapshots) without one.
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
│   ├── s3.tf                          # S3 buckets (state, ALB logs)
│   ├── backups.tf                     # S3 bucket for Litestream SQLite replication
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
    ├── caching/                       # ElastiCache Redis
    │   ├── main.tf
    │   ├── variables.tf
    │   ├── outputs.tf
    │   └── versions.tf
    ├── compute/ecs/                   # ECS Fargate cluster, EFS volume, Litestream sidecar
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
# - S3 bucket: payraider-terraform-state-ACCOUNT_ID
# - DynamoDB table: terraform-locks
# - ECR repositories: payraider-backend
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
  -backend-config="bucket=payraider-terraform-state-$(aws sts get-caller-identity --query Account --output text)" \
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

# Output key values (ALB DNS, Redis endpoint, etc.)
terraform output

# Monitor deployment progress
aws ecs describe-services --cluster payraider-production --services payraider-service
```

## Environment-Specific Configurations

Only three environments exist under `terraform/environments/`: `dev`,
`staging`, `production`. (There is a `k8s/overlays/mainnet/` for the k8s
deployment path, but no corresponding Terraform environment -- if you're
looking for one, it hasn't been created yet.)

**Backend replicas are pinned to 1 in all three**, and auto-scaling is
disabled. SQLite permits exactly one writer and there is no shared storage
for multiple replicas to safely use the same database file -- see
`docs/adr/0001-sqlite-vs-postgres.md`.

### Development (`terraform/environments/dev/`)

- **Instance sizes**: Minimal (t3.small compute)
- **Replicas**: 1
- **Multi-AZ**: Disabled (single AZ, cost optimized)
- **Cost estimate**: ~$65/month

```bash
cd terraform/environments/dev
terraform init -backend-config="key=dev/terraform.tfstate"
terraform plan
terraform apply
```

### Staging (`terraform/environments/staging/`)

- **Instance sizes**: Medium (t3.medium compute)
- **Replicas**: 1
- **Multi-AZ**: Enabled (networking/caching; not applicable to the
  single-instance backend)
- **Cost estimate**: ~$96/month

### Production (`terraform/environments/production/`)

- **Instance sizes**: Medium (t3.medium compute)
- **Replicas**: 1
- **Multi-AZ**: Enabled (networking/caching; not applicable to the
  single-instance backend)
- **Cost estimate**: ~$142/month

## Common Operations

### Scaling Compute

**The backend cannot be horizontally scaled** in its current form --
`desired_count` is pinned to 1 in every environment's `module "compute"`
call, on purpose. Raising it without first giving the backend a real
multi-writer story would mean multiple tasks either fighting over one
`ReadWriteOnce`-mounted EFS file or (worse, if the volume config allowed it)
corrupting it. See `docs/adr/0001-sqlite-vs-postgres.md`, "Revisit this
decision when... horizontal scaling of the backend becomes a requirement."

Other compute resources (`container_cpu`, `container_memory`) can be scaled
vertically as normal via `terraform.tfvars`.

### Database Backup & Restore

There is no RDS instance and no `aws rds` command applies here. See
[`docs/backup-system.md`](../docs/backup-system.md) for the actual backup
story: a Litestream sidecar continuously replicating the SQLite file to S3
(with `litestream restore` commands), plus `backup.rs`'s periodic local
snapshots.

### Updating Images

Container images are pushed to ECR and deployed via CodeDeploy:

```bash
# Build and push image to ECR
docker build -t payraider-backend:TAG backend/
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin ECR_REPOSITORY_URL
docker tag payraider-backend:TAG ECR_REPOSITORY_URL/payraider-backend:TAG
docker push ECR_REPOSITORY_URL/payraider-backend:TAG

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
- **caching**: Redis cluster, auto-failover, backup snapshots
- **compute/ecs**: ECS task definitions, EFS volume for the SQLite database,
  Litestream sidecar, health checks (no auto-scaling -- see "Scaling
  Compute" above)
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
terraform state show module.compute.aws_efs_file_system.backend_data
terraform state mv old_name new_name
terraform state rm resource_name  # Only if removing from management
```

**IMPORTANT**: Never commit `terraform.tfvars` or local state files to git. State files contain sensitive data (database passwords, API keys).

## Disaster Recovery

All infrastructure is designed for disaster recovery:

1. **Database**: Litestream continuously replicates SQLite to S3 (point-in-time
   recovery), plus periodic local snapshots via `backup.rs`. See
   `docs/backup-system.md`. There is no automatic failover for the database
   itself -- it's a single EFS-backed volume behind a single task, by design
   (see ADR 0001).
2. **Multi-AZ failover** is automatic for Redis (and for networking/ALB)
3. **Infrastructure as Code** allows entire environment to be re-provisioned from git
4. **Blue-green deployments** allow instant rollback to previous version, though
   note the backend Deployment/ECS service currently briefly runs old+new
   tasks concurrently during cutover even at desired_count=1 -- a known,
   unresolved edge case against the single-writer constraint (see the
   staging/production terraform comments).

See [Disaster Recovery Plan](../docs/disaster-recovery.md) for detailed runbooks.

## Security Considerations

- **Encryption**: EFS encryption at rest, S3 backups bucket SSE, in-transit (SSL/TLS)
- **Network isolation**: Private subnets for cache/EFS, NACLs for ingress control
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
aws s3 ls s3://payraider-terraform-state-ACCOUNT_ID/
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
aws ecs describe-services --cluster payraider-production --services payraider-service

# View ECS task logs
aws logs tail /ecs/payraider-production --follow

# Check CodeDeploy deployment status
aws deploy get-deployment --deployment-id deployment-id
```

### Network Connectivity Issues

```bash
# Verify security groups
aws ec2 describe-security-groups --filters "Name=group-name,Values=payraider-*"

# Check route tables
aws ec2 describe-route-tables --filters "Name=vpc-id,Values=VPC_ID"

# Test the EFS mount is working (exec into the running task/pod)
ls -la /data/payraider.db
```

## Cost Optimization

The infrastructure is designed for cost-efficiency:

- **No RDS**: the SQLite/EFS/Litestream setup is a fraction of Multi-AZ RDS's
  cost (~$1/month EFS storage vs. ~$30-150/month RDS, see the cost_estimate
  outputs in each environment) -- a side effect of ADR 0001, not a deliberate
  cost optimization on its own
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
- **PayRaider Docs**: See [../docs/](../docs/)
- **Disaster Recovery**: [../docs/disaster-recovery.md](../docs/disaster-recovery.md)
- **Backup System**: [../docs/backup-system.md](../docs/backup-system.md)

## License

This Terraform configuration is part of the PayRaider project and follows the same license terms.
