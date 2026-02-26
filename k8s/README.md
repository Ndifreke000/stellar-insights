# Stellar Insights - Kubernetes Deployment

Production-ready Kubernetes manifests for deploying Stellar Insights application with high availability, security, and observability.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Deployment](#deployment)
- [Validation](#validation)
- [Monitoring](#monitoring)
- [Security](#security)
- [Troubleshooting](#troubleshooting)

## Overview

This directory contains complete Kubernetes manifests for deploying Stellar Insights:

- **Backend API**: Rust-based API server with 3 replicas
- **Frontend**: Web application with 2 replicas
- **PostgreSQL**: Stateful database with persistent storage
- **Redis**: In-memory cache for performance
- **Ingress**: NGINX ingress with TLS/SSL
- **Monitoring**: Prometheus ServiceMonitor
- **Security**: NetworkPolicies, RBAC, SecurityContexts

### Key Features

✅ Zero-downtime rolling updates  
✅ Horizontal Pod Autoscaling (HPA)  
✅ Pod Disruption Budgets (PDB)  
✅ Comprehensive health probes  
✅ Resource requests and limits  
✅ Security best practices (non-root, read-only filesystem)  
✅ Network policies for isolation  
✅ Multi-environment support (dev/staging/production)  
✅ Kustomize for configuration management  

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Ingress                              │
│              (NGINX + TLS + Rate Limiting)                   │
└────────────────┬────────────────────────┬───────────────────┘
                 │                        │
        ┌────────▼────────┐      ┌───────▼────────┐
        │    Frontend     │      │    Backend     │
        │   (2 replicas)  │      │  (3 replicas)  │
        └─────────────────┘      └────┬──────┬────┘
                                      │      │
                            ┌─────────▼──┐ ┌─▼──────┐
                            │ PostgreSQL │ │ Redis  │
                            │(StatefulSet│ │        │
                            └────────────┘ └────────┘
```

## Prerequisites

### Required Tools

- **kubectl** (v1.25+): Kubernetes CLI
- **kustomize** (v4.5+): Configuration management
- **helm** (v3.0+): Optional, for cert-manager

### Cluster Requirements

- Kubernetes v1.25 or higher
- Storage class for persistent volumes
- Ingress controller (NGINX recommended)
- cert-manager for TLS certificates (optional)
- Metrics server for HPA (optional)
- Prometheus operator for monitoring (optional)

### Installation

```bash
# Install kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
chmod +x kubectl
sudo mv kubectl /usr/local/bin/

# Install kustomize
curl -s "https://raw.githubusercontent.com/kubernetes-sigs/kustomize/master/hack/install_kustomize.sh" | bash
sudo mv kustomize /usr/local/bin/

# Install cert-manager (optional)
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml
```

## Quick Start

### 1. Create Secrets

**IMPORTANT**: Never commit actual secrets to version control!

```bash
# Generate secure secrets
JWT_SECRET=$(openssl rand -base64 48)
ENCRYPTION_KEY=$(openssl rand -hex 32)
POSTGRES_PASSWORD=$(openssl rand -base64 32)

# Create secret
kubectl create secret generic stellar-insights-secrets \
  --from-literal=database-url="postgresql://postgres:${POSTGRES_PASSWORD}@postgresql:5432/stellar_insights" \
  --from-literal=redis-url="redis://redis:6379" \
  --from-literal=jwt-secret="${JWT_SECRET}" \
  --from-literal=encryption-key="${ENCRYPTION_KEY}" \
  --from-literal=postgres-user="postgres" \
  --from-literal=postgres-password="${POSTGRES_PASSWORD}" \
  --namespace=stellar-insights
```

### 2. Deploy to Production

```bash
# Navigate to k8s directory
cd k8s

# Validate manifests
./scripts/validate.sh

# Deploy
./scripts/deploy.sh production

# Or manually with kubectl
kubectl apply -k overlays/production
```

### 3. Verify Deployment

```bash
# Run tests
./scripts/test-deployment.sh stellar-insights

# Check pods
kubectl get pods -n stellar-insights

# Check services
kubectl get svc -n stellar-insights

# Check ingress
kubectl get ingress -n stellar-insights
```

## Configuration

### Environment Variables

Configuration is managed through ConfigMaps and Secrets:

**ConfigMap** (`config/configmap.yaml`):
- Stellar network settings
- Logging configuration
- CORS origins
- Database pool settings
- Admin IP whitelist

**Secret** (`config/secret-template.yaml`):
- Database credentials
- Redis URL
- JWT secret
- Encryption key

### Multi-Environment Setup

Three environments are supported:

#### Development
```bash
kubectl apply -k overlays/dev
```
- 1 backend replica
- 1 frontend replica
- Testnet configuration
- Debug logging

#### Staging
```bash
kubectl apply -k overlays/staging
```
- 2 backend replicas
- 2 frontend replicas
- Testnet configuration
- Info logging

#### Production
```bash
kubectl apply -k overlays/production
```
- 3 backend replicas
- 2 frontend replicas
- Mainnet configuration
- Info logging

### Customization

Edit `overlays/<env>/kustomization.yaml` to customize:

```yaml
# Change image tags
images:
- name: stellar-insights/backend
  newTag: v1.2.3

# Override configuration
configMapGenerator:
- name: stellar-insights-config
  behavior: merge
  literals:
  - rust-log=debug
```

## Deployment

### Initial Deployment

```bash
# 1. Create namespace
kubectl apply -f namespace.yaml

# 2. Create secrets (see Quick Start)

# 3. Deploy all resources
kubectl apply -k overlays/production

# 4. Wait for rollout
kubectl rollout status deployment/stellar-insights-backend -n stellar-insights
kubectl rollout status deployment/stellar-insights-frontend -n stellar-insights
```

### Update Deployment

```bash
# Update image tag
cd overlays/production
kustomize edit set image stellar-insights/backend:v1.2.3

# Apply changes
kubectl apply -k overlays/production

# Monitor rollout
kubectl rollout status deployment/stellar-insights-backend -n stellar-insights
```

### Rollback

```bash
# Rollback to previous version
kubectl rollout undo deployment/stellar-insights-backend -n stellar-insights

# Rollback to specific revision
kubectl rollout undo deployment/stellar-insights-backend --to-revision=2 -n stellar-insights

# Check rollout history
kubectl rollout history deployment/stellar-insights-backend -n stellar-insights
```

## Validation

### Pre-Deployment Validation

```bash
# Validate YAML syntax
./scripts/validate.sh

# Dry-run deployment
kubectl apply --dry-run=client -k overlays/production

# Check for deprecated APIs
kubectl apply --dry-run=server -k overlays/production
```

### Post-Deployment Testing

```bash
# Run comprehensive tests
./scripts/test-deployment.sh stellar-insights

# Manual checks
kubectl get pods -n stellar-insights
kubectl get svc -n stellar-insights
kubectl get ingress -n stellar-insights

# Check pod logs
kubectl logs -f deployment/stellar-insights-backend -n stellar-insights

# Test health endpoint
kubectl exec -n stellar-insights deployment/stellar-insights-backend -- \
  wget -q -O- http://localhost:8080/health
```

### Scaling Tests

```bash
# Test HPA
kubectl get hpa -n stellar-insights

# Generate load (example)
kubectl run -i --tty load-generator --rm --image=busybox --restart=Never -- \
  /bin/sh -c "while sleep 0.01; do wget -q -O- http://stellar-insights-backend:8080/health; done"

# Watch scaling
kubectl get hpa -n stellar-insights --watch
```

## Monitoring

### Prometheus Integration

ServiceMonitor is configured for Prometheus Operator:

```yaml
# monitoring/servicemonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: stellar-insights-backend
spec:
  endpoints:
  - port: metrics
    interval: 30s
    path: /metrics
```

### Metrics Endpoints

- Backend metrics: `http://stellar-insights-backend:9090/metrics`
- Health check: `http://stellar-insights-backend:8080/health`

### Grafana Dashboards

Import dashboards for:
- Kubernetes cluster overview
- Application metrics
- Database performance
- Redis cache statistics

### Alerts

Configure alerts for:
- Pod restarts
- High CPU/memory usage
- Failed health checks
- High error rates
- Database connection issues

## Security

### Security Features

1. **Pod Security**
   - Non-root user (UID 1000)
   - Read-only root filesystem
   - Drop all capabilities
   - Seccomp profile

2. **Network Security**
   - NetworkPolicies for isolation
   - Ingress-only external access
   - Internal service communication

3. **RBAC**
   - Least-privilege ServiceAccounts
   - Role-based access control
   - No cluster-admin permissions

4. **Secrets Management**
   - External secret management recommended
   - Sealed Secrets or External Secrets Operator
   - No secrets in version control

5. **TLS/SSL**
   - cert-manager for automatic certificates
   - Let's Encrypt integration
   - Force HTTPS redirect

### Security Best Practices

```bash
# Scan images for vulnerabilities
trivy image stellar-insights/backend:latest

# Check for security issues
kubectl auth can-i --list --namespace=stellar-insights

# Audit RBAC
kubectl get rolebindings,clusterrolebindings -n stellar-insights

# Review network policies
kubectl get networkpolicies -n stellar-insights
```

## Troubleshooting

### Common Issues

#### Pods Not Starting

```bash
# Check pod status
kubectl describe pod <pod-name> -n stellar-insights

# Check logs
kubectl logs <pod-name> -n stellar-insights

# Check events
kubectl get events -n stellar-insights --sort-by='.lastTimestamp'
```

#### Database Connection Issues

```bash
# Check database pod
kubectl get pod -l component=database -n stellar-insights

# Test database connection
kubectl exec -it <backend-pod> -n stellar-insights -- \
  psql $DATABASE_URL -c "SELECT 1"

# Check database logs
kubectl logs -l component=database -n stellar-insights
```

#### Ingress Not Working

```bash
# Check ingress status
kubectl describe ingress stellar-insights -n stellar-insights

# Check ingress controller logs
kubectl logs -n ingress-nginx -l app.kubernetes.io/component=controller

# Test internal service
kubectl run -it --rm debug --image=curlimages/curl --restart=Never -- \
  curl http://stellar-insights-backend.stellar-insights.svc.cluster.local:8080/health
```

#### HPA Not Scaling

```bash
# Check metrics server
kubectl top nodes
kubectl top pods -n stellar-insights

# Check HPA status
kubectl describe hpa stellar-insights-backend -n stellar-insights

# Check HPA events
kubectl get events -n stellar-insights | grep HorizontalPodAutoscaler
```

### Debug Commands

```bash
# Get all resources
kubectl get all -n stellar-insights

# Describe deployment
kubectl describe deployment stellar-insights-backend -n stellar-insights

# Check resource usage
kubectl top pods -n stellar-insights

# Port forward for local testing
kubectl port-forward svc/stellar-insights-backend 8080:8080 -n stellar-insights

# Execute commands in pod
kubectl exec -it <pod-name> -n stellar-insights -- /bin/sh

# View logs from all replicas
kubectl logs -l component=backend -n stellar-insights --tail=100 -f
```

### Performance Tuning

```bash
# Adjust resource limits
kubectl set resources deployment stellar-insights-backend \
  --limits=cpu=2000m,memory=2Gi \
  --requests=cpu=500m,memory=1Gi \
  -n stellar-insights

# Scale manually
kubectl scale deployment stellar-insights-backend --replicas=5 -n stellar-insights

# Update HPA
kubectl autoscale deployment stellar-insights-backend \
  --min=3 --max=15 --cpu-percent=70 \
  -n stellar-insights
```

## Additional Resources

- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Kustomize Documentation](https://kustomize.io/)
- [NGINX Ingress Controller](https://kubernetes.github.io/ingress-nginx/)
- [cert-manager Documentation](https://cert-manager.io/docs/)
- [Prometheus Operator](https://prometheus-operator.dev/)

## Support

For issues or questions:
1. Check the troubleshooting section
2. Review pod logs and events
3. Consult the disaster recovery plan
4. Contact the DevOps team

---

**Last Updated**: 2026-02-23  
**Version**: 1.0.0  
**Maintained By**: DevOps Team
