#!/bin/bash
set -e

echo "=== Kubernetes Manifest Validation ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if kubectl is installed
if ! command -v kubectl &> /dev/null; then
    echo -e "${RED}Error: kubectl is not installed${NC}"
    exit 1
fi

echo -e "${GREEN}✓ kubectl is installed${NC}"

# Check if kustomize is installed
if ! command -v kustomize &> /dev/null; then
    echo -e "${YELLOW}Warning: kustomize is not installed (optional)${NC}"
else
    echo -e "${GREEN}✓ kustomize is installed${NC}"
fi

# Validate YAML syntax
echo ""
echo "=== Validating YAML Syntax ==="
for file in $(find . -name "*.yaml" -not -path "*/overlays/*"); do
    echo "Checking $file..."
    kubectl apply --dry-run=client -f "$file" > /dev/null 2>&1
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ $file is valid${NC}"
    else
        echo -e "${RED}✗ $file has errors${NC}"
        kubectl apply --dry-run=client -f "$file"
        exit 1
    fi
done

# Validate with kustomize
if command -v kustomize &> /dev/null; then
    echo ""
    echo "=== Validating with Kustomize ==="
    
    for env in dev staging production; do
        if [ -d "overlays/$env" ]; then
            echo "Validating $env environment..."
            kustomize build overlays/$env > /tmp/k8s-$env.yaml
            kubectl apply --dry-run=client -f /tmp/k8s-$env.yaml > /dev/null 2>&1
            if [ $? -eq 0 ]; then
                echo -e "${GREEN}✓ $env environment is valid${NC}"
            else
                echo -e "${RED}✗ $env environment has errors${NC}"
                exit 1
            fi
        fi
    done
fi

# Check for common issues
echo ""
echo "=== Checking for Common Issues ==="

# Check for hardcoded secrets
echo "Checking for hardcoded secrets..."
if grep -r "CHANGE_ME" . --include="*.yaml" | grep -v "secret-template.yaml" | grep -v "validate.sh"; then
    echo -e "${RED}✗ Found hardcoded secrets (CHANGE_ME)${NC}"
    exit 1
else
    echo -e "${GREEN}✓ No hardcoded secrets found${NC}"
fi

# Check for deprecated API versions
echo "Checking for deprecated API versions..."
DEPRECATED_APIS=("extensions/v1beta1" "apps/v1beta1" "apps/v1beta2")
for api in "${DEPRECATED_APIS[@]}"; do
    if grep -r "$api" . --include="*.yaml"; then
        echo -e "${RED}✗ Found deprecated API version: $api${NC}"
        exit 1
    fi
done
echo -e "${GREEN}✓ No deprecated API versions found${NC}"

# Check resource limits
echo "Checking for resource limits..."
if grep -r "kind: Deployment" . --include="*.yaml" -A 50 | grep -q "resources:"; then
    echo -e "${GREEN}✓ Resource limits are defined${NC}"
else
    echo -e "${YELLOW}Warning: Some deployments may be missing resource limits${NC}"
fi

# Check security contexts
echo "Checking for security contexts..."
if grep -r "kind: Deployment" . --include="*.yaml" -A 50 | grep -q "securityContext:"; then
    echo -e "${GREEN}✓ Security contexts are defined${NC}"
else
    echo -e "${RED}✗ Security contexts are missing${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}=== All validations passed! ===${NC}"
