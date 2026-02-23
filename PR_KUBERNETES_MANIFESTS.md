# Production-Ready Kubernetes Deployment Manifests

## Overview

This PR implements complete, production-ready Kubernetes deployment manifests for the Stellar Insights application, following best practices for high availability, security, and observability.

Closes #325

## What's Included

### Core Components

✅ **Backend Deployment**
- 3 replicas with rolling updates (maxUnavailable: 0 for zero downtime)
- Init container for database migrations
- Comprehensive health probes (liveness, readiness, startup)
- Resource requests and limits defined
- HPA: 3-10 replicas based on CPU/memory
- PDB: minAvailable 2 for high availability

✅ **Frontend Deployment**
- 2 replicas with rolling updates
- Health probes configured
- Resource requests and limits
- HPA: 2-6 replicas
- PDB: minAvailable 1

✅ **PostgreSQL StatefulSet**
- Persistent storage with 50Gi volume
- Health checks and resource limits
- Custom configuration via ConfigMap
- Headless service for stable network identity

✅ **Redis Deployment**
- In-memory cache with persistence
- Custom configuration (LRU eviction, 256MB max memory)
- Health checks and resource limits

✅ **Ingress Configuration**
- NGINX ingress with TLS/SSL
- cert-manager integration for Let's Encrypt
- Rate limiting (100 RPS, 50 connections)
- Security headers (X-Frame-Options, CSP, etc.)
- CORS configuration

### Security Features

🔒 **Pod Security**
- Non-root user (UID 1000)
- Read-only root filesystem
- Drop ALL capabilities
- Seccomp profile (RuntimeDefault)

🔒 **Network Security**
- NetworkPolicies for pod-to-pod isolation
- Backend can only access database and Redis
- Database and Redis only accessible from backend
- Ingress-only external access

🔒 **RBAC**
- Least-privilege ServiceAccounts for each component
- No cluster-admin permissions
- Role-based access control

🔒 **Secrets Management**
- Secret template with clear warnings
- Examples for Sealed Secrets and External Secrets Operator
- No hardcoded secrets in manifests

### Multi-Environment Support

📦 **Kustomize Overlays**
- **Development**: 1 backend, 1 frontend, testnet, debug logging
- **Staging**: 2 backend, 2 frontend, testnet, info logging
- **Production**: 3 backend, 2 frontend, mainnet, info logging

### Observability

📊 **Monitoring**
- Prometheus ServiceMonitor for metrics scraping
- Metrics endpoint on port 9090
- Health check endpoint on port 8080
- Pod annotations for Prometheus discovery

### Automation & Validation

🛠️ **Scripts**
- `scripts/validate.sh`: Validate YAML syntax, check for deprecated APIs, security issues
- `scripts/deploy.sh`: Automated deployment with rollout status monitoring
- `scripts/test-deployment.sh`: Comprehensive post-deployment testing

📚 **Documentation**
- `README.md`: Complete deployment guide with troubleshooting
- `CI-CD-INTEGRATION.md`: Integration examples for GitHub Actions, GitLab CI, Jenkins, ArgoCD, Flux CD

## File Structure

```
k8s/
├── README.md                          # Main documentation
├── CI-CD-INTEGRATION.md               # CI/CD integration guide
├── namespace.yaml                     # Namespace definition
├── kustomization.yaml                 # Base kustomization
├── backend/
│   ├── deployment.yaml                # Backend deployment
│   ├── service.yaml                   # Backend service
│   ├── hpa.yaml                       # Horizontal Pod Autoscaler
│   ├── pdb.yaml                       # Pod Disruption Budget
│   └── serviceaccount.yaml            # RBAC ServiceAccount
├── frontend/
│   ├── deployment.yaml                # Frontend deployment
│   ├── service.yaml                   # Frontend service
│   ├── hpa.yaml                       # Horizontal Pod Autoscaler
│   ├── pdb.yaml                       # Pod Disruption Budget
│   ├── configmap.yaml                 # Frontend configuration
│   └── serviceaccount.yaml            # RBAC ServiceAccount
├── database/
│   ├── statefulset.yaml               # PostgreSQL StatefulSet
│   └── service.yaml                   # Database service
├── redis/
│   ├── deployment.yaml                # Redis deployment
│   └── service.yaml                   # Redis service
├── config/
│   ├── configmap.yaml                 # Application configuration
│   └── secret-template.yaml           # Secret template (DO NOT commit actual secrets)
├── ingress/
│   └── ingress.yaml                   # NGINX Ingress with TLS
├── monitoring/
│   └── servicemonitor.yaml            # Prometheus ServiceMonitor
├── network-policy.yaml                # NetworkPolicies for isolation
├── overlays/
│   ├── dev/
│   │   ├── kustomization.yaml
│   │   ├── backend-replicas.yaml
│   │   ├── frontend-replicas.yaml
│   │   └── configmap-patch.yaml
│   ├── staging/
│   │   ├── kustomization.yaml
│   │   ├── backend-replicas.yaml
│   │   └── frontend-replicas.yaml
│   └── production/
│       └── kustomization.yaml
└── scripts/
    ├── validate.sh                    # Validation script
    ├── deploy.sh                      # Deployment script
    └── test-deployment.sh             # Testing script
```

## Deployment Instructions

### Prerequisites

1. Kubernetes cluster (v1.25+)
2. kubectl and kustomize installed
3. Storage class configured
4. Ingress controller (NGINX)
5. cert-manager (optional, for TLS)

### Quick Start

```bash
# 1. Create secrets
JWT_SECRET=$(openssl rand -base64 48)
ENCRYPTION_KEY=$(openssl rand -hex 32)
POSTGRES_PASSWORD=$(openssl rand -base64 32)

kubectl create secret generic stellar-insights-secrets \
  --from-literal=database-url="postgresql://postgres:${POSTGRES_PASSWORD}@postgresql:5432/stellar_insights" \
  --from-literal=redis-url="redis://redis:6379" \
  --from-literal=jwt-secret="${JWT_SECRET}" \
  --from-literal=encryption-key="${ENCRYPTION_KEY}" \
  --from-literal=postgres-user="postgres" \
  --from-literal=postgres-password="${POSTGRES_PASSWORD}" \
  --namespace=stellar-insights

# 2. Validate manifests
cd k8s
./scripts/validate.sh

# 3. Deploy to production
./scripts/deploy.sh production

# 4. Verify deployment
./scripts/test-deployment.sh stellar-insights
```

## Validation Checklist

- [x] All manifests use stable API versions (apps/v1, networking.k8s.io/v1, etc.)
- [x] No deprecated APIs used
- [x] Resource requests and limits defined for all containers
- [x] Security contexts configured (non-root, read-only filesystem)
- [x] Health probes configured (liveness, readiness, startup)
- [x] Rolling update strategy with zero downtime
- [x] HPA configured for auto-scaling
- [x] PDB configured for high availability
- [x] NetworkPolicies for pod isolation
- [x] RBAC with least-privilege access
- [x] No hardcoded secrets
- [x] TLS/SSL configured
- [x] Monitoring integration (Prometheus)
- [x] Multi-environment support
- [x] Validation scripts pass
- [x] Documentation complete

## Testing

### Validation Tests

```bash
cd k8s
./scripts/validate.sh
```

Checks:
- YAML syntax validation
- Deprecated API detection
- Hardcoded secret detection
- Resource limits verification
- Security context verification

### Deployment Tests

```bash
cd k8s
./scripts/test-deployment.sh stellar-insights
```

Checks:
- Namespace exists
- All pods running
- All pods ready
- Services configured
- Ingress configured
- Backend health endpoint responding
- HPA configured
- PDB configured
- Resource usage

### Manual Testing

```bash
# Check pods
kubectl get pods -n stellar-insights

# Check services
kubectl get svc -n stellar-insights

# Check ingress
kubectl get ingress -n stellar-insights

# Test backend health
kubectl exec -n stellar-insights deployment/stellar-insights-backend -- \
  wget -q -O- http://localhost:8080/health

# Check logs
kubectl logs -f deployment/stellar-insights-backend -n stellar-insights

# Test scaling
kubectl get hpa -n stellar-insights --watch
```

## Security Considerations

1. **Secrets**: Use external secret management (Sealed Secrets, External Secrets Operator, Vault)
2. **Images**: Scan images for vulnerabilities before deployment
3. **RBAC**: Review and audit ServiceAccount permissions regularly
4. **Network**: NetworkPolicies enforce least-privilege network access
5. **TLS**: cert-manager automates certificate management
6. **Updates**: Keep Kubernetes and dependencies up to date

## Performance Considerations

1. **Scaling**: HPA automatically scales based on CPU/memory
2. **Resources**: Requests and limits prevent resource contention
3. **Caching**: Redis improves response times
4. **Database**: PostgreSQL configured with connection pooling
5. **CDN**: Consider adding CDN for frontend assets

## Rollback Plan

```bash
# Rollback to previous version
kubectl rollout undo deployment/stellar-insights-backend -n stellar-insights

# Rollback to specific revision
kubectl rollout undo deployment/stellar-insights-backend --to-revision=2 -n stellar-insights

# Check rollout history
kubectl rollout history deployment/stellar-insights-backend -n stellar-insights
```

## Monitoring & Alerts

Configure alerts for:
- Pod restarts
- High CPU/memory usage
- Failed health checks
- High error rates
- Database connection issues
- HPA scaling events

## Next Steps

1. Review and merge this PR
2. Set up CI/CD pipeline (see `CI-CD-INTEGRATION.md`)
3. Configure external secret management
4. Set up monitoring dashboards
5. Configure alerts
6. Perform load testing
7. Document runbooks for common issues

## Related Issues

Closes #325

## Checklist

- [x] All manifests created and validated
- [x] Security best practices implemented
- [x] Multi-environment support configured
- [x] Scripts created and tested
- [x] Documentation complete
- [x] CI/CD integration examples provided
- [x] No hardcoded secrets
- [x] All files committed and pushed

---

**Ready for Review** ✅

Please review the manifests, documentation, and deployment scripts. Test in a development environment before deploying to production.
