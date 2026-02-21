#!/bin/bash
# Apply Terraform configuration for an environment
#
# This script applies a pre-planned Terraform configuration with safety checks.
#
# Usage: ./terraform/scripts/apply.sh dev
# Usage: ./terraform/scripts/apply.sh staging
# Usage: ./terraform/scripts/apply.sh production

set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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
        exit 1
        ;;
esac

echo -e "${GREEN}=== Terraform Apply ===${NC}"
echo -e "Environment: ${YELLOW}${ENVIRONMENT}${NC}"
echo ""

cd "${STATE_PATH}" || exit 1

# Check if tfplan exists
if [[ ! -f "tfplan" ]]; then
    echo -e "${YELLOW}No pre-planned tfplan found. Running terraform plan...${NC}"
    terraform plan -out=tfplan
fi

# Show plan summary
echo -e "${YELLOW}Plan Summary:${NC}"
terraform show tfplan | grep -E "Plan:|will be created|will be updated|will be destroyed" || true
echo ""

# Safety checks
if [[ "${ENVIRONMENT}" == "production" ]]; then
    echo -e "${RED}WARNING: This is PRODUCTION environment!${NC}"
    echo -e "Review the plan carefully. Continue? (type 'production-apply' to confirm)"
    read -r CONFIRM
    if [[ "${CONFIRM}" != "production-apply" ]]; then
        echo "Cancelled."
        rm tfplan 2>/dev/null || true
        exit 1
    fi
else
    echo -e "Review the plan. Continue? (yes/no)"
    read -r CONFIRM
    if [[ "${CONFIRM}" != "yes" ]]; then
        echo "Cancelled."
        rm tfplan 2>/dev/null || true
        exit 1
    fi
fi

# Apply the plan
echo ""
echo -e "${YELLOW}Applying configuration...${NC}"
terraform apply tfplan

echo ""
echo -e "${GREEN}✓ Apply complete${NC}"
echo ""

# Show outputs
echo -e "${YELLOW}Outputs:${NC}"
terraform output

echo ""
echo -e "${GREEN}=== Next Steps ===${NC}"
echo ""
echo "1. Verify deployment:"
echo -e "   ${YELLOW}terraform show${NC}"
echo ""
echo "2. Check resource status (AWS Console):"
echo -e "   ECS: https://console.aws.amazon.com/ecs/v2/clusters"
echo -e "   RDS: https://console.aws.amazon.com/rds/home"
echo -e "   ALB: https://console.aws.amazon.com/ec2/v2/home#LoadBalancers"
echo ""
echo "3. Access the application (if ALB deployed):"
echo -e "   ${YELLOW}curl https://\$(terraform output -raw alb_dns_name)/health${NC}"
echo ""
