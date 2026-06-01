# Post-Incident Review Template

**Incident ID:** [INC-YYYY-NNNN]  
**Date:** [YYYY-MM-DD]  
**Severity:** [SEV-1/2/3/4]  
**Status:** [Draft/Final]

---

## Executive Summary

**One-line summary:** [Brief description of what happened]

**Impact:**
- Duration: [X hours Y minutes]
- Affected users: [Number or percentage]
- Services impacted: [List]
- Data loss: [Yes/No - details]

**Root Cause:** [One sentence]

**Resolution:** [One sentence]

---

## Incident Details

### Basic Information

| Field | Value |
|-------|-------|
| Incident ID | INC-YYYY-NNNN |
| Severity | SEV-X |
| Start Time | YYYY-MM-DD HH:MM UTC |
| End Time | YYYY-MM-DD HH:MM UTC |
| Duration | X hours Y minutes |
| Incident Commander | [Name] |
| Technical Lead | [Name] |

### Services Affected

- [ ] Backend API
- [ ] Frontend
- [ ] Database
- [ ] Redis Cache
- [ ] Monitoring
- [ ] Other: [Specify]

### Impact Assessment

**User Impact:**
- Total users affected: [Number]
- Percentage of user base: [X%]
- Geographic regions: [List]
- User-facing errors: [Description]

**Business Impact:**
- Revenue impact: [$X or N/A]
- SLA breach: [Yes/No]
- Reputation impact: [Low/Medium/High]
- Regulatory implications: [Yes/No]

**Technical Impact:**
- Data loss: [Yes/No - details]
- Data corruption: [Yes/No - details]
- Service degradation: [Description]
- Recovery time: [X hours]

---

## Timeline

### Detection Phase

**[HH:MM UTC]** - Initial Detection
- How detected: [Monitoring alert / User report / Other]
- Alert/ticket: [Link]
- First responder: [Name]

### Response Phase

**[HH:MM UTC]** - Incident Declared
- Severity assigned: SEV-X
- Incident Commander: [Name]
- Response team assembled

**[HH:MM UTC]** - Investigation Began
- Initial hypothesis: [Description]
- Diagnostic steps taken: [List]

**[HH:MM UTC]** - Root Cause Identified
- Root cause: [Description]
- Contributing factors: [List]

### Resolution Phase

**[HH:MM UTC]** - Mitigation Started
- Actions taken: [List]
- Challenges encountered: [List]

**[HH:MM UTC]** - Service Restored
- Resolution method: [Description]
- Verification steps: [List]

**[HH:MM UTC]** - Incident Closed
- Final checks completed
- Monitoring resumed
- Post-mortem scheduled

---

## Root Cause Analysis

### Primary Root Cause

**What happened:**
[Detailed technical explanation of the root cause]

**Why it happened:**
[Underlying reasons - technical, process, or human factors]

**Why it wasn't caught earlier:**
[Gaps in monitoring, testing, or processes]

### Contributing Factors

1. **[Factor 1]**
   - Description: [Details]
   - Impact: [How it contributed]

2. **[Factor 2]**
   - Description: [Details]
   - Impact: [How it contributed]

### Five Whys Analysis

1. **Why did the incident occur?**
   - [Answer]

2. **Why [answer from #1]?**
   - [Answer]

3. **Why [answer from #2]?**
   - [Answer]

4. **Why [answer from #3]?**
   - [Answer]

5. **Why [answer from #4]?**
   - [Root cause]

---

## What Went Well

### Positive Aspects

1. **[Aspect 1]**
   - What: [Description]
   - Why it helped: [Explanation]

2. **[Aspect 2]**
   - What: [Description]
   - Why it helped: [Explanation]

### Effective Procedures

- [Procedure that worked well]
- [Tool or process that helped]
- [Communication that was effective]

---

## What Went Wrong

### Issues Identified

1. **[Issue 1]**
   - What: [Description]
   - Impact: [How it affected response]
   - Severity: [High/Medium/Low]

2. **[Issue 2]**
   - What: [Description]
   - Impact: [How it affected response]
   - Severity: [High/Medium/Low]

### Gaps in Procedures

- [Missing documentation]
- [Inadequate monitoring]
- [Insufficient testing]
- [Communication breakdown]

---

## Action Items

### Immediate Actions (Complete within 1 week)

| ID | Action | Owner | Due Date | Status |
|----|--------|-------|----------|--------|
| AI-1 | [Action description] | [Name] | YYYY-MM-DD | [Open/In Progress/Done] |
| AI-2 | [Action description] | [Name] | YYYY-MM-DD | [Open/In Progress/Done] |

### Short-term Actions (Complete within 1 month)

| ID | Action | Owner | Due Date | Status |
|----|--------|-------|----------|--------|
| AI-3 | [Action description] | [Name] | YYYY-MM-DD | [Open/In Progress/Done] |
| AI-4 | [Action description] | [Name] | YYYY-MM-DD | [Open/In Progress/Done] |

### Long-term Actions (Complete within 3 months)

| ID | Action | Owner | Due Date | Status |
|----|--------|-------|----------|--------|
| AI-5 | [Action description] | [Name] | YYYY-MM-DD | [Open/In Progress/Done] |
| AI-6 | [Action description] | [Name] | YYYY-MM-DD | [Open/In Progress/Done] |

---

## Lessons Learned

### Technical Lessons

1. **[Lesson 1]**
   - What we learned: [Description]
   - How to apply: [Action]

2. **[Lesson 2]**
   - What we learned: [Description]
   - How to apply: [Action]

### Process Lessons

1. **[Lesson 1]**
   - What we learned: [Description]
   - How to apply: [Action]

2. **[Lesson 2]**
   - What we learned: [Description]
   - How to apply: [Action]

### Communication Lessons

1. **[Lesson 1]**
   - What we learned: [Description]
   - How to apply: [Action]

---

## Prevention Measures

### Monitoring Improvements

- [ ] Add alert for [specific condition]
- [ ] Improve dashboard for [specific metric]
- [ ] Implement automated check for [specific issue]

### Process Improvements

- [ ] Update runbook: [specific runbook]
- [ ] Add testing for [specific scenario]
- [ ] Improve documentation for [specific procedure]

### Technical Improvements

- [ ] Implement [specific technical change]
- [ ] Add redundancy for [specific component]
- [ ] Improve error handling in [specific area]

---

## Communication

### Internal Communication

**Timeline:**
- [HH:MM] - Initial notification sent
- [HH:MM] - Status update 1
- [HH:MM] - Status update 2
- [HH:MM] - Resolution notification

**Channels Used:**
- Slack #incidents
- Email distribution list
- Phone calls (if applicable)

**Effectiveness:**
- What worked: [Description]
- What didn't: [Description]
- Improvements needed: [List]

### External Communication

**Customer Notifications:**
- Status page updates: [Number]
- Email notifications: [Yes/No]
- Social media: [Yes/No]

**Stakeholder Communication:**
- Executive briefing: [Date/Time]
- Customer success team: [Date/Time]
- Sales team: [Date/Time]

---

## Metrics

### Response Metrics

| Metric | Target | Actual | Met? |
|--------|--------|--------|------|
| Time to Detect | < 5 min | [X min] | [Yes/No] |
| Time to Acknowledge | < 5 min | [X min] | [Yes/No] |
| Time to Mitigate | < 30 min | [X min] | [Yes/No] |
| Time to Resolve | < RTO | [X hours] | [Yes/No] |

### Impact Metrics

| Metric | Value |
|--------|-------|
| Total Downtime | [X hours Y min] |
| Users Affected | [Number] |
| Failed Requests | [Number] |
| Data Loss | [Amount] |
| Recovery Point | [Timestamp] |

---

## Attachments

### Supporting Documents

- [ ] Incident timeline (detailed)
- [ ] Chat logs
- [ ] Monitoring graphs
- [ ] Log excerpts
- [ ] Configuration changes
- [ ] Code changes (if applicable)

### Links

- Incident ticket: [URL]
- Monitoring dashboard: [URL]
- Related incidents: [URLs]
- Action item tracker: [URL]

---

## Review and Approval

### Review Meeting

**Date:** [YYYY-MM-DD]  
**Attendees:**
- [Name] - [Role]
- [Name] - [Role]
- [Name] - [Role]

**Discussion Points:**
- [Key point 1]
- [Key point 2]
- [Key point 3]

### Approvals

| Role | Name | Approved | Date |
|------|------|----------|------|
| Incident Commander | [Name] | [Yes/No] | [Date] |
| Technical Lead | [Name] | [Yes/No] | [Date] |
| Engineering Manager | [Name] | [Yes/No] | [Date] |

---

## Follow-up

### Action Item Review

**Next Review Date:** [YYYY-MM-DD]

**Review Cadence:**
- Week 1: Check immediate actions
- Week 2: Check short-term actions
- Month 1: Check all actions
- Month 3: Final review

### Knowledge Sharing

- [ ] Post-mortem published internally
- [ ] Team meeting presentation scheduled
- [ ] Documentation updated
- [ ] Training materials created (if needed)
- [ ] External blog post (if applicable)

---

**Document Status:** [Draft/Final]  
**Last Updated:** [YYYY-MM-DD]  
**Next Review:** [YYYY-MM-DD]  
**Owner:** [Name]
