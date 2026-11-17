# Runbook: [Service / Subsystem Name] - [Operational Scenario]

- **Document ID:** `RB-[SUB]-[NUMBER]`
- **Status:** Active | Draft | Deprecated
- **Target System:** [Service / Daemon Name]
- **Severity Classification:** P1 (Critical Outage) | P2 (Degraded Performance) | P3 (Minor Incident)

---

## 1. Operational Overview
[Concise description of the subsystem and its critical path responsibilities.]

---

## 2. Trigger Conditions & Alert Metrics
- **Alert Name:** `[AlertManager / Datadog / Prometheus Alert Name]`
- **Condition:** [e.g., Error rate > 5% over 2 minutes, or CPU saturation > 90%]
- **Dashboard Link:** `[URL / Metric Path]`

---

## 3. Immediate Triage & Diagnostics
Step-by-step diagnostic commands to inspect system health:

```bash
# 1. Inspect daemon process status
systemctl status [service-name]

# 2. Check recent fatal log entries
journalctl -u [service-name] -n 50 --no-pager | grep -i "panic\|error"

# 3. Check memory & socket saturation
netstat -tulpn | grep :[port]
```

---

## 4. Remediation & Recovery Procedures

### Scenario A: Process Hanging or Deadlocked
1. Execute graceful restart:
   ```bash
   systemctl reload-or-restart [service-name]
   ```
2. Verify process re-attaches to socket within 5 seconds.

### Scenario B: Database Connection Exhaustion
1. Purge idle pool connections.
2. Check upstream connection pool metrics.

---

## 5. Escalation & Incident Management
- **Primary On-Call:** [Role / Contact]
- **Escalation Path:** If service does not recover within 10 minutes, escalate to Engineering Lead.
- **Post-Mortem Requirement:** Mandatory post-mortem review for all P1/P2 incidents.
