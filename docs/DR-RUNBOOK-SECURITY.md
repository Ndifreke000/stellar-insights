# Security Incident Response Runbook

**Priority:** P0 - Critical  
**RTO:** Varies by incident  
**Last Updated:** 2024

---

## Incident Types

| Type | Response Time | Procedure |
|------|---------------|-----------|
| Data Breach | Immediate | [Data Breach](#data-breach) |
| DDoS Attack | 15 minutes | [DDoS Mitigation](#ddos-attack) |
| Unauthorized Access | Immediate | [Access Breach](#unauthorized-access) |
| Malware/Ransomware | Immediate | [Malware Response](#malware-ransomware) |

---

## Data Breach

### Immediate Actions (0-15 minutes)

```bash
# 1. Isolate affected systems
sudo iptables -A INPUT -j DROP  # Block all incoming
sudo iptables -A OUTPUT -j DROP  # Block all outgoing

# 2. Preserve evidence
sudo dd if=/dev/sda of=/mnt/forensics/disk-image.dd bs=4M
sudo tar -czf /mnt/forensics/logs-$(date +%Y%m%d).tar.gz /var/log/

# 3. Revoke all API keys
psql -U postgres -d stellar_insights -c "UPDATE api_keys SET revoked = true;"

# 4. Force password resets
psql -U postgres -d stellar_insights -c "UPDATE users SET force_password_reset = true;"
```

### Investigation (15-60 minutes)

```bash
# Check access logs
sudo grep -i "unauthorized\|failed\|breach" /var/log/auth.log

# Check database access
psql -U postgres -d stellar_insights -c "
  SELECT * FROM admin_audit_log 
  WHERE timestamp > NOW() - INTERVAL '24 hours'
  ORDER BY timestamp DESC;
"

# Check for data exfiltration
sudo netstat -an | grep ESTABLISHED
sudo tcpdump -i any -w /tmp/capture.pcap
```

### Containment

```bash
# 1. Change all credentials
# 2. Rotate encryption keys
# 3. Update firewall rules
# 4. Enable additional logging
# 5. Notify affected users
```

---

## DDoS Attack

### Detection

```bash
# Check connection counts
netstat -an | grep :80 | wc -l

# Check top IPs
netstat -an | grep :80 | awk '{print $5}' | cut -d: -f1 | sort | uniq -c | sort -rn | head -20

# Check bandwidth
iftop -i eth0
```

### Mitigation

```bash
# 1. Enable rate limiting
sudo iptables -A INPUT -p tcp --dport 80 -m limit --limit 25/minute --limit-burst 100 -j ACCEPT

# 2. Block attacking IPs
for ip in $(cat /tmp/attack-ips.txt); do
  sudo iptables -A INPUT -s $ip -j DROP
done

# 3. Enable CloudFlare/CDN protection
# Update DNS to point to CDN

# 4. Scale infrastructure
# Add more servers via load balancer
```

---

## Unauthorized Access

### Immediate Response

```bash
# 1. Lock affected accounts
psql -U postgres -d stellar_insights -c "
  UPDATE users SET locked = true 
  WHERE id IN (SELECT user_id FROM suspicious_logins);
"

# 2. Terminate active sessions
psql -U postgres -d stellar_insights -c "DELETE FROM user_sessions WHERE user_id = <affected_user_id>;"

# 3. Review access logs
sudo grep "Failed password" /var/log/auth.log | tail -100

# 4. Check for privilege escalation
sudo grep "sudo" /var/log/auth.log | tail -100
```

### Investigation

```bash
# Check login attempts
psql -U postgres -d stellar_insights -c "
  SELECT username, ip_address, timestamp, success
  FROM login_attempts
  WHERE timestamp > NOW() - INTERVAL '24 hours'
  ORDER BY timestamp DESC;
"

# Check admin actions
psql -U postgres -d stellar_insights -c "
  SELECT * FROM admin_audit_log
  WHERE user_id = <suspicious_user_id>
  ORDER BY timestamp DESC;
"
```

---

## Malware/Ransomware

### Immediate Actions

```bash
# 1. Isolate infected systems
sudo iptables -A INPUT -j DROP
sudo iptables -A OUTPUT -j DROP

# 2. Stop all services
sudo systemctl stop stellar-insights-*

# 3. Take snapshots
# Create VM/container snapshots for forensics

# 4. Notify security team
```

### Recovery

```bash
# 1. Restore from clean backup
# Use backup from before infection

# 2. Scan all systems
sudo clamscan -r /

# 3. Update all software
sudo apt-get update && sudo apt-get upgrade -y

# 4. Rebuild from clean images
# Deploy fresh infrastructure
```

---

## Post-Incident

### Required Actions

- [ ] Document timeline
- [ ] Preserve evidence
- [ ] Notify stakeholders
- [ ] File reports (if required)
- [ ] Update security controls
- [ ] Conduct post-mortem
- [ ] Implement preventive measures

### Notification Requirements

**Internal:**
- Security team (immediate)
- Legal team (within 1 hour)
- Executive team (within 2 hours)

**External:**
- Affected users (within 72 hours)
- Regulatory bodies (as required)
- Law enforcement (if criminal)

---

## Escalation

- **Immediate:** Security Lead
- **15 minutes:** CISO
- **30 minutes:** Legal team
- **1 hour:** Executive team

---

**Document Owner:** Security Team  
**Emergency Contact:** security@stellar-insights.com
