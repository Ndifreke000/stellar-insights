# GDPR Compliance Implementation

## Overview

This document describes the GDPR compliance features implemented in Stellar Insights, including:
- **Right to Access (Data Portability)** — Users can export their personal data
- **Right to be Forgotten (Erasure)** — Users can request deletion of their personal data
- **Consent Management** — Users can view and manage their privacy consents

## Personal Data Inventory

All personal data held about a user is tracked across the following tables:

### User Account Data
- **users** — Account information (username, password hash)
  - **Export**: username, account creation/modification timestamps
  - **Deletion**: Username anonymized to `deleted_user_<uuid>`, password hash removed

### Consent and Privacy Data
- **user_consents** — Privacy consent records (marketing, analytics, etc.)
  - **Export**: All consent choices with grant/revoke timestamps
  - **Deletion**: Completely deleted (no legal requirement to retain)

- **data_processing_log** — Records of data processing activities on the user's account
  - **Export**: Processing activity log (purpose, legal basis, timestamps)
  - **Deletion**: Completely deleted (used for audit only)

### API Access Data
- **api_keys** — User-generated API keys and their metadata
  - **Export**: Key IDs, names, creation dates (NOT secrets or hashes)
  - **Deletion**: All API keys deleted (cannot function without them)

### Audit and Security Data
- **admin_audit_log** — Administrative actions affecting the user's account
  - **Export**: User's own actions logged in this table
  - **Deletion**: User reference anonymized to `anonymized` (GDPR Article 17(3)(b) permits retention for legal/compliance purposes)

- **vault_audit_log** — Vault operations by or affecting the user
  - **Export**: User-triggered vault operations
  - **Deletion**: User reference anonymized to `anonymized` (audit trail retention required)

- **data_export_requests** — Historical export request data
  - **Export**: Visible in export (user can see their export history)
  - **Deletion**: Completely deleted

- **data_deletion_requests** — Historical deletion request data
  - **Export**: Visible in export (user can see their deletion history)
  - **Deletion**: Completely deleted

## Data Export (Right to Access)

### API Endpoint
```
POST /api/v1/gdpr/export
{
  "data_types": ["profile", "consents", "api_keys", "activity_log"],
  "export_format": "json"
}
```

### Process
1. User submits an export request via the API
2. Request is stored with status `pending`
3. A background job processes the request:
   - Retrieves all personal data for the user
   - Compiles data into a structured JSON format
   - Marks request as `completed`
   - Data expires after 7 days for security
4. User can download the export via the signed download link

### Export Format

The export is delivered as a single JSON file with the following structure:
```json
{
  "exported_at": "2026-01-15T10:30:00Z",
  "user_info": {
    "id": "user_id_123",
    "username": "john_doe",
    "created_at": "2025-01-01T00:00:00Z",
    "updated_at": "2026-01-10T15:45:00Z"
  },
  "consents": [
    {
      "consent_type": "marketing",
      "consent_given": true,
      "granted_at": "2025-01-05T12:00:00Z",
      "revoked_at": null
    }
  ],
  "api_keys": [
    {
      "id": "key_abc123",
      "name": "Production API Key",
      "created_at": "2025-06-01T08:00:00Z"
    }
  ],
  "audit_logs": [
    {
      "id": "audit_xyz",
      "action": "export_created",
      "timestamp": "2026-01-15T10:30:00Z",
      "status": "success"
    }
  ]
}
```

### Data Portability
The JSON format enables easy data portability — users can import this data into other systems or services.

## Data Deletion (Right to be Forgotten)

### API Endpoint
```
POST /api/v1/gdpr/deletion
{
  "reason": "I no longer need this service",
  "delete_all_data": true
}
```

### Deletion Workflow
1. **Request Creation** (Day 0)
   - User submits deletion request
   - Server creates request with status `pending`
   - A confirmation token is generated and sent to user's email (or verified via 2FA)

2. **Confirmation** (User confirms via email/2FA)
   - User clicks confirmation link or verifies via 2FA
   - Request status moves to `confirmed`

3. **Deletion Scheduled** (Days 1-30 grace period)
   - Request status is `scheduled`
   - User can cancel up until day 30

4. **Cancellation Window**
   - User can cancel deletion any time in the 30-day grace period
   - After 30 days, cancellation is no longer possible

5. **Automatic Execution** (After Day 30)
   - A background job detects scheduled deletions past their grace period
   - Deletion is executed atomically in a database transaction
   - All personal data is either deleted or anonymized per the inventory above

### Deletion and Anonymization Strategy

**Completely Deleted (No Retention):**
- user_consents
- data_processing_log
- api_keys
- data_export_requests
- data_deletion_requests

**Anonymized (Legal Retention Required — GDPR Article 17(3)(b)):**
- admin_audit_log — user_id anonymized to "anonymized"
- vault_audit_log — user_id anonymized to "anonymized"

**Partial Anonymization:**
- users — username anonymized to `deleted_user_<uuid>`, password_hash removed

**Rationale**: Audit logs must be retained for legal compliance, regulatory requirements, and to detect fraud/abuse. Anonymizing rather than deleting ensures the audit trail remains intact while severing the link to the user's identity.

## Consent Management (Transparency & Control)

### API Endpoints

**Get all consents:**
```
GET /api/v1/gdpr/consents
```

**Update a single consent:**
```
PUT /api/v1/gdpr/consents
{
  "consent_type": "marketing",
  "consent_given": false
}
```

**Batch update consents:**
```
PUT /api/v1/gdpr/consents/batch
{
  "consents": [
    {"consent_type": "marketing", "consent_given": false},
    {"consent_type": "analytics", "consent_given": true}
  ]
}
```

### Consent Types

| Consent Type | Description | Legal Basis | Default |
|---|---|---|---|
| marketing | Marketing communications and promotional emails | Consent | Opt-out |
| analytics | Usage analytics and aggregated behavior analysis | Consent | Opt-out |
| data_improvement | Using data to improve service features | Consent | Opt-out |
| third_party_sharing | Sharing anonymized data with partners | Consent | Opt-out |

### Consent Recording

Each consent change is recorded with:
- Timestamp (when the change occurred)
- IP address (optional, for audit trail)
- User agent (optional, device information)
- Previous and new values
- Grant/revoke timestamps

## GDPR Compliance Features

### Transparency
- Users can export all personal data held about them
- Users can view their complete consent history
- Users can see what data is being processed and why

### Control
- Users can change their consent choices any time
- Users can request data deletion (Right to be Forgotten)
- Users can cancel deletion requests up to 30 days after request

### Audit Trail
- All consent changes are logged with timestamps
- Admin actions affecting users are logged and anonymized if user is deleted
- Export/deletion requests are tracked for compliance verification

### Security
- Exports are delivered via signed URLs with short TTL (7 days)
- Deletion requires confirmation step (email/2FA) to prevent accidental erasure
- All database operations are atomic to prevent partial deletions
- Audit logs cannot be modified, only appended (immutable)

## Legal Bases

Data processing activities are logged with their legal basis:

| Legal Basis | Description |
|---|---|
| Consent | User has explicitly consented to this processing |
| Legitimate Interest | Processing is necessary for our legitimate business interests |
| Contract | Processing is required to perform a service the user requested |
| Legal Obligation | Processing is required by law or regulation |
| Public Task | Processing is necessary to perform official duties |

## API Response Codes

| Code | Scenario |
|---|---|
| 200 | Successfully retrieved data |
| 201 | Successfully created export/deletion request |
| 400 | Invalid request (e.g., no data types for export) |
| 401 | Not authenticated |
| 403 | User does not have permission |
| 404 | Request/resource not found |
| 500 | Server error |

## Testing

### Unit Tests
- Consent upsert (insert new, update existing)
- Export request creation
- Deletion request state transitions
- Batch consent updates

### Integration Tests
- End-to-end export workflow
- End-to-end deletion workflow with confirmation
- Deletion cancellation
- Consent change audit logging

## Implementation Notes

### Design Decisions
1. **30-day deletion grace period** — Complies with GDPR while providing a safety window for accidental requests
2. **Anonymization vs. Deletion for Audit Logs** — Retains legal compliance while honoring user's erasure request
3. **No automatic exports** — Exports require explicit user request; automatic data retention is not performed
4. **Audit log immutability** — Prevents tampering with compliance records

### Future Enhancements
- Automated email confirmations for deletion requests
- 2FA verification option for deletion confirmation
- Scheduled background job for automatic deletion execution
- Data minimization review (regularly audit and delete non-essential data)
- Vendor/processor data audit (identify third-party data sharing)
