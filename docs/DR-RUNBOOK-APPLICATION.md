# Application Disaster Recovery Runbook

**Service:** Backend API (Rust/Axum) & Frontend (Next.js)  
**Priority:** P0 - Critical  
**RTO:** 2 hours  
**RPO:** N/A (Stateless)  
**Last Updated:** 2024

---

## Quick Reference

| Scenario | Procedure | Time Estimate |
|----------|-----------|---------------|
| Service Crash | [Restart Service](#scenario-1-service-crash) | 5 minutes |
| Memory Leak | [Restart with Investigation](#scenario-2-memory-leak) | 15 minutes |
| Bad Deployment | [Rollback](#scenario-3-bad-deployment) | 10 minutes |
| Configuration Error | [Fix Config](#scenario-4-configuration-error) | 20 minutes |

---

## Prerequisites

### Required Access
- [ ] SSH access to application servers
- [ ] Docker/container access
- [ ] Git repository access
- [ ] CI/CD pipeline access
- [ ] Monitoring dashboard access

### Required Tools
```bash
# Verify tools
docker --version
git --version
curl --version
```

---

## Scenario 1: Service Crash

**Symptoms:**
- HTTP 502/503 errors
- Health check failures
- Container/process not running

### Detection

```bash
# Check service status
sudo systemctl status stellar-insights-backend
sudo systemctl status stellar-insights-frontend

# Check Docker containers
docker ps -a | grep stellar-insights

# Check logs
docker logs stellar-insights-backend --tail 100
docker logs stellar-insights-frontend --tail 100
```

### Recovery Steps

#### Step 1: Quick Restart (5 minutes)

```bash
# Backend restart
sudo systemctl restart stellar-insights-backend

# Or if using Docker
docker restart stellar-insights-backend

# Frontend restart
sudo systemctl restart stellar-insights-frontend

# Or if using Docker
docker restart stellar-insights-frontend

# Verify services are running
curl http://localhost:8080/health
curl http://localhost:3000/
```

**Validation:**
```bash
# Check health endpoints
curl -f http://localhost:8080/health || echo "Backend unhealthy"
curl -f http://localhost:3000/ || echo "Frontend unhealthy"

# Check logs for errors
docker logs stellar-insights-backend --tail 50 | grep -i error
docker logs stellar-insights-frontend --tail 50 | grep -i error
```

---

## Scenario 2: Memory Leak

**Symptoms:**
- Increasing memory usage
- OOM killer events
- Slow response times

### Recovery Steps

#### Step 1: Identify Memory Issue

```bash
# Check memory usage
docker stats --no-stream

# Check for OOM events
dmesg | grep -i "out of memory"

# Check container resource limits
docker inspect stellar-insights-backend | grep -A 10 "Memory"
```

#### Step 2: Restart and Monitor

```bash
# Restart service
docker restart stellar-insights-backend

# Monitor memory usage
watch -n 5 'docker stats --no-stream | grep stellar-insights'

# Check for memory leaks in logs
docker logs stellar-insights-backend | grep -i "memory\|heap\|allocation"
```

---

## Scenario 3: Bad Deployment

**Symptoms:**
- Errors after deployment
- Failed health checks
- Increased error rates

### Recovery Steps

#### Step 1: Identify Bad Deployment

```bash
# Check recent deployments
git log --oneline -10

# Check current version
curl http://localhost:8080/version

# Check error rates in monitoring
```

#### Step 2: Rollback (10 minutes)

```bash
# Stop current version
docker stop stellar-insights-backend

# Pull previous version
docker pull stellar-insights/backend:previous-tag

# Start previous version
docker run -d \
  --name stellar-insights-backend \
  --env-file /opt/stellar-insights/.env \
  -p 8080:8080 \
  stellar-insights/backend:previous-tag

# Verify rollback
curl http://localhost:8080/health
```

**Validation:**
```bash
# Check version
curl http://localhost:8080/version

# Monitor error rates
# Check monitoring dashboard

# Test critical endpoints
curl http://localhost:8080/api/anchors
curl http://localhost:8080/api/corridors
```

---

## Scenario 4: Configuration Error

**Symptoms:**
- Service starts but fails to function
- Database connection errors
- External API failures

### Recovery Steps

#### Step 1: Identify Configuration Issue

```bash
# Check environment variables
docker exec stellar-insights-backend env | grep -E "DATABASE|REDIS|STELLAR"

# Check configuration file
docker exec stellar-insights-backend cat /app/.env

# Check logs for config errors
docker logs stellar-insights-backend | grep -i "config\|environment\|variable"
```

#### Step 2: Fix Configuration

```bash
# Update environment file
sudo nano /opt/stellar-insights/.env

# Restart service with new config
docker restart stellar-insights-backend

# Verify configuration
docker exec stellar-insights-backend env | grep DATABASE_URL
```

---

## Common Issues

### Issue: Port Already in Use

```bash
# Find process using port
sudo lsof -i :8080

# Kill process
sudo kill -9 <PID>

# Restart service
docker start stellar-insights-backend
```

### Issue: Database Connection Failed

```bash
# Test database connectivity
docker exec stellar-insights-backend psql -h db-host -U postgres -c "SELECT 1"

# Check database is running
sudo systemctl status postgresql

# Verify connection string
docker exec stellar-insights-backend env | grep DATABASE_URL
```

---

## Post-Recovery Checklist

- [ ] Services running
- [ ] Health checks passing
- [ ] No errors in logs
- [ ] Performance normal
- [ ] Monitoring active
- [ ] Incident documented

---

## Escalation

- **5 minutes:** Notify on-call engineer
- **15 minutes:** Notify Technical Lead
- **30 minutes:** Notify Incident Commander

---

**Last Tested:** [Date]  
**Document Owner:** DevOps Team
