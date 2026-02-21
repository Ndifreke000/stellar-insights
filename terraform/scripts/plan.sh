#!/bin/bash
# Plan Terraform configuration for an environment
#
# This script runs a Terraform plan with warning checks and cost estimates.
#
# Usage: ./terraform/scripts/plan.sh dev
# Usage: ./terraform/scripts/plan.sh staging
# Usage: ./terraform/scripts/plan.sh production

set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get environment from argument
if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <environment>"
    echo "Example: $0 dev"
    exit 1
fi

ENVIRONMENT="${1}"
STATE_PATH="terraform/environments/${ENVIRONMENT}"

# Validate environment
case "${ENVIRONMENT}" in
    dev|staging|production)
        ;;
    *)
        echo -e "${RED}✗ Invalid environment: ${ENVIRONMENT}${NC}"
        echo "Must be one of: dev, staging, production"
        exit 1
        ;;
esac

echo -e "${GREEN}=== Terraform Plan ===${NC}"
echo -e "Environment: ${YELLOW}${ENVIRONMENT}${NC}"
echo -e "State Path: ${YELLOW}${STATE_PATH}${NC}"
echo ""

# Check state exists
if [[ ! -d "${STATE_PATH}" ]]; then
    echo -e "${RED}✗ Environment directory not found: ${STATE_PATH}${NC}"
    exit 1
fi

cd "${STATE_PATH}" || exit 1

# Step 1: Validate configuration
echo -e "${YELLOW}Step 1: Validating configuration...${NC}"
terraform validate
echo -e "${GREEN}✓ Configuration valid${NC}"
echo ""

# Step 2: Format check
echo -e "${YELLOW}Step 2: Checking format...${NC}"
if terraform fmt -check -recursive . > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Format OK${NC}"
else
    echo -e "${YELLOW}⚠ Format issues found (auto-fixing)${NC}"
    terraform fmt -recursive .
fi
echo ""

# Step 3: Run plan
echo -e "${YELLOW}Step 3: Running Terraform plan...${NC}"
terraform plan -out=tfplan

echo -e "${GREEN}✓ Plan created: tfplan${NC}"
echo ""

# Step 4: Show summary
echo -e "${YELLOW}Step 4: Plan Summary${NC}"
echo ""
terraform show tfplan | grep -E "Plan:|will be created|will be updated|will be destroyed" || true
echo ""

# Step 5: Resource count
RESOURCE_COUNT=$(terraform show tfplan -json | jq '[.resource_changes[]?.change.actions[]?] | length' 2>/dev/null || echo "?")
echo -e "Total resource changes: ${YELLOW}${RESOURCE_COUNT}${NC}"
echo ""

# Step 6: Cost estimate (if available)
echo -e "${BLUE}--- Cost Estimate ---${NC}"
if [[ "${ENVIRONMENT}" == "dev" ]]; then
    echo "ALB: \$20/month"
    echo "NAT Gateway: \$30/month"
    echo "ECS t3.micro: \$7/month"
    echo "ElastiCache: \$5/month"
    echo "Data transfer: \$5/month"
    echo -e "Total: ${YELLOW}~\$67/month${NC}"
elif [[ "${ENVIRONMENT}" == "staging" ]]; then
    echo "ALB: \$20/month"
    echo "NAT Gateway: \$30/month"
    echo "ECS t3.small: \$60/month"
    echo "RDS t3.small: \$60/month"
    echo "ElastiCache: \$20/month"
    echo "Data transfer: \$15/month"
    echo -e "Total: ${YELLOW}~\$205/month${NC}"
elif [[ "${ENVIRONMENT}" == "production" ]]; then
    echo "ALB: \$20/month"
    echo "NAT Gateways (3x): \$90/month"
    echo "ECS t3.small (3x): \$90/month"
    echo "ECS auto-scaling: \$30/month"
    echo "RDS Multi-AZ: \$150/month"
    echo "ElastiCache Multi-AZ: \$40/month"
    echo "Data transfer: \$20/month"
    echo "CloudWatch: \$10/month"
    echo -e "Total: ${YELLOW}~\$450/month${NC}"
fi
echo ""

# Step 7: Next steps
echo -e "${GREEN}=== Next Steps ===${NC}"
echo ""
echo "1. Review the plan:"
echo -e "   ${YELLOW}terraform show tfplan | less${NC}"
echo ""
echo "2. Apply the configuration:"
echo -e "   ${YELLOW}./scripts/plan.sh apply ${ENVIRONMENT}${NC}"
echo "   OR"
echo -e "   ${YELLOW}terraform apply tfplan${NC}"
echo ""
echo "3. Cancel and clean up:"
echo -e "   ${YELLOW}rm tfplan${NC}"
echo ""
