#!/usr/bin/env bash
set -euo pipefail

# Blue-green deployment script for stellar-insights backend
# Usage: ./scripts/deploy.sh <environment> <image-tag>

AWS_REGION="us-east-1"
ECR_REPOSITORY="stellar-insights-backend"
CONTAINER_NAME="stellar-insights"
CONTAINER_PORT=8080

usage() {
    echo "Usage: $0 <environment> <image-tag>"
    echo ""
    echo "Arguments:"
    echo "  environment   staging or production"
    echo "  image-tag     Docker image tag (e.g., git SHA)"
    echo ""
    echo "Example: $0 staging abc123def"
    exit 1
}

if [ $# -ne 2 ]; then
    usage
fi

ENVIRONMENT="$1"
IMAGE_TAG="$2"

if [[ "$ENVIRONMENT" != "staging" && "$ENVIRONMENT" != "production" ]]; then
    echo "Error: environment must be 'staging' or 'production'"
    exit 1
fi

CLUSTER="stellar-insights-${ENVIRONMENT}"
TASK_FAMILY="stellar-insights-${ENVIRONMENT}"
CODEDEPLOY_APP="stellar-insights-${ENVIRONMENT}"
CODEDEPLOY_GROUP="stellar-insights-${ENVIRONMENT}-dg"

# Get AWS account ID
ACCOUNT_ID=$(aws sts get-caller-identity --query 'Account' --output text)
IMAGE_URI="${ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com/${ECR_REPOSITORY}:${IMAGE_TAG}"

echo "=== Blue-Green Deploy ==="
echo "Environment: ${ENVIRONMENT}"
echo "Image:       ${IMAGE_URI}"
echo "Cluster:     ${CLUSTER}"
echo ""

# Verify image exists in ECR
echo "Verifying ECR image..."
if ! aws ecr describe-images \
    --repository-name "${ECR_REPOSITORY}" \
    --image-ids "imageTag=${IMAGE_TAG}" \
    --region "${AWS_REGION}" > /dev/null 2>&1; then
    echo "Error: Image ${IMAGE_TAG} not found in ECR repository ${ECR_REPOSITORY}"
    exit 1
fi
echo "Image verified."

# Get current task definition
echo "Fetching current task definition..."
aws ecs describe-task-definition \
    --task-definition "${TASK_FAMILY}" \
    --query 'taskDefinition' \
    --output json > /tmp/current-task-def.json

# Update image and register new revision
echo "Registering new task definition..."
jq --arg IMAGE "$IMAGE_URI" --arg CONTAINER "$CONTAINER_NAME" \
    '.containerDefinitions |= map(if .name == $CONTAINER then .image = $IMAGE else . end) |
     del(.taskDefinitionArn, .revision, .status, .requiresAttributes, .compatibilities, .registeredAt, .registeredBy)' \
    /tmp/current-task-def.json > /tmp/new-task-def.json

NEW_TASK_DEF_ARN=$(aws ecs register-task-definition \
    --cli-input-json file:///tmp/new-task-def.json \
    --query 'taskDefinition.taskDefinitionArn' \
    --output text)

echo "New task definition: ${NEW_TASK_DEF_ARN}"

# Create AppSpec
APPSPEC=$(jq -nc \
    --arg TD "$NEW_TASK_DEF_ARN" \
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

# Create deployment
echo "Creating CodeDeploy deployment..."
DEPLOYMENT_ID=$(aws deploy create-deployment \
    --application-name "${CODEDEPLOY_APP}" \
    --deployment-group-name "${CODEDEPLOY_GROUP}" \
    --revision "revisionType=AppSpecContent,appSpecContent={content='${APPSPEC}'}" \
    --description "Manual deploy: ${IMAGE_TAG}" \
    --query 'deploymentId' \
    --output text)

echo "Deployment created: ${DEPLOYMENT_ID}"

# Wait for deployment
echo "Waiting for deployment to complete..."
if aws deploy wait deployment-successful --deployment-id "${DEPLOYMENT_ID}"; then
    echo ""
    echo "=== Deployment successful! ==="
    echo "Deployment ID: ${DEPLOYMENT_ID}"
    echo "Image:         ${IMAGE_URI}"
else
    echo ""
    echo "=== Deployment failed ==="
    echo "Check the deployment in AWS Console or run:"
    echo "  aws deploy get-deployment --deployment-id ${DEPLOYMENT_ID}"
    exit 1
fi
