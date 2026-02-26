#!/usr/bin/env bash
set -euo pipefail

# Emergency rollback script for stellar-insights backend
# Usage: ./scripts/rollback.sh <environment>

AWS_REGION="us-east-1"
CONTAINER_NAME="stellar-insights"
CONTAINER_PORT=8080

usage() {
    echo "Usage: $0 <environment>"
    echo ""
    echo "Arguments:"
    echo "  environment   staging or production"
    echo ""
    echo "This script will:"
    echo "  1. Stop any in-progress deployment (triggers auto-rollback), OR"
    echo "  2. Redeploy the previous task definition revision"
    exit 1
}

if [ $# -ne 1 ]; then
    usage
fi

ENVIRONMENT="$1"

if [[ "$ENVIRONMENT" != "staging" && "$ENVIRONMENT" != "production" ]]; then
    echo "Error: environment must be 'staging' or 'production'"
    exit 1
fi

CLUSTER="stellar-insights-${ENVIRONMENT}"
SERVICE="stellar-insights-service"
TASK_FAMILY="stellar-insights-${ENVIRONMENT}"
CODEDEPLOY_APP="stellar-insights-${ENVIRONMENT}"
CODEDEPLOY_GROUP="stellar-insights-${ENVIRONMENT}-dg"

echo "=== Emergency Rollback ==="
echo "Environment: ${ENVIRONMENT}"
echo ""

# Check for in-progress deployment
echo "Checking for active deployments..."
ACTIVE_DEPLOYMENT=$(aws deploy list-deployments \
    --application-name "${CODEDEPLOY_APP}" \
    --deployment-group-name "${CODEDEPLOY_GROUP}" \
    --include-only-statuses "InProgress" "Queued" \
    --query 'deployments[0]' \
    --output text 2>/dev/null || echo "None")

if [ "$ACTIVE_DEPLOYMENT" != "None" ] && [ -n "$ACTIVE_DEPLOYMENT" ]; then
    echo "Found active deployment: ${ACTIVE_DEPLOYMENT}"
    echo ""
    read -r -p "Stop this deployment and trigger auto-rollback? [y/N] " CONFIRM
    if [[ "$CONFIRM" =~ ^[Yy]$ ]]; then
        echo "Stopping deployment..."
        aws deploy stop-deployment \
            --deployment-id "${ACTIVE_DEPLOYMENT}" \
            --auto-rollback-enabled
        echo "Deployment stopped — auto-rollback triggered."
        echo ""
        echo "Monitor rollback progress:"
        echo "  aws deploy get-deployment --deployment-id ${ACTIVE_DEPLOYMENT}"
    else
        echo "Aborted."
    fi
    exit 0
fi

echo "No active deployment found."
echo ""

# Get current task definition revision
CURRENT_TASK_DEF=$(aws ecs describe-services \
    --cluster "${CLUSTER}" \
    --services "${SERVICE}" \
    --query 'services[0].taskDefinition' \
    --output text)

CURRENT_REV=$(echo "$CURRENT_TASK_DEF" | grep -o '[0-9]*$')
PREV_REV=$((CURRENT_REV - 1))

if [ "$PREV_REV" -lt 1 ]; then
    echo "Error: No previous task definition revision to roll back to."
    exit 1
fi

PREV_TASK_DEF="${TASK_FAMILY}:${PREV_REV}"

# Get previous image for display
PREV_IMAGE=$(aws ecs describe-task-definition \
    --task-definition "${PREV_TASK_DEF}" \
    --query "taskDefinition.containerDefinitions[?name=='${CONTAINER_NAME}'].image" \
    --output text)

PREV_ARN=$(aws ecs describe-task-definition \
    --task-definition "${PREV_TASK_DEF}" \
    --query 'taskDefinition.taskDefinitionArn' \
    --output text)

echo "Current revision: ${CURRENT_REV}"
echo "Rollback to:      ${PREV_REV}"
echo "Previous image:   ${PREV_IMAGE}"
echo ""

read -r -p "Proceed with rollback? [y/N] " CONFIRM
if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

# Create rollback deployment
APPSPEC=$(jq -nc \
    --arg TD "$PREV_ARN" \
    --arg CN "$CONTAINER_NAME" \
    --argjson CP "$CONTAINER_PORT" \
    '{
        version: 0.0,
        Resources: [{
            TargetService: {
                Type: "AWS::ECS::Service",
                Properties: {
                    TaskDefinition: $TD,
                    LoadBalancerInfo: {
                        ContainerName: $CN,
                        ContainerPort: $CP
                    }
                }
            }
        }]
    }')

echo "Creating rollback deployment..."
DEPLOYMENT_ID=$(aws deploy create-deployment \
    --application-name "${CODEDEPLOY_APP}" \
    --deployment-group-name "${CODEDEPLOY_GROUP}" \
    --revision "revisionType=AppSpecContent,appSpecContent={content='${APPSPEC}'}" \
    --description "Rollback to revision ${PREV_REV}" \
    --query 'deploymentId' \
    --output text)

echo "Rollback deployment created: ${DEPLOYMENT_ID}"

# Wait for deployment
echo "Waiting for rollback to complete..."
if aws deploy wait deployment-successful --deployment-id "${DEPLOYMENT_ID}"; then
    echo ""
    echo "=== Rollback successful! ==="
    echo "Deployment ID: ${DEPLOYMENT_ID}"
    echo "Rolled back to: revision ${PREV_REV} (${PREV_IMAGE})"
else
    echo ""
    echo "=== Rollback deployment failed ==="
    echo "  aws deploy get-deployment --deployment-id ${DEPLOYMENT_ID}"
    exit 1
fi
