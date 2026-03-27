#!/bin/bash
# Initialize S3 remote state backend (MUST run first before other terraform commands)
# 
# This script creates the S3 bucket and DynamoDB table for Terraform state management.
# CRITICAL: Run terraform/global/ apply first (bootstrap.sh), then this script.
#
# Usage: ./terraform/scripts/init-state.sh us-east-1 dev
# Usage: ./terraform/scripts/init-state.sh us-east-1 staging  
# Usage: ./terraform/scripts/init-state.sh us-east-1 production

set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get AWS region and environment from arguments
if [[ $# -lt 2 ]]; then
    echo "Usage: $0 <aws_region> <environment>"
    echo "Example: $0 us-east-1 dev"
    exit 1
fi

AWS_REGION="${1:-us-east-1}"
ENVIRONMENT="${2:-dev}"
ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
BUCKET_NAME="stellar-insights-terraform-state-${ACCOUNT_ID}"
STATE_PATH="terraform/environments/${ENVIRONMENT}"

echo -e "${GREEN}=== Terraform State Backend Initialization ===${NC}"
echo -e "Region: ${YELLOW}${AWS_REGION}${NC}"
echo -e "Environment: ${YELLOW}${ENVIRONMENT}${NC}"
echo -e "Account ID: ${YELLOW}${ACCOUNT_ID}${NC}"
echo -e "S3 Bucket: ${YELLOW}${BUCKET_NAME}${NC}"
echo -e "State Path: ${YELLOW}${STATE_PATH}${NC}"
echo ""

# Step 1: Verify S3 bucket exists (created by terraform/global/apply)
echo -e "${YELLOW}Step 1: Checking S3 bucket...${NC}"
if aws s3api head-bucket --bucket "${BUCKET_NAME}" --region "${AWS_REGION}" 2>/dev/null; then
    echo -e "${GREEN}✓ S3 bucket exists${NC}"
else
    echo -e "${RED}✗ S3 bucket not found. Run 'terraform/scripts/bootstrap.sh' first to create the global stack.${NC}"
    exit 1
fi

# Step 2: Verify DynamoDB table exists
echo -e "${YELLOW}Step 2: Checking DynamoDB locks table...${NC}"
if aws dynamodb describe-table --table-name terraform-locks --region "${AWS_REGION}" 2>/dev/null | grep -q "TableName"; then
    echo -e "${GREEN}✓ DynamoDB table exists${NC}"
else
    echo -e "${RED}✗ DynamoDB table not found. Run 'terraform/scripts/bootstrap.sh' first.${NC}"
    exit 1
fi

# Step 3: Initialize Terraform with S3 backend
echo -e "${YELLOW}Step 3: Initializing Terraform with S3 backend...${NC}"
cd "${STATE_PATH}" || { echo "Directory ${STATE_PATH} not found"; exit 1; }

terraform init \
    -backend-config="bucket=${BUCKET_NAME}" \
    -backend-config="key=${ENVIRONMENT}/terraform.tfstate" \
    -backend-config="region=${AWS_REGION}" \
    -backend-config="dynamodb_table=terraform-locks" \
    -backend-config="encrypt=true"

echo -e "${GREEN}✓ Terraform initialized${NC}"
echo ""

# Step 4: Suggest next steps
echo -e "${GREEN}=== Next Steps ===${NC}"
echo ""
echo "1. Review the Terraform plan:"
echo -e "   ${YELLOW}cd ${STATE_PATH}${NC}"
echo -e "   ${YELLOW}terraform plan${NC}"
echo ""
echo "2. Review cost estimates:"
echo -e "   ${YELLOW}terraform plan | grep -i cost${NC}"
echo ""
echo "3. Apply the configuration:"
echo -e "   ${YELLOW}terraform apply${NC}"
echo ""
echo "Tip: Set AWS_PROFILE if using multiple AWS credentials:"
echo -e "   ${YELLOW}AWS_PROFILE=stellar-insights terraform plan${NC}"
