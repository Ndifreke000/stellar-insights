# Networking Module for stellar-insights

This module provisions all network infrastructure:
- **VPC** (10.0.0.0/16) with DNS enabled
- **Subnets**: 2 public (for ALB), 4 private (for app/database) across 3 AZs
- **Internet Gateway** for public subnet egress
- **NAT Gateways** (1 per AZ) for private subnet egress
- **Route Tables** for public and private routing
- **Security Groups**: ALB, backend, database, Redis

## Variables (set in environment terraform.tfvars)
- `environment` - dev/staging/production
- `vpc_cidr` - VPC CIDR block (default: 10.0.0.0/16)
- `enable_nat_per_az` - NAT gateway per AZ (true for prod, false for dev)
- `enable_vpc_flow_logs` - Send VPC logs to CloudWatch (true for prod)

## Outputs (used by other modules)
- `vpc_id` - VPC resource ID
- `private_subnet_ids` - List of private subnet IDs for ECS/RDS
- `public_subnet_ids` - List of public subnet IDs for ALB
- `security_group_*` - Security group IDs (backend, database, redis)

## Architecture
```
Public Tier (ALB)
├── AZ-A: 10.0.1.0/24 (igw-nat)
├── AZ-B: 10.0.2.0/24 (igw-nat)
└── AZ-C: 10.0.3.0/24 (igw-nat)

Private Tier (Backend ECS)
├── AZ-A: 10.0.11.0/24 (nat)
├── AZ-B: 10.0.12.0/24 (nat)
└── AZ-C: 10.0.13.0/24 (nat)

Private Tier (Database/Redis)
├── AZ-A: 10.0.21.0/24 (nat)
├── AZ-B: 10.0.22.0/24 (nat)
└── AZ-C: 10.0.23.0/24 (nat)
```

## Dev vs Production
| Aspect | Dev | Production |
|--------|-----|-----------|
| NAT Gateways | 1 (shared) | 3 (one per AZ) |
| VPC Flow Logs | No | Yes |
| Subnets | 2 AZs only | 3 AZs for HA |

## Usage
```hcl
module "networking" {
  source = "./modules/networking"

  environment           = var.environment
  vpc_cidr              = "10.0.0.0/16"
  enable_nat_per_az     = var.environment == "production"
  enable_vpc_flow_logs  = var.environment == "production"
}
```

See terraform/environments/ for example configurations.
