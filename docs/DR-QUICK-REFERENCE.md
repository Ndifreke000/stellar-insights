# Disaster Recovery Quick Reference Guide

**Emergency Contact:** devops@stellar-insights.com  
**On-Call:** [PagerDuty/Phone Number]

---

## 🚨 Emergency Response

### Step 1: Assess (2 minutes)
```bash
# Check service status
curl http://localhost:8080/health
docker ps | grep stellar-insights
sudo systemctl status postgresql
```

### Step 2: Declare Incident (3 minutes)
- Determine severity: SEV-1/2/3/4
- Notify team via Slack #incidents
- Assign Incident Commander

### Step 3: Execute Runbook
- Find appropriate runbook below
- Follow procedures step-by-step
- Document actions taken

---

## 📚 Runbook Quick Links

| Scenario | Runbook | RTO | Priority |
|----------|---------|-----|----------|
| **Database Down** | [DR-RUNBOOK-DATABASE.md](./DR-RUNBOOK-DATABASE.md) | 1 hour | P0 |
| **API Outage** | [DR-RUNBOOK-APPLICATION.md](./DR-RUNBOOK-APPLICATION.md) | 2 hours | P0 |
| **Security Breach** | [DR-RUNBOOK-SECURITY.md](./DR-RUNBOOK-SECURITY.md) | Varies | P0 |
| **Infrastructure Loss** | [DR-RUNBOOK-INFRASTRUCTURE.md](./DR-RUNBOOK-INFRASTRUCTURE.md) | 4 hours | P0 |
| **Third-Party Outage** | [DR-RUNBOOK-THIRD-PARTY.md](./DR-RUNBOOK-THIRD-PARTY.md) | 4 hours | P1 |
| **Data Loss** | [DR-RUNBOOK-DATA-LOSS.md](./DR-RUNBOOK-DATA-LOSS.md) | 4 hours | P0 |

---

## 🔥 Common Scenarios

### Database Crash
```bash
# Quick restart
sudo systemctl restart postgresql
pg_isready -h localhost -p 5432

# If fails, restore from backup
wal-g backup-fetch /var/lib/postgresql/14/main LATEST
sudo systemctl start postgresql
```

### API Service Down
```bash
# Quick restart
docker restart stellar-insights-backend

# Check logs
docker logs stellar-insights-backend --tail 50

# Verify health
curl http://localhost:8080/health
```

### Out of Disk Space
```bash
# Check space
df -h

# Clean logs
sudo find /var/log -name "*.log" -mtime +7 -delete

# Clean old backups
find /var/backups -mtime +30 -delete
```

### High Memory Usage
```bash
# Check memory
free -h
docker stats --no-stream

# Restart service
docker restart stellar-insights-backend
```

---

## 📞 Escalation Path

**Level 1:** On-call Engineer (0-15 min)  
**Level 2:** Technical Lead (15-30 min)  
**Level 3:** Incident Commander (30-60 min)  
**Level 4:** CTO (60+ min or SEV-1)

---

## 🔐 Critical Credentials

**Location:** 1Password / Vault  
**Access:** Emergency break-glass procedure

- Database: `stellar_insights_admin`
- AWS: `stellar-insights-admin`
- SSH: `~/.ssh/stellar-insights.pem`

---

## 💾 Backup Locations

**Primary:** `s3://stellar-insights-backups/`  
**Secondary:** `/var/backups/`  
**Retention:** 30 days

```bash
# List backups
wal-g backup-list

# Restore latest
wal-g backup-fetch /var/lib/postgresql/14/main LATEST
```

---

## ✅ Recovery Validation

After any recovery:

```bash
# 1. Check services
docker ps
sudo systemctl status postgresql
sudo systemctl status redis

# 2. Test connectivity
curl http://localhost:8080/health
psql -U postgres -c "SELECT 1"
redis-cli PING

# 3. Verify data
psql -U postgres -d stellar_insights -c "
  SELECT COUNT(*) FROM anchors;
  SELECT COUNT(*) FROM corridors;
"

# 4. Check logs
docker logs stellar-insights-backend --tail 50
tail -f /var/log/postgresql/postgresql-14-main.log
```

---

## 📊 Key Metrics

| Service | RTO | RPO |
|---------|-----|-----|
| Database | 1 hour | 15 min |
| Backend API | 2 hours | N/A |
| Frontend | 4 hours | N/A |
| Redis | 2 hours | 1 hour |

---

## 🔔 Monitoring

**Dashboard:** https://monitoring.stellar-insights.com  
**Alerts:** PagerDuty / Slack #alerts

**Key Alerts:**
- Database down
- API error rate > 5%
- Response time > 2s
- Disk space > 80%
- Backup failure

---

## 📝 Documentation

**Full DR Plan:** [DISASTER_RECOVERY_PLAN.md](./DISASTER_RECOVERY_PLAN.md)  
**Backup Procedures:** [BACKUP-RESTORE-PROCEDURES.md](./BACKUP-RESTORE-PROCEDURES.md)  
**Testing:** [DR-TESTING-PROCEDURES.md](./DR-TESTING-PROCEDURES.md)  
**Post-Incident:** [POST-INCIDENT-REVIEW-TEMPLATE.md](./POST-INCIDENT-REVIEW-TEMPLATE.md)

---

## 🆘 Emergency Commands

### Stop Everything
```bash
docker-compose down
sudo systemctl stop stellar-insights-*
sudo systemctl stop postgresql
```

### Start Everything
```bash
sudo systemctl start postgresql
sudo systemctl start redis
docker-compose up -d
```

### Check Everything
```bash
# Services
docker ps
sudo systemctl status postgresql
sudo systemctl status redis

# Connectivity
curl http://localhost:8080/health
psql -U postgres -c "SELECT 1"
redis-cli PING

# Logs
docker logs stellar-insights-backend --tail 20
tail -20 /var/log/postgresql/postgresql-14-main.log
```

---

## 🎯 Remember

1. **Stay Calm** - Follow the runbooks
2. **Communicate** - Update #incidents channel
3. **Document** - Record all actions
4. **Validate** - Verify recovery success
5. **Learn** - Complete post-incident review

---

**Last Updated:** 2024  
**Emergency Contact:** devops@stellar-insights.com
