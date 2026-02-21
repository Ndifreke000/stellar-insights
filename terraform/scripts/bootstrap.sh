#!/bin/bash
# Bootstrap the global Terraform state (MUST run first)
#
# This script initializes S3 remote state backend and creates ECR repositories.
# Run this ONCE before creating any other infrastructure.
#
# Usage: ./terraform/scripts/bootstrap.sh us-east-1

set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get AWS region from argument
if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <aws_region>"
    echo "Example: $0 us-east-1"
    exit 1
fi

AWS_REGION="${1:-us-east-1}"
ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)

echo -e "${GREEN}=== Terraform Global Bootstrap ===${NC}"
echo -e "Region: ${YELLOW}${AWS_REGION}${NC}"
echo -e "Account ID: ${YELLOW}${ACCOUNT_ID}${NC}"
echo ""

# Step 1: Initialize Terraform in /global (no backend initially)
echo -e "${YELLOW}Step 1: Initializing global Terraform...${NC}"
cd terraform/global || { echo "terraform/global directory not found"; exit 1; }

terraform init

echo -e "${GREEN}✓ Global Terraform initialized${NC}"
echo ""

# Step 2: Plan the global infrastructure
echo -e "${YELLOW}Step 2: Planning global infrastructure...${NC}"
terraform plan -out=tfplan

echo -e "${GREEN}✓ Plan created${NC}"
echo ""

# Step 3: Apply with confirmation
echo -e "${YELLOW}Step 3: Applying global infrastructure...${NC}"
echo -e "${RED}WARNING: This will create S3 buckets, DynamoDB tables, and IAM roles.${NC}"
echo -e "Continue? (yes/no)"
read -r CONFIRM
if [[ "${CONFIRM}" != "yes" ]]; then
    echo "Cancelled."
    exit 1
fi

terraform apply tfplan

echo -e "${GREEN}✓ Global infrastructure created${NC}"
echo ""

# Step 4: Get outputs
echo -e "${YELLOW}Step 4: Retrieving outputs...${NC}"
STATE_BUCKET=$(terraform output -raw state_bucket_name)
ECR_BACKEND=$(terraform output -json ecr_repo_urls | jq -r '.backend')
ECR_FRONTEND=$(terraform output -json ecr_repo_urls | jq -r '.frontend')

echo -e "${GREEN}✓ Outputs retrieved${NC}"
echo ""

# Step 5: Summary
echo -e "${GREEN}=== Bootstrap Complete ===${NC}"
echo ""
echo "S3 State Bucket: ${YELLOW}${STATE_BUCKET}${NC}"
echo "ECR Backend: ${YELLOW}${ECR_BACKEND}${NC}"
echo "ECR Frontend: ${YELLOW}${ECR_FRONTEND}${NC}"
echo ""

# Step 6: Next steps
echo -e "${GREEN}=== Next Steps ===${NC}"
echo ""
echo "1. Push container images to ECR:"
echo -e "   ${YELLOW}aws ecr get-login-password --region ${AWS_REGION} | docker login --username AWS --password-stdin ${ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com${NC}"
echo -e "   ${YELLOW}docker build -t stellar-insights-backend:latest -f backend/Dockerfile .${NC}"
echo -e "   ${YELLOW}docker tag stellar-insights-backend:latest ${ECR_BACKEND}:latest${NC}"
echo -e "   ${YELLOW}docker push ${ECR_BACKEND}:latest${NC}"
echo ""
echo "2. Initialize dev environment state:"
echo -e "   ${YELLOW}./terraform/scripts/init-state.sh ${AWS_REGION} dev${NC}"
echo ""
echo "3. Deploy dev environment:"
echo -e "   ${YELLOW}cd terraform/environments/dev${NC}"
echo -e "   ${YELLOW}terraform plan${NC}"
echo -e "   ${YELLOW}terraform apply${NC}"
echo ""
