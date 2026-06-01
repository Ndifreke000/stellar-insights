# Terraform Global Setup Bootstrap

This directory sets up the foundational AWS infrastructure:
- **S3 bucket** for Terraform remote state (with versioning, encryption, MFA delete)
- **ECR repositories** for Docker images (backend, frontend)
- **IAM roles** for Terraform execution
- **DynamoDB table** for state locking

## ⚠️ BOOTSTRAP PROCESS (Critical)

This module must be applied **FIRST and ONLY ONCE** before any other Terraform modules.

### Step 1: Create AWS Backend Resources Locally
```bash
cd terraform/global/
terraform init
terraform plan
terraform apply
```

**Do NOT set remote state for this module** — it needs local state while creating the S3 bucket.

### Step 2: Enable Remote State in Other Modules
After `terraform apply` completes, other modules will reference the S3 bucket and DynamoDB table via `backend.tf`:

```hcl
terraform {
  backend "s3" {
    bucket         = "stellar-insights-terraform-state-${aws_account_id}"
    key            = "${environment}/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "terraform-locks"
  }
}
```

### Step 3: Copy Outputs
```bash
terraform output
# Copy values to environments/*/terraform.tfvars
```

## Variables
- `aws_region` - Default: `us-east-1`
- `environment` - Default: `dev`
- `enable_mfa_delete` - Default: `false` (enable for production)

## Outputs
- `state_bucket_name` - S3 bucket for Terraform state
- `dynamodb_table_name` - Lock table name
- `ecr_backend_repository_url` - ECR for backend Docker image
- `ecr_frontend_repository_url` - ECR for frontend Docker image

## Important Notes
- This module creates long-lived AWS resources
- **DO NOT destroy this module** unless you plan to lose all Terraform state
- S3 bucket has versioning enabled (recover old states)
- Encryption at rest (SSE-S3)
- Block public access enabled
