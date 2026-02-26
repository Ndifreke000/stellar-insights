#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
ENVIRONMENT=${1:-production}
DRY_RUN=${2:-false}

echo -e "${BLUE}=== Stellar Insights Kubernetes Deployment ===${NC}"
echo ""
echo "Environment: $ENVIRONMENT"
echo "Dry Run: $DRY_RUN"
echo ""

# Validate environment
if [[ ! "$ENVIRONMENT" =~ ^(dev|staging|production)$ ]]; then
    echo -e "${RED}Error: Invalid environment. Must be dev, staging, or production${NC}"
    exit 1
fi

# Check prerequisites
echo "=== Checking Prerequisites ==="
if ! command -v kubectl &> /dev/null; then
    echo -e "${RED}Error: kubectl is not installed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ kubectl is installed${NC}"

if ! command -v kustomize &> /dev/null; then
    echo -e "${RED}Error: kustomize is not installed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ kustomize is installed${NC}"

# Check cluster connection
echo ""
echo "=== Checking Cluster Connection ==="
if ! kubectl cluster-info &> /dev/null; then
    echo -e "${RED}Error: Cannot connect to Kubernetes cluster${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Connected to cluster${NC}"
kubectl cluster-info | head -n 1

# Validate manifests
echo ""
echo "=== Validating Manifests ==="
./scripts/validate.sh
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Manifest validation failed${NC}"
    exit 1
fi

# Build kustomization
echo ""
echo "=== Building Kustomization ==="
if [ "$ENVIRONMENT" == "production" ]; then
    KUSTOMIZE_PATH="overlays/production"
else
    KUSTOMIZE_PATH="overlays/$ENVIRONMENT"
fi

kustomize build "$KUSTOMIZE_PATH" > /tmp/k8s-deploy.yaml
echo -e "${GREEN}✓ Kustomization built successfully${NC}"

# Deploy
echo ""
if [ "$DRY_RUN" == "true" ]; then
    echo "=== Dry Run - No changes will be applied ==="
    kubectl apply --dry-run=client -f /tmp/k8s-deploy.yaml
else
    echo "=== Deploying to Cluster ==="
    echo -e "${YELLOW}This will apply changes to the cluster. Continue? (y/n)${NC}"
    read -r response
    if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
        kubectl apply -f /tmp/k8s-deploy.yaml
        
        # Wait for rollout
        echo ""
        echo "=== Waiting for Rollout ==="
        kubectl rollout status deployment/stellar-insights-backend -n stellar-insights --timeout=5m
        kubectl rollout status deployment/stellar-insights-frontend -n stellar-insights --timeout=5m
        
        echo ""
        echo -e "${GREEN}=== Deployment Complete! ===${NC}"
        
        # Show status
        echo ""
        echo "=== Deployment Status ==="
        kubectl get pods -n stellar-insights
        
        echo ""
        echo "=== Services ==="
        kubectl get svc -n stellar-insights
        
        echo ""
        echo "=== Ingress ==="
        kubectl get ingress -n stellar-insights
    else
        echo "Deployment cancelled"
        exit 0
    fi
fi

echo ""
echo -e "${GREEN}Done!${NC}"
