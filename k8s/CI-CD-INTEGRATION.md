# CI/CD Integration Guide

This guide explains how to integrate Kubernetes deployments into your CI/CD pipeline.

## Table of Contents

- [GitHub Actions](#github-actions)
- [GitLab CI](#gitlab-ci)
- [Jenkins](#jenkins)
- [ArgoCD](#argocd)
- [Flux CD](#flux-cd)

## GitHub Actions

### Workflow Example

Create `.github/workflows/deploy.yml`:

```yaml
name: Deploy to Kubernetes

on:
  push:
    branches:
      - main
      - staging
      - develop
  pull_request:
    branches:
      - main

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  validate:
    name: Validate Manifests
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup kubectl
        uses: azure/setup-kubectl@v3
        with:
          version: 'v1.28.0'

      - name: Setup kustomize
        run: |
          curl -s "https://raw.githubusercontent.com/kubernetes-sigs/kustomize/master/hack/install_kustomize.sh" | bash
          sudo mv kustomize /usr/local/bin/

      - name: Validate manifests
        run: |
          cd k8s
          chmod +x scripts/validate.sh
          ./scripts/validate.sh

  build:
    name: Build and Push Images
    runs-on: ubuntu-latest
    needs: validate
    permissions:
      contents: read
      packages: write
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=ref,event=branch
            type=ref,event=pr
            type=semver,pattern={{version}}
            type=semver,pattern={{major}}.{{minor}}
            type=sha

      - name: Build and push backend
        uses: docker/build-push-action@v5
        with:
          context: ./backend
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

      - name: Build and push frontend
        uses: docker/build-push-action@v5
        with:
          context: ./frontend
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  deploy-dev:
    name: Deploy to Development
    runs-on: ubuntu-latest
    needs: build
    if: github.ref == 'refs/heads/develop'
    environment:
      name: development
      url: https://dev.stellar-insights.com
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup kubectl
        uses: azure/setup-kubectl@v3

      - name: Setup kustomize
        run: |
          curl -s "https://raw.githubusercontent.com/kubernetes-sigs/kustomize/master/hack/install_kustomize.sh" | bash
          sudo mv kustomize /usr/local/bin/

      - name: Configure kubectl
        run: |
          echo "${{ secrets.KUBECONFIG_DEV }}" | base64 -d > kubeconfig
          export KUBECONFIG=kubeconfig

      - name: Deploy to dev
        run: |
          cd k8s
          kustomize edit set image stellar-insights/backend:${{ github.sha }}
          kustomize edit set image stellar-insights/frontend:${{ github.sha }}
          kubectl apply -k overlays/dev
          kubectl rollout status deployment/stellar-insights-backend -n stellar-insights-dev --timeout=5m

      - name: Run tests
        run: |
          cd k8s
          chmod +x scripts/test-deployment.sh
          ./scripts/test-deployment.sh stellar-insights-dev

  deploy-staging:
    name: Deploy to Staging
    runs-on: ubuntu-latest
    needs: build
    if: github.ref == 'refs/heads/staging'
    environment:
      name: staging
      url: https://staging.stellar-insights.com
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup kubectl
        uses: azure/setup-kubectl@v3

      - name: Setup kustomize
        run: |
          curl -s "https://raw.githubusercontent.com/kubernetes-sigs/kustomize/master/hack/install_kustomize.sh" | bash
          sudo mv kustomize /usr/local/bin/

      - name: Configure kubectl
        run: |
          echo "${{ secrets.KUBECONFIG_STAGING }}" | base64 -d > kubeconfig
          export KUBECONFIG=kubeconfig

      - name: Deploy to staging
        run: |
          cd k8s
          kustomize edit set image stellar-insights/backend:${{ github.sha }}
          kustomize edit set image stellar-insights/frontend:${{ github.sha }}
          kubectl apply -k overlays/staging
          kubectl rollout status deployment/stellar-insights-backend -n stellar-insights-staging --timeout=5m

  deploy-production:
    name: Deploy to Production
    runs-on: ubuntu-latest
    needs: build
    if: github.ref == 'refs/heads/main'
    environment:
      name: production
      url: https://stellar-insights.com
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup kubectl
        uses: azure/setup-kubectl@v3

      - name: Setup kustomize
        run: |
          curl -s "https://raw.githubusercontent.com/kubernetes-sigs/kustomize/master/hack/install_kustomize.sh" | bash
          sudo mv kustomize /usr/local/bin/

      - name: Configure kubectl
        run: |
          echo "${{ secrets.KUBECONFIG_PROD }}" | base64 -d > kubeconfig
          export KUBECONFIG=kubeconfig

      - name: Deploy to production
        run: |
          cd k8s
          kustomize edit set image stellar-insights/backend:${{ github.sha }}
          kustomize edit set image stellar-insights/frontend:${{ github.sha }}
          kubectl apply -k overlays/production
          kubectl rollout status deployment/stellar-insights-backend -n stellar-insights --timeout=10m

      - name: Run smoke tests
        run: |
          cd k8s
          chmod +x scripts/test-deployment.sh
          ./scripts/test-deployment.sh stellar-insights

      - name: Notify on failure
        if: failure()
        run: |
          # Send notification (Slack, email, etc.)
          echo "Deployment failed!"
```

### Required Secrets

Configure these secrets in GitHub repository settings:

- `KUBECONFIG_DEV`: Base64-encoded kubeconfig for dev cluster
- `KUBECONFIG_STAGING`: Base64-encoded kubeconfig for staging cluster
- `KUBECONFIG_PROD`: Base64-encoded kubeconfig for production cluster

```bash
# Generate base64-encoded kubeconfig
cat ~/.kube/config | base64 -w 0
```

## GitLab CI

### Pipeline Example

Create `.gitlab-ci.yml`:

```yaml
stages:
  - validate
  - build
  - deploy

variables:
  DOCKER_DRIVER: overlay2
  DOCKER_TLS_CERTDIR: "/certs"
  REGISTRY: registry.gitlab.com
  IMAGE_TAG: $CI_REGISTRY_IMAGE:$CI_COMMIT_SHORT_SHA

validate:
  stage: validate
  image: alpine/k8s:1.28.0
  script:
    - cd k8s
    - chmod +x scripts/validate.sh
    - ./scripts/validate.sh
  only:
    - merge_requests
    - main
    - staging
    - develop

build-backend:
  stage: build
  image: docker:24
  services:
    - docker:24-dind
  before_script:
    - docker login -u $CI_REGISTRY_USER -p $CI_REGISTRY_PASSWORD $CI_REGISTRY
  script:
    - cd backend
    - docker build -t $REGISTRY/$CI_PROJECT_PATH/backend:$CI_COMMIT_SHORT_SHA .
    - docker push $REGISTRY/$CI_PROJECT_PATH/backend:$CI_COMMIT_SHORT_SHA
  only:
    - main
    - staging
    - develop

build-frontend:
  stage: build
  image: docker:24
  services:
    - docker:24-dind
  before_script:
    - docker login -u $CI_REGISTRY_USER -p $CI_REGISTRY_PASSWORD $CI_REGISTRY
  script:
    - cd frontend
    - docker build -t $REGISTRY/$CI_PROJECT_PATH/frontend:$CI_COMMIT_SHORT_SHA .
    - docker push $REGISTRY/$CI_PROJECT_PATH/frontend:$CI_COMMIT_SHORT_SHA
  only:
    - main
    - staging
    - develop

deploy-dev:
  stage: deploy
  image: alpine/k8s:1.28.0
  before_script:
    - echo "$KUBECONFIG_DEV" | base64 -d > kubeconfig
    - export KUBECONFIG=kubeconfig
  script:
    - cd k8s
    - kustomize edit set image stellar-insights/backend=$REGISTRY/$CI_PROJECT_PATH/backend:$CI_COMMIT_SHORT_SHA
    - kustomize edit set image stellar-insights/frontend=$REGISTRY/$CI_PROJECT_PATH/frontend:$CI_COMMIT_SHORT_SHA
    - kubectl apply -k overlays/dev
    - kubectl rollout status deployment/stellar-insights-backend -n stellar-insights-dev --timeout=5m
  environment:
    name: development
    url: https://dev.stellar-insights.com
  only:
    - develop

deploy-staging:
  stage: deploy
  image: alpine/k8s:1.28.0
  before_script:
    - echo "$KUBECONFIG_STAGING" | base64 -d > kubeconfig
    - export KUBECONFIG=kubeconfig
  script:
    - cd k8s
    - kustomize edit set image stellar-insights/backend=$REGISTRY/$CI_PROJECT_PATH/backend:$CI_COMMIT_SHORT_SHA
    - kustomize edit set image stellar-insights/frontend=$REGISTRY/$CI_PROJECT_PATH/frontend:$CI_COMMIT_SHORT_SHA
    - kubectl apply -k overlays/staging
    - kubectl rollout status deployment/stellar-insights-backend -n stellar-insights-staging --timeout=5m
  environment:
    name: staging
    url: https://staging.stellar-insights.com
  only:
    - staging

deploy-production:
  stage: deploy
  image: alpine/k8s:1.28.0
  before_script:
    - echo "$KUBECONFIG_PROD" | base64 -d > kubeconfig
    - export KUBECONFIG=kubeconfig
  script:
    - cd k8s
    - kustomize edit set image stellar-insights/backend=$REGISTRY/$CI_PROJECT_PATH/backend:$CI_COMMIT_SHORT_SHA
    - kustomize edit set image stellar-insights/frontend=$REGISTRY/$CI_PROJECT_PATH/frontend:$CI_COMMIT_SHORT_SHA
    - kubectl apply -k overlays/production
    - kubectl rollout status deployment/stellar-insights-backend -n stellar-insights --timeout=10m
    - chmod +x scripts/test-deployment.sh
    - ./scripts/test-deployment.sh stellar-insights
  environment:
    name: production
    url: https://stellar-insights.com
  when: manual
  only:
    - main
```

## Jenkins

### Jenkinsfile Example

```groovy
pipeline {
    agent any
    
    environment {
        REGISTRY = 'docker.io'
        IMAGE_NAME = 'stellar-insights'
        KUBECONFIG = credentials('kubeconfig-prod')
    }
    
    stages {
        stage('Validate') {
            steps {
                sh '''
                    cd k8s
                    chmod +x scripts/validate.sh
                    ./scripts/validate.sh
                '''
            }
        }
        
        stage('Build') {
            parallel {
                stage('Build Backend') {
                    steps {
                        sh '''
                            cd backend
                            docker build -t ${REGISTRY}/${IMAGE_NAME}/backend:${BUILD_NUMBER} .
                            docker push ${REGISTRY}/${IMAGE_NAME}/backend:${BUILD_NUMBER}
                        '''
                    }
                }
                stage('Build Frontend') {
                    steps {
                        sh '''
                            cd frontend
                            docker build -t ${REGISTRY}/${IMAGE_NAME}/frontend:${BUILD_NUMBER} .
                            docker push ${REGISTRY}/${IMAGE_NAME}/frontend:${BUILD_NUMBER}
                        '''
                    }
                }
            }
        }
        
        stage('Deploy to Dev') {
            when {
                branch 'develop'
            }
            steps {
                sh '''
                    cd k8s
                    kustomize edit set image stellar-insights/backend=${REGISTRY}/${IMAGE_NAME}/backend:${BUILD_NUMBER}
                    kustomize edit set image stellar-insights/frontend=${REGISTRY}/${IMAGE_NAME}/frontend:${BUILD_NUMBER}
                    kubectl apply -k overlays/dev
                    kubectl rollout status deployment/stellar-insights-backend -n stellar-insights-dev --timeout=5m
                '''
            }
        }
        
        stage('Deploy to Production') {
            when {
                branch 'main'
            }
            steps {
                input message: 'Deploy to production?', ok: 'Deploy'
                sh '''
                    cd k8s
                    kustomize edit set image stellar-insights/backend=${REGISTRY}/${IMAGE_NAME}/backend:${BUILD_NUMBER}
                    kustomize edit set image stellar-insights/frontend=${REGISTRY}/${IMAGE_NAME}/frontend:${BUILD_NUMBER}
                    kubectl apply -k overlays/production
                    kubectl rollout status deployment/stellar-insights-backend -n stellar-insights --timeout=10m
                    chmod +x scripts/test-deployment.sh
                    ./scripts/test-deployment.sh stellar-insights
                '''
            }
        }
    }
    
    post {
        failure {
            // Send notification
            echo 'Deployment failed!'
        }
        success {
            echo 'Deployment successful!'
        }
    }
}
```

## ArgoCD

### Application Manifest

Create `argocd/application.yaml`:

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: stellar-insights
  namespace: argocd
spec:
  project: default
  
  source:
    repoURL: https://github.com/your-org/stellar-insights.git
    targetRevision: main
    path: k8s/overlays/production
  
  destination:
    server: https://kubernetes.default.svc
    namespace: stellar-insights
  
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
      allowEmpty: false
    syncOptions:
    - CreateNamespace=true
    - PrunePropagationPolicy=foreground
    - PruneLast=true
    retry:
      limit: 5
      backoff:
        duration: 5s
        factor: 2
        maxDuration: 3m
  
  ignoreDifferences:
  - group: apps
    kind: Deployment
    jsonPointers:
    - /spec/replicas  # Ignore HPA-managed replicas
```

### Deploy with ArgoCD

```bash
# Install ArgoCD
kubectl create namespace argocd
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml

# Create application
kubectl apply -f argocd/application.yaml

# Access ArgoCD UI
kubectl port-forward svc/argocd-server -n argocd 8080:443

# Get admin password
kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath="{.data.password}" | base64 -d
```

## Flux CD

### Bootstrap Flux

```bash
# Install Flux CLI
curl -s https://fluxcd.io/install.sh | sudo bash

# Bootstrap Flux
flux bootstrap github \
  --owner=your-org \
  --repository=stellar-insights \
  --branch=main \
  --path=./clusters/production \
  --personal
```

### GitRepository and Kustomization

Create `clusters/production/stellar-insights.yaml`:

```yaml
apiVersion: source.toolkit.fluxcd.io/v1
kind: GitRepository
metadata:
  name: stellar-insights
  namespace: flux-system
spec:
  interval: 1m
  url: https://github.com/your-org/stellar-insights
  ref:
    branch: main
---
apiVersion: kustomize.toolkit.fluxcd.io/v1
kind: Kustomization
metadata:
  name: stellar-insights
  namespace: flux-system
spec:
  interval: 10m
  path: ./k8s/overlays/production
  prune: true
  sourceRef:
    kind: GitRepository
    name: stellar-insights
  healthChecks:
  - apiVersion: apps/v1
    kind: Deployment
    name: stellar-insights-backend
    namespace: stellar-insights
  - apiVersion: apps/v1
    kind: Deployment
    name: stellar-insights-frontend
    namespace: stellar-insights
```

## Best Practices

### 1. Image Tagging

- Use semantic versioning for production
- Use commit SHA for staging/dev
- Never use `latest` tag

### 2. Secrets Management

- Use external secret management (Sealed Secrets, External Secrets Operator)
- Never commit secrets to Git
- Rotate secrets regularly

### 3. Deployment Strategy

- Always validate before deploying
- Use canary or blue-green deployments for production
- Implement automated rollback on failure

### 4. Monitoring

- Monitor deployment progress
- Set up alerts for failed deployments
- Track deployment metrics

### 5. Testing

- Run validation tests before deployment
- Perform smoke tests after deployment
- Implement automated integration tests

---

**Last Updated**: 2026-02-23  
**Version**: 1.0.0
