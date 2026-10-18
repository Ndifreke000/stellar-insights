# PayRaider Kubernetes Deployment Guide

## Architecture Overview

The payraider application is deployed as a containerized microservices architecture on Kubernetes with the following components:

### Core Services
- **Backend API** (`backend/`): Soroban RPC indexer and analytics API
  - Deployment: 2 replicas (configurable via HPA: min 2, max 8 with 70% CPU threshold)
  - Port: 8080 (container), 80 (service ClusterIP)
  - Persistent volume: Required for local state
  - PDB: Minimum 1 available replica during disruptions

- **Frontend** (`frontend/`): Next.js web application
  - Deployment: 2 replicas (configurable via HPA: min 2, max 5 with 80% CPU threshold)
  - Port: 3000 (container), 3000 (service ClusterIP)
  - Served via Ingress with TLS termination
  - PDB: Minimum 1 available replica during disruptions

### Data Layer
- **SQLite Database**: primary data store. There is no separate database
  pod/StatefulSet -- the backend is SQLite-only
  (`docs/adr/0001-sqlite-vs-postgres.md`), so the database is a file on a
  volume mounted directly into the backend Deployment.
  - `backend/pvc.yaml`: 5Gi PersistentVolumeClaim (`payraider-backend-data`),
    `ReadWriteOnce`, mounted at `/data`
  - Backend `replicas: 1`, `strategy: Recreate` (not RollingUpdate) --
    the RWO volume can't be mounted by two pods at once, and SQLite
    permits exactly one writer anyway
  - A `litestream` sidecar container in the same Deployment continuously
    replicates the database file to S3 -- see `docs/backup-system.md`

- **Redis Cache** (`redis/`): In-memory session and cache store
  - StatefulSet: Single-replica
  - Port: 6379 (service)
  - Persistent volume: 5Gi (RDB snapshots)
  - Used by backend for session management and rate limiting

### Networking
- **Ingress** (`ingress/ingress.yaml`): External access
  - NGINX Ingress Controller (nginx.ingress.kubernetes.io/ingress.class)
  - TLS termination with cert-manager (ACME)
  - CORS headers enabled
  - Rate limiting: 100 requests/minute per IP
  - Routing:
    - `/` → frontend service
    - `/api/*` → backend service

- **Network Policy** (`network-policy.yaml`): Microsegmentation
  - Restricts traffic between pods
  - Allows ingress controller to reach frontend/backend
  - Allows backend to reach database and redis
  - Default deny for east-west traffic

- **Service Accounts** (`*/serviceaccount.yaml`): RBAC (role-based access control)
  - Per-service service accounts for least-privilege access
  - Enables pod identity and workload identity integration

### Monitoring
- **Prometheus** (`monitoring/prometheus.yaml`): Metrics scraping
  - Scrape interval: 30s
  - Data retention: 15 days (configurable)
  - ServiceMonitor resources for automatic target discovery
  - Scrape targets: backend, frontend, kube-state-metrics

- **ServiceMonitor** (`monitoring/servicemonitor.yaml`): Prometheus Operator integration
  - Defines scrape targets for backend metrics (port 8080/metrics)
  - Automatic label-based service discovery

## Deployment Prerequisites

### Cluster Requirements
- Kubernetes 1.24+ (tested on 1.25+)
- 3+ worker nodes with:
  - Minimum 4 CPU cores and 8GB RAM per node
  - 20Gi free disk space for stateful volumes
- Storage class supporting PersistentVolumeClaims (RWO):
  - AWS EBS (gp3 recommended)
  - Google Cloud Persistent Disks
  - Azure Managed Disks
  - Local provisioner (development only)

### Required Tools
- `kubectl` CLI (matching cluster version)
- `kustomize` v5.0+ or built-in `kubectl -k` support
- `cert-manager` (for TLS - optional but recommended)
- `helm` (optional, if installing Prometheus Operator)

### Prerequisite Installations
```bash
# Install cert-manager (for automatic TLS)
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml
kubectl wait --for=condition=Available --timeout=300s deployment/cert-manager -n cert-manager

# Install NGINX Ingress Controller (if not already present)
helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx
helm install ingress-nginx ingress-nginx/ingress-nginx \
  --namespace ingress-nginx \
  --create-namespace \
  --set controller.service.type=LoadBalancer
```

## Configuration Management

### Secrets Management
Secrets are externalized and stored outside the repository for security. Required secrets:

1. **Database** (`config/secrets.yaml` - not committed)
   - `database-url`: SQLite file path (`sqlite:///data/payraider.db`) -- not
     network credentials, see `docs/adr/0001-sqlite-vs-postgres.md`

2. **Backend Secrets**
   - `SOROBAN_RPC_URL`: Stellar Soroban RPC endpoint (e.g., https://soroban-testnet.stellar.org)
   - `JWT_SECRET`: JWT signing key (minimum 32 characters)
   - `SESSION_SECRET`: Session encryption key (minimum 32 characters)
   - `REDIS_PASSWORD`: Redis authentication password

3. **TLS Certificate Secrets**
   - Managed by cert-manager with Let's Encrypt (production) or self-signed (development)
   - Secret name: `payraider-tls`

### Environment Configuration
ConfigMaps define environment-specific configuration:

- **Global** (`config/configmap.yaml`):
  - `ENVIRONMENT`: dev/staging/testnet/mainnet
  - `LOG_LEVEL`: debug/info/warn/error
  - `METRICS_ENABLED`: true/false

- **Backend** (`backend/deployment.yaml`):
  - `API_PORT`: Service port
  - `CACHE_TTL`: Cache invalidation timeout (seconds)
  - `BATCH_SIZE`: Indexer batch size

- **Frontend** (`frontend/configmap.yaml`):
  - `NEXT_PUBLIC_API_BASE_URL`: Backend API URL
  - `NEXT_PUBLIC_ANALYTICS_ID`: Analytics tracking ID

### Overlay Configuration
Environment-specific configurations are in `overlays/`:

```
k8s/overlays/
├── dev/           # Development: single replica, small resources
├── staging/       # Staging: 2 replicas, moderate resources, real testnet
├── testnet/       # Testnet: 2 replicas, production-like setup on testnet
├── mainnet/       # Mainnet: HA setup with HPA, production resources
└── production/    # Production: full HA with monitoring (not committed)
```

## Deployment Steps

### 1. Prepare Environment

```bash
# Clone repository
git clone https://github.com/Ndifreke000/payraider.git
cd payraider/k8s

# Create secrets file (not tracked by git)
cat > config/secrets.yaml <<EOF
apiVersion: v1
kind: Secret
metadata:
  name: payraider-secrets
  namespace: payraider
type: Opaque
stringData:
  database-url: "sqlite:///data/payraider.db"
  jwt-secret: "$(openssl rand -base64 48)"
  encryption-key: "$(openssl rand -hex 32)"
  redis-url: "redis://:$(openssl rand -base64 32)@redis:6379"
EOF

# Validate manifests
kubectl kustomize overlays/testnet | kubeval --strict
```

### 2. Deploy to Cluster

```bash
# Select environment (dev/staging/testnet/mainnet)
ENVIRONMENT=testnet

# Apply manifests using kustomize
kubectl apply -k overlays/$ENVIRONMENT

# Verify deployment
kubectl get all -n payraider
kubectl get events -n payraider --sort-by='.lastTimestamp'
```

### 3. Create Secrets

```bash
# Create secrets (see config/secret-template.yaml for the full set)
kubectl create secret generic payraider-secrets \
  --from-literal=database-url="sqlite:///data/payraider.db" \
  --from-literal=jwt-secret="$(openssl rand -base64 48)" \
  --from-literal=encryption-key="$(openssl rand -hex 32)" \
  -n payraider

# Create TLS certificate secret (if not using cert-manager)
kubectl create secret tls payraider-tls \
  --cert=path/to/cert.crt \
  --key=path/to/key.key \
  -n payraider
```

### 4. Wait for Readiness

```bash
# Backend readiness
kubectl rollout status deployment/payraider-backend -n payraider

# Frontend readiness
kubectl rollout status deployment/payraider-frontend -n payraider

# Database readiness
kubectl wait --for=condition=ready pod \
  -l app=payraider,component=database \
  -n payraider \
  --timeout=300s
```

### 5. Verify Deployment

```bash
# Check pod status
kubectl get pods -n payraider

# View logs
kubectl logs -f deployment/payraider-backend -n payraider
kubectl logs -f deployment/payraider-frontend -n payraider

# Test API endpoint
BACKEND_POD=$(kubectl get pod -l app=payraider,component=backend \
  -n payraider -o jsonpath='{.items[0].metadata.name}')
kubectl exec $BACKEND_POD -n payraider -- curl -s http://localhost:8080/health

# Test Ingress
kubectl get ingress -n payraider
# Wait for LoadBalancer IP assignment (check status column)
```

## Scaling and High Availability

### Horizontal Pod Autoscaling
HPA policies are configured for backend and frontend:

```bash
# View HPA status
kubectl get hpa -n payraider

# Manual scaling (temporary override of HPA)
kubectl scale deployment payraider-backend --replicas=5 -n payraider

# Monitor autoscaling
kubectl describe hpa payraider-backend -n payraider
```

Scaling thresholds:
- Backend: 2-8 replicas, scale at 70% CPU utilization
- Frontend: 2-5 replicas, scale at 80% CPU utilization

### Pod Disruption Budgets
PDBs ensure HA during voluntary disruptions (cluster upgrades, drains):

```bash
# Verify PDB configuration
kubectl get pdb -n payraider
kubectl describe pdb payraider-backend -n payraider
```

Constraints:
- Backend: Minimum 1 replica available
- Frontend: Minimum 1 replica available

### Persistent Storage
Database and Redis require persistent volumes:

```bash
# Check PVC status
kubectl get pvc -n payraider

# Resize PVC (for growing databases)
kubectl patch pvc payraider-database-data -p \
  '{"spec":{"resources":{"requests":{"storage":"50Gi"}}}}' \
  -n payraider
```

## Monitoring and Observability

### Prometheus Metrics
Prometheus scrapes metrics from all services via ServiceMonitor:

- Backend: `http://payraider-backend:8080/metrics` (30s scrape interval)
- Frontend: Not instrumented (static serving)

### Querying Metrics

```bash
# Port-forward Prometheus
kubectl port-forward svc/prometheus-server 9090:9090 -n payraider

# Example PromQL queries (visit http://localhost:9090)
# API latency (p95): histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
# Memory usage: container_memory_usage_bytes{pod=~"payraider-backend.*"}
# Request rate: rate(http_requests_total[1m])
```

### Alert Rules
Define alerts in `monitoring/prometheus-rules.yaml`:

```bash
# Apply alert rules
kubectl apply -f monitoring/prometheus-rules.yaml

# View active alerts
kubectl exec prometheus-0 -n payraider -- \
  curl -s http://localhost:9090/api/v1/alerts | jq '.data.alerts'
```

## Rollback and Recovery

### Rolling Back a Deployment

```bash
# Check deployment history
kubectl rollout history deployment/payraider-backend -n payraider

# Rollback to previous version
kubectl rollout undo deployment/payraider-backend -n payraider

# Rollback to specific revision
kubectl rollout undo deployment/payraider-backend --to-revision=2 -n payraider
```

### Database Recovery

No separate database pod to exec into -- the SQLite file lives on the
backend pod's mounted volume. See `docs/backup-system.md` for the full
story (Litestream continuous S3 replication + backup.rs local snapshots).

```bash
# Manual backup (exec into the backend pod)
kubectl exec -it deployment/payraider-backend -n payraider -c backend -- \
  sh -c 'sqlite3 /data/payraider.db ".backup /tmp/backup.db" && cat /tmp/backup.db' \
  > backup-$(date +%s).db

# Restore from a Litestream replica (see docs/backup-system.md for the
# litestream restore command), then copy the restored file onto the pod
kubectl cp ./restored.db payraider/<pod-name>:/data/payraider.db -c backend
```

## Troubleshooting

### Common Issues

1. **Pods stuck in Pending**
   ```bash
   kubectl describe pod <pod-name> -n payraider
   # Check: resource requests vs available nodes, PVC binding
   ```

2. **CrashLoopBackOff**
   ```bash
   kubectl logs <pod-name> -n payraider --previous
   # Check: missing secrets, invalid env vars, disk space
   ```

3. **Ingress not routing traffic**
   ```bash
   kubectl describe ingress payraider -n payraider
   # Check: TLS cert status, backend service health, ingress controller logs
   ```

4. **Database connection errors**
   ```bash
   kubectl exec <backend-pod> -n payraider -- \
     env | grep DATABASE
   # Verify credentials match secrets
   ```

### Debug Logging

```bash
# Enable debug logging in backend
kubectl set env deployment/payraider-backend LOG_LEVEL=debug -n payraider

# Stream logs from all backend replicas
kubectl logs -f -l app=payraider,component=backend -n payraider

# Interactive shell in pod
kubectl exec -it <pod-name> -n payraider -- /bin/sh
```

## Performance Tuning

### Resource Limits
Adjust resource requests/limits in deployment manifests based on workload:

```yaml
resources:
  requests:
    cpu: 500m          # Guaranteed minimum
    memory: 512Mi       # Guaranteed minimum
  limits:
    cpu: 2000m         # Maximum allowed
    memory: 2Gi        # Maximum allowed
```

### Connection Pooling
Backend uses pgbouncer for database connection pooling:

```bash
kubectl set env deployment/payraider-backend \
  DB_POOL_SIZE=20 \
  DB_POOL_MIN_SIZE=5 \
  -n payraider
```

## Security Hardening

### Network Policies
Microsegmentation is enforced via network-policy.yaml:

```bash
# Verify network policy
kubectl get networkpolicy -n payraider
kubectl describe networkpolicy default-deny -n payraider
```

### RBAC
Per-service service accounts follow least-privilege principle:

```bash
# View role bindings
kubectl get rolebindings -n payraider
kubectl describe rolebinding payraider-backend -n payraider
```

### Image Security
Use image scanning and signed images in production:

```bash
# Scan images
trivy image payraider/backend:latest

# Use private registry with authentication
kubectl create secret docker-registry regcred \
  --docker-server=private.registry.com \
  --docker-username=user \
  --docker-password=pass \
  -n payraider
```

## Maintenance

### Regular Tasks

- **Weekly**: Monitor resource utilization, review error logs
- **Monthly**: Backup database, test restore procedure, rotate secrets
- **Quarterly**: Update base images, run security scans, update Kubernetes version

### Updating Applications

```bash
# Update image tags in kustomization.yaml
kubectl kustomize overlays/$ENVIRONMENT | kubectl apply -f -

# Monitor rollout
kubectl rollout status deployment/payraider-backend -n payraider
```

## Support and Documentation

- **Architecture Decisions**: See `docs/architecture.md`
- **Container Images**: Built from `Dockerfile` in project root
- **CI/CD Integration**: `.github/workflows/deploy-k8s.yaml`
- **Terraform IaC**: `terraform/` directory (optional infrastructure provisioning)

## FAQ

**Q: How do I scale the application?**
A: HPA automatically scales based on CPU utilization. For manual control: `kubectl scale deployment payraider-backend --replicas=N -n payraider`

**Q: How do I update environment variables?**
A: Update ConfigMaps in `config/configmap.yaml`, then redeploy or restart pods: `kubectl rollout restart deployment/payraider-backend -n payraider`

**Q: How do I change database password?**
A: Update the secret and restart pods: `kubectl delete secret payraider-secrets` and recreate with new password.

**Q: What's the upgrade process?**
A: Kustomize overlays handle environment-specific configuration. Update image tags and apply new manifests with `kubectl apply -k overlays/$ENVIRONMENT`.

---

**Last updated**: 2026-08-26
**Manifest version**: 1.0
**Tested with Kubernetes**: 1.25+
