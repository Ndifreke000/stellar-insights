#!/bin/bash
# Destroy Terraform infrastructure for an environment (DANGEROUS!)
#
# This script destroys all infrastructure for an environment.
# WARNING: This will delete databases, storage, and all resources!
#
# Usage: ./terraform/scripts/destroy.sh dev
# Usage: ./terraform/scripts/destroy.sh staging
# Usage: ./terraform/scripts/destroy.sh production

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

echo -e "${RED}=== TERRAFORM DESTROY ===${NC}"
echo -e "${RED}WARNING: This will DELETE all infrastructure for: ${YELLOW}${ENVIRONMENT}${RED}${NC}"
echo ""

# Validate environment
case "${ENVIRONMENT}" in
    dev)
        echo "Cost saved: ~\$67/month"
        ;;
    staging)
        echo "Cost saved: ~\$205/month"
        ;;
    production)
        echo -e "${RED}Cost saved: ~\$450/month${NC}"
        echo -e "${RED}CRITICAL: All production data will be DELETED!${NC}"
        echo -e "${RED}Ensure you have backups in place before proceeding.${NC}"
        ;;
    *)
        echo -e "${RED}✗ Invalid environment: ${ENVIRONMENT}${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "Resources that will be destroyed:"
echo "- VPC, subnets, routing tables, security groups"
echo "- ALB and target groups"
echo "- ECS cluster, services, tasks"
if [[ "${ENVIRONMENT}" != "dev" ]]; then
    echo "- RDS PostgreSQL database (DATA WILL BE LOST)"
fi
echo "- ElastiCache Redis cluster"
echo "- CloudWatch logs and alarms"
echo "- IAM roles and policies"
echo ""

cd "${STATE_PATH}" || exit 1

# Confirmation prompts
echo -e "${RED}This action CANNOT be undone.${NC}"
echo -e "Type the environment name to confirm: ${YELLOW}${ENVIRONMENT}${NC}"
read -r CONFIRM1

if [[ "${CONFIRM1}" != "${ENVIRONMENT}" ]]; then
    echo "Confirmation mismatch. Cancelled."
    exit 1
fi

if [[ "${ENVIRONMENT}" == "production" ]]; then
    echo -e "${RED}This is PRODUCTION. Are you absolutely certain? (type 'destroy-production')${NC}"
    read -r CONFIRM2
    if [[ "${CONFIRM2}" != "destroy-production" ]]; then
        echo "Production destroy cancelled."
        exit 1
    fi
fi

# Destroy
echo ""
echo -e "${YELLOW}Destroying infrastructure...${NC}"
terraform destroy

echo ""
echo -e "${RED}✓ Infrastructure destroyed${NC}"
echo ""
echo -e "${YELLOW}Cleanup:${NC}"
echo "- Remove local state files: rm -rf .terraform/"
echo "- S3 bucket retention: Terraform state remains for recovery (can be manually deleted)"
echo "- RDS snapshots: Check AWS Console for final snapshots"
echo ""
