#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

NAMESPACE=${1:-stellar-insights}

echo -e "${BLUE}=== Testing Kubernetes Deployment ===${NC}"
echo ""
echo "Namespace: $NAMESPACE"
echo ""

# Check if namespace exists
echo "=== Checking Namespace ==="
if kubectl get namespace "$NAMESPACE" &> /dev/null; then
    echo -e "${GREEN}✓ Namespace $NAMESPACE exists${NC}"
else
    echo -e "${RED}✗ Namespace $NAMESPACE does not exist${NC}"
    exit 1
fi

# Check pods
echo ""
echo "=== Checking Pods ==="
PODS=$(kubectl get pods -n "$NAMESPACE" --no-headers)
if [ -z "$PODS" ]; then
    echo -e "${RED}✗ No pods found${NC}"
    exit 1
fi

echo "$PODS"
echo ""

# Check if all pods are running
NOT_RUNNING=$(kubectl get pods -n "$NAMESPACE" --field-selector=status.phase!=Running --no-headers | wc -l)
if [ "$NOT_RUNNING" -gt 0 ]; then
    echo -e "${YELLOW}Warning: $NOT_RUNNING pods are not running${NC}"
    kubectl get pods -n "$NAMESPACE" --field-selector=status.phase!=Running
else
    echo -e "${GREEN}✓ All pods are running${NC}"
fi

# Check pod readiness
echo ""
echo "=== Checking Pod Readiness ==="
NOT_READY=$(kubectl get pods -n "$NAMESPACE" -o json | jq -r '.items[] | select(.status.conditions[] | select(.type=="Ready" and .status!="True")) | .metadata.name' | wc -l)
if [ "$NOT_READY" -gt 0 ]; then
    echo -e "${RED}✗ $NOT_READY pods are not ready${NC}"
    kubectl get pods -n "$NAMESPACE" -o json | jq -r '.items[] | select(.status.conditions[] | select(.type=="Ready" and .status!="True")) | .metadata.name'
    exit 1
else
    echo -e "${GREEN}✓ All pods are ready${NC}"
fi

# Check services
echo ""
echo "=== Checking Services ==="
SERVICES=$(kubectl get svc -n "$NAMESPACE" --no-headers)
if [ -z "$SERVICES" ]; then
    echo -e "${RED}✗ No services found${NC}"
    exit 1
fi

echo "$SERVICES"
echo -e "${GREEN}✓ Services are configured${NC}"

# Check ingress
echo ""
echo "=== Checking Ingress ==="
INGRESS=$(kubectl get ingress -n "$NAMESPACE" --no-headers)
if [ -z "$INGRESS" ]; then
    echo -e "${YELLOW}Warning: No ingress found${NC}"
else
    echo "$INGRESS"
    echo -e "${GREEN}✓ Ingress is configured${NC}"
fi

# Test backend health endpoint
echo ""
echo "=== Testing Backend Health Endpoint ==="
BACKEND_POD=$(kubectl get pods -n "$NAMESPACE" -l component=backend -o jsonpath='{.items[0].metadata.name}')
if [ -z "$BACKEND_POD" ]; then
    echo -e "${RED}✗ No backend pod found${NC}"
    exit 1
fi

HEALTH_CHECK=$(kubectl exec -n "$NAMESPACE" "$BACKEND_POD" -- wget -q -O- http://localhost:8080/health 2>/dev/null || echo "failed")
if [ "$HEALTH_CHECK" == "failed" ]; then
    echo -e "${RED}✗ Backend health check failed${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Backend health check passed${NC}"
    echo "Response: $HEALTH_CHECK"
fi

# Check HPA
echo ""
echo "=== Checking Horizontal Pod Autoscaler ==="
HPA=$(kubectl get hpa -n "$NAMESPACE" --no-headers)
if [ -z "$HPA" ]; then
    echo -e "${YELLOW}Warning: No HPA found${NC}"
else
    echo "$HPA"
    echo -e "${GREEN}✓ HPA is configured${NC}"
fi

# Check PDB
echo ""
echo "=== Checking Pod Disruption Budget ==="
PDB=$(kubectl get pdb -n "$NAMESPACE" --no-headers)
if [ -z "$PDB" ]; then
    echo -e "${YELLOW}Warning: No PDB found${NC}"
else
    echo "$PDB"
    echo -e "${GREEN}✓ PDB is configured${NC}"
fi

# Check resource usage
echo ""
echo "=== Checking Resource Usage ==="
kubectl top pods -n "$NAMESPACE" 2>/dev/null || echo -e "${YELLOW}Warning: Metrics server not available${NC}"

# Check recent events
echo ""
echo "=== Recent Events ==="
kubectl get events -n "$NAMESPACE" --sort-by='.lastTimestamp' | tail -n 10

echo ""
echo -e "${GREEN}=== All tests passed! ===${NC}"
