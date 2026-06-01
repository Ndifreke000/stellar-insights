# Terraform Infrastructure as Code for Stellar Insights

Complete Infrastructure-as-Code setup for Stellar Insights using Terraform 1.5+ and AWS.

## Overview

This Terraform configuration deploys a complete, production-ready infrastructure for Stellar Insights including:

- **Networking**: VPC, subnets, NAT gateways, route tables, security groups
- **Database**: RDS PostgreSQL (staging/production only)
- **Caching**: ElastiCache Redis cluster
- **Compute**: ECS cluster with auto-scaling
- **Load Balancing**: Application Load Balancer with HTTPS
- **Secrets Management**: Vault integration via HCP Cloud
- **Monitoring**: CloudWatch logs, metrics, alarms, dashboards

## Quick Start

### Prerequisites

- Terraform 1.5+
- AWS CLI v2 configured with credentials
- AWS Account with appropriate permissions
- Docker (for building container images)
- Git (for version control)

### Step 1: Bootstrap (One-time)

Initialize the global Terraform state backend:

```bash
# Make scripts executable
chmod +x terraform/scripts/*.sh

# Bootstrap S3, DynamoDB, and ECR
./terraform/scripts/bootstrap.sh us-east-1
```

This creates:
- S3 bucket for Terraform state
- DynamoDB table for state locks
- ECR repositories for container images

### Step 2: Deploy Dev Environment

```bash
# Initialize remote state for dev
./terraform/scripts/init-state.sh us-east-1 dev

# Plan infrastructure
./terraform/scripts/plan.sh dev

# Apply configuration
./terraform/scripts/apply.sh dev
```

### Step 3: Deploy Staging/Production

```bash
# Staging
./terraform/scripts/init-state.sh us-east-1 staging
./terraform/scripts/plan.sh staging
./terraform/scripts/apply.sh staging

# Production (requires additional confirmation)
./terraform/scripts/init-state.sh us-east-1 production
./terraform/scripts/plan.sh production
./terraform/scripts/apply.sh production
```

## Directory Structure

```
terraform/
├── global/                          # Global resources (state, ECR) - apply first!
│   ├── README.md                   # Bootstrap instructions
│   ├── versions.tf                 # Provider requirements
│   ├── variables.tf                # Input variables
│   ├── s3.tf                       # S3 state bucket
│   ├── dynamodb.tf                 # DynamoDB locks table
│   ├── ecr.tf                      # ECR repositories
│   ├── iam.tf                      # IAM roles (GitHub OIDC)
│   └── outputs.tf                  # Output values
│
├── modules/                        # Reusable Terraform modules
│   ├── networking/                 # VPC, subnets, routing, security groups
│   ├── database/                   # RDS PostgreSQL
│   ├── caching/                    # ElastiCache Redis
│   ├── compute/ecs/                # ECS cluster, task definitions, auto-scaling
│   ├── load_balancing/             # ALB with HTTPS
│   ├── vault/                      # Vault integration (reference)
│   └── monitoring/                 # CloudWatch logs, alarms, dashboards
│
├── environments/                   # Environment-specific configurations
│   ├── dev/                        # Development (minimal cost)
│   │   ├── main.tf                # Module composition
│   │   ├── variables.tf           # Dev-specific variables
│   │   └── terraform.tfvars       # Default values
│   ├── staging/                    # Staging (testing & QA)
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   └── terraform.tfvars
│   └── production/                 # Production (HA & backups)
│       ├── main.tf
│       ├── variables.tf
│       └── terraform.tfvars
│
└── scripts/                        # Helper scripts
    ├── bootstrap.sh               # Create global state backend
    ├── init-state.sh              # Initialize environment state
    ├── plan.sh                    # Run terraform plan
    ├── apply.sh                   # Run terraform apply
    └── destroy.sh                 # Destroy infrastructure (DANGEROUS!)
```

## Module Reference

### networking/

Manages VPC, subnets, routing, and security groups.

**Inputs:**
- `vpc_cidr`: VPC CIDR block (e.g., "10.0.0.0/16")
- `environment`: "dev", "staging", or "production"
- `enable_nat_per_az`: Single NAT (false) or per-AZ (true)
- `enable_vpc_flow_logs`: Enable VPC Flow Logs (cost impact)
- `azs`: Number of availability zones (2 or 3)

**Outputs:**
- `vpc_id`: VPC identifier
- `public_subnet_ids`: ALB subnets
- `private_app_subnet_ids`: ECS subnets
- `private_db_subnet_ids`: RDS/Redis subnets
- `security_group_*_id`: Security group IDs

**Example:**
```hcl
module "networking" {
  source = "../../modules/networking"
  
  vpc_cidr = "10.0.0.0/16"
  environment = "dev"
  enable_nat_per_az = false
  enable_vpc_flow_logs = false
  azs = 2
}
```

### database/

Manages RDS PostgreSQL instance with backups and monitoring.

**Inputs:**
- `identifier`: RDS instance name
- `instance_class`: "db.t3.micro", "db.t3.small", etc.
- `allocated_storage`: Storage in GB (20-65536)
- `multi_az`: Enable Multi-AZ failover (false for dev/staging, true for production)
- `backup_retention_period`: Days to keep backups (1-35)
- `enable_cloudwatch_logs_exports`: Export logs to CloudWatch

**Outputs:**
- `rds_endpoint`: Database connection string (host:port)
- `rds_address`: Hostname only
- `rds_port`: Port (5432)

**Example:**
```hcl
module "database" {
  source = "../../modules/database"
  
  identifier = "stellar-insights-dev"
  instance_class = "db.t3.micro"
  allocated_storage = 20
  multi_az = false
  backup_retention_period = 7
}
```

### caching/

Manages ElastiCache Redis cluster.

**Inputs:**
- `cluster_id`: Redis cluster name
- `node_type`: "cache.t3.micro", "cache.t3.small", etc.
- `num_cache_nodes`: Number of nodes (1 or 3 for Multi-AZ)
- `automatic_failover_enabled`: Enable failover (false for dev, true for production)
- `snapshot_retention_limit`: Days to keep snapshots

**Outputs:**
- `primary_endpoint`: Redis connection (host:port)
- `redis_connection_string`: Full connection string with auth

**Example:**
```hcl
module "caching" {
  source = "../../modules/caching"
  
  cluster_id = "stellar-insights-dev"
  node_type = "cache.t3.micro"
  num_cache_nodes = 1
  automatic_failover_enabled = false
}
```

### compute/ecs/

Manages ECS cluster, task definitions, and auto-scaling.

**Inputs:**
- `cluster_name`: ECS cluster name
- `container_image`: ECR image URI with tag
- `desired_count`: Target number of running tasks
- `min_size` / `max_size`: Auto-scaling bounds
- `instance_type`: EC2 instance type
- `vault_addr`: Vault server address
- `db_url`: Database connection string
- `redis_url`: Redis connection string
- `enable_auto_scaling`: Enable target-tracking auto-scaling

**Outputs:**
- `cluster_name`: ECS cluster name
- `service_name`: ECS service name
- `log_group_name`: CloudWatch log group

**Example:**
```hcl
module "compute" {
  source = "../../modules/compute/ecs"
  
  cluster_name = "stellar-insights-dev"
  container_image = "123456789.dkr.ecr.us-east-1.amazonaws.com/stellar-insights-backend:latest"
  desired_count = 2
  min_size = 1
  max_size = 4
  instance_type = "t3.small"
  vault_addr = "https://vault.hcp.com:8200"
  db_url = var.db_connection_string
  redis_url = module.caching.redis_connection_string
}
```

### load_balancing/

Manages Application Load Balancer with HTTPS.

**Inputs:**
- `name`: ALB name
- `subnets`: Public subnet IDs
- `security_groups`: ALB security group IDs
- `certificate_arn`: ACM certificate ARN
- `domain_name`: Domain for HTTPS
- `target_group_name`: Target group name
- `target_port`: Backend port (8080)

**Outputs:**
- `alb_dns_name`: ALB DNS (for Route53 CNAME)
- `target_group_arn`: Target group ARN

**Example:**
```hcl
module "load_balancing" {
  source = "../../modules/load_balancing"
  
  name = "stellar-insights-alb-dev"
  subnets = module.networking.public_subnet_ids
  security_groups = [module.networking.security_group_alb_id]
  certificate_arn = "arn:aws:acm:us-east-1:123456789:certificate/..."
  domain_name = "dev-api.stellar-insights.com"
  target_port = 8080
}
```

### vault/

Reference module for Vault integration (external HCP Cloud).

**Outputs:**
- `vault_secret_paths`: Map of secret paths in Vault KV v2
- `vault_oidc_role_arn`: IAM role for GitHub OIDC

See [VAULT_INTEGRATION_GUIDE.md](../VAULT_INTEGRATION_GUIDE.md) for setup instructions.

### monitoring/

Manages CloudWatch logs, metrics, alarms, and dashboards.

**Inputs:**
- `cluster_name`: ECS cluster name
- `log_group_names`: Map of log group names
- `alarm_email`: SNS email for notifications
- `enable_dashboard`: Create CloudWatch dashboard
- `enable_alarms`: Create CloudWatch alarms

**Example:**
```hcl
module "monitoring" {
  source = "../../modules/monitoring"
  
  cluster_name = module.compute.cluster_name
  log_group_names = {
    ecs = module.compute.log_group_name
  }
  alarm_email = "ops@stellar-insights.com"
  enable_dashboard = true
  enable_alarms = true
}
```

## Environment-Specific Configuration

### Development Environment

**Cost Target:** ~$67/month

**Features:**
- Single NAT gateway
- No VPC Flow Logs
- RDS: SQLite local (no RDS)
- ECS: 1x t3.micro
- Redis: 1x cache.t3.micro, no failover
- No CloudWatch alarms
- No dashboard

**Deploy:**
```bash
cd terraform/environments/dev
terraform init -backend-config="bucket=stellar-insights-terraform-state-ACCOUNT_ID"
terraform plan
terraform apply
```

### Staging Environment

**Cost Target:** ~$205/month

**Features:**
- Single NAT gateway (can upgrade to HA)
- VPC Flow Logs enabled
- RDS: db.t3.small, 100GB, 7-day backups, Single-AZ
- ECS: 2x t3.small with auto-scaling
- Redis: 1x cache.t3.small
- CloudWatch dashboards
- CloudWatch alarms to email

**Deploy:**
```bash
cd terraform/environments/staging
terraform init -backend-config="bucket=stellar-insights-terraform-state-ACCOUNT_ID"
terraform plan
terraform apply
```

### Production Environment

**Cost Target:** ~$450/month

**Features:**
- Per-AZ NAT gateways (3x) for HA
- VPC Flow Logs enabled
- RDS: db.t3.small, 500GB, Multi-AZ, 30-day backups
- ECS: 3x t3.small auto-scaled to 10
- Redis: 3-node cluster with failover
- All CloudWatch monitoring enabled
- SNS notifications for critical alarms
- Deletion protection on critical resources

**Pre-deployment Checklist:**
- [ ] VPC Flow Logs verified
- [ ] Database: Backups tested, restore procedure documented
- [ ] Redis: Failover tested
- [ ] ALB: HTTPS certificate installed
- [ ] ECS: Container image tested
- [ ] Vault: All secrets configured
- [ ] Monitoring: Alarms tested
- [ ] DNS: Route53 configured
- [ ] Security groups: All rules reviewed
- [ ] Load testing: Verified performance

**Deploy:**
```bash
cd terraform/environments/production
terraform init -backend-config="bucket=stellar-insights-terraform-state-ACCOUNT_ID"
terraform plan
terraform apply  # Requires manual confirmation
```

## Helper Scripts

### bootstrap.sh

Bootstrap the global Terraform state backend (S3 + DynamoDB + ECR).

**Must run first before any other Terraform operations.**

```bash
./terraform/scripts/bootstrap.sh us-east-1
```

### init-state.sh

Initialize Terraform remote state for a specific environment.

```bash
./terraform/scripts/init-state.sh us-east-1 dev
./terraform/scripts/init-state.sh us-east-1 staging
./terraform/scripts/init-state.sh us-east-1 production
```

### plan.sh

Run Terraform plan with validation and cost estimates.

```bash
./terraform/scripts/plan.sh dev
terraform show tfplan  # Review detailed plan
```

### apply.sh

Apply Terraform configuration with safety checks.

```bash
./terraform/scripts/apply.sh dev
# Requires confirmation for staging
# Requires explicit "production-apply" for production
```

### destroy.sh

Destroy Terraform infrastructure (DANGEROUS!).

```bash
./terraform/scripts/destroy.sh dev
# Requires environment name confirmation
# Requires "destroy-production" for production
```

## Cost Breakdown

### Development (~$67/month)
```
ALB:                $20
NAT Gateway:        $30
ECS t3.micro:       $7
Redis cache:        $5
Data transfer:      $5
CloudWatch:         <$1
Total:              ~$67
```

### Staging (~$205/month)
```
ALB:                $20
NAT Gateway:        $30
ECS t3.small (2x):  $60
RDS t3.small:       $60
Redis cache:        $20
Data transfer:      $10
CloudWatch:         $5
Total:              ~$205
```

### Production (~$450/month)
```
ALB:                $20
NAT Gateways (3x):  $90
ECS t3.small (3x):  $90
ECS auto-scaling:   $30
RDS Multi-AZ:       $150
Redis Multi-AZ:     $40
Data transfer:      $20
CloudWatch:         $10
WAF:                ~$5
Total:              ~$455
```

## State Management

### Remote State (S3 + DynamoDB)

Terraform state is stored remotely in S3 for team collaboration:

```hcl
terraform {
  backend "s3" {
    bucket         = "stellar-insights-terraform-state-ACCOUNT_ID"
    key            = "dev/terraform.tfstate"
    dynamodb_table = "terraform-locks"
    encrypt        = true
    region         = "us-east-1"
  }
}
```

**DynamoDB Lock Table** prevents concurrent modifications.

### State Locking

State is automatically locked during `terraform apply` and `terraform destroy` operations via DynamoDB.

To manually unlock (if lock is stuck):
```bash
cd terraform/environments/dev
terraform force-unlock LOCK_ID
```

## Common Tasks

### List Resources in Environment

```bash
cd terraform/environments/dev
terraform state list
```

### Show Specific Resource

```bash
terraform state show module.compute.aws_ecs_cluster.main
```

### Refresh State from AWS

```bash
terraform refresh
```

### Update a Specific Module

```bash
terraform apply -target=module.compute
```

### Taint a Resource (force recreation)

```bash
terraform taint module.compute.aws_ecs_service.app
terraform apply
```

### Remove a Resource from State (without destroying)

```bash
terraform state rm module.monitoring
# Then delete from configuration
```

## Troubleshooting

### State Lock Stuck

If Terraform hangs with "Acquiring state lock":

```bash
# List locks
aws dynamodb scan --table-name terraform-locks --region us-east-1

# Force unlock (use LockID from above)
terraform force-unlock <LOCK_ID>
```

### Module Source Changes

If updating module source path:

```bash
rm -rf .terraform/modules
terraform init
```

### Variable Syntax Errors

Validate syntax:

```bash
terraform fmt -recursive .
terraform validate
```

### Resource Not Found in AWS

Refresh state:

```bash
terraform refresh
terraform plan  # Should show no changes if in sync
```

### Cost Overrun

Check what resources are deployed:

```bash
terraform show | grep -E "resource|Cost"
terraform state list
```

Kill expensive resources:

```bash
terraform destroy -target=module.name.resource_type.name
```

## Best Practices

1. **Always Review Plans**: Review `terraform plan` output before `terraform apply`
2. **Staging First**: Test infrastructure changes in staging before production
3. **Backup State**: Regular backups of S3 state bucket
4. **Variable Validation**: Use `validation` blocks in variables.tf
5. **Documentation**: Update README/TERRAFORM.md with configuration changes
6. **Lock Mechanism**: Never disable DynamoDB locks
7. **Secrets in Vault**: Store sensitive data in Vault, never in Terraform
8. **Auto-scaling Limits**: Set reasonable min/max for auto-scaling groups
9. **Monitoring Enabled**: Always enable CloudWatch in production
10. **Disaster Recovery**: Test database backups, snapshot recovery procedures

## CI/CD Integration

### GitHub Actions (existing workflows)

The `.github/workflows/` include:
- `backend.yml`: Build & test backend
- `frontend.yml`: Build & test frontend
- `contracts.yml`: Build Soroban contracts
- `full-stack.yml`: End-to-end testing

To add Terraform deployment:

```yaml
name: Terraform Deploy

on:
  push:
    branches: [main]
    paths: ['terraform/**']

jobs:
  terraform:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: hashicorp/setup-terraform@v2
        with:
          terraform_version: 1.5.0
      
      - name: Terraform Init
        working-directory: terraform/environments/staging
        run: terraform init
      
      - name: Terraform Plan
        working-directory: terraform/environments/staging
        run: terraform plan -out=tfplan
      
      - name: Terraform Apply
        working-directory: terraform/environments/staging
        if: github.event_name == 'push'
        run: terraform apply tfplan
```

## Support & Documentation

- [AWS Terraform Provider](https://registry.terraform.io/providers/hashicorp/aws/latest/docs)
- [Terraform State Management](https://www.terraform.io/language/state)
- [Vault Integration](../VAULT_INTEGRATION_GUIDE.md)
- [Architecture Overview](../CORRIDOR_COMPARISON.md)

## Contact

For infrastructure questions or issues:
- File an issue: https://github.com/Ndifreke000/stellar-insights/issues
- Tag: `infrastructure`, `terraform`, `aws`
