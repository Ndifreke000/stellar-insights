# Load Balancing Module

Manages AWS Application Load Balancer (ALB) and target groups for Stellar Insights.

## Features

- Application Load Balancer (ALB)
- HTTP and HTTPS listeners
- HTTP → HTTPS redirect
- SSL/TLS certificate from AWS Certificate Manager (ACM)
- Target group with health checks
- CloudWatch monitoring and alarms
- Request logging (S3)
- WAF integration (optional)

## Architecture

```
Internet → (HTTPS 443) → ALB → (HTTP 8080) → ECS Service → Backend Rust Server
              ↓ HTTP 80 (redirect to 443)
```

## Usage

```hcl
module "load_balancing" {
  source = "../../modules/load_balancing"

  # ALB configuration
  name                = "stellar-insights-alb-${var.environment}"
  internal            = false
  load_balancer_type  = "application"
  
  # Networking
  subnets            = module.networking.public_subnet_ids
  security_groups    = [module.networking.security_group_alb_id]
  
  # Target group
  target_group_name  = "stellar-insights-targets"
  target_port        = 8080
  
  # SSL/TLS
  certificate_arn    = aws_acm_certificate.main.arn
  domain_name        = "api.stellar-insights.com"
  
  # Logging
  enable_logs         = var.environment == "production"
  logs_bucket         = aws_s3_bucket.alb_logs.id
  
  environment        = var.environment
}
```

## Inputs

| Name | Description | Type | Required |
|------|-------------|------|----------|
| name | ALB name | `string` | Yes |
| internal | Internal vs public ALB | `bool` | No (default: `false`) |
| load_balancer_type | Type (application, network) | `string` | No (default: `application`) |
| subnets | Public subnet IDs | `list(string)` | Yes |
| security_groups | Security group IDs | `list(string)` | Yes |
| target_group_name | Target group name | `string` | Yes |
| target_port | Backend port (8080) | `number` | No |
| certificate_arn | ACM certificate ARN | `string` | Yes |
| domain_name | Domain for HTTPS | `string` | Yes |
| enable_logs | Enable request logging | `bool` | No (default: `true`) |
| logs_bucket | S3 bucket for logs | `string` | Yes (if logs enabled) |
| enable_waf | Enable WAF | `bool` | No (default: `false`) |
| environment | Environment name | `string` | Yes |

## Outputs

| Name | Description |
|------|-------------|
| alb_dns_name | ALB DNS name for Route53 CNAME |
| alb_arn | ALB ARN |
| target_group_arn | Target group ARN (for ECS service) |
| security_group_id | ALB security group ID |

## Cost Estimates

**All Environments:**
- ALB: ~$20/month
- Data processing: variable (~$5-20/month)
- Monthly: ~$25-40/month

## Notes

- Certificate management via AWS Certificate Manager (ACM)
- HTTP to HTTPS redirect is automatic
- Health checks every 30 seconds
- Unhealthy threshold: 3 checks
- Request/response timeout: 60 seconds
- See [networking/security_groups.tf](../networking/security_groups.tf) for ALB → backend routing rules
- For custom domains: update Route53 with ALB CNAME
