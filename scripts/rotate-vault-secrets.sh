#!/bin/bash
# Vault Secret Rotation Script
# Rotates long-lived secrets and updates them in Vault
# Run this periodically (monthly recommended) for security.
# 
# Usage: ./scripts/rotate-vault-secrets.sh [secret-name]
#   secret-name: optional, rotate specific secret (jwt, oauth, api_keys, all)

set -e

VAULT_ADDR="${VAULT_ADDR:-}"
VAULT_TOKEN="${VAULT_TOKEN:-}"

if [ -z "$VAULT_ADDR" ] || [ -z "$VAULT_TOKEN" ]; then
    echo "Error: VAULT_ADDR and VAULT_TOKEN must be set"
    exit 1
fi

SECRET_TO_ROTATE="${1:-all}"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Backup existing secrets
backup_secrets() {
    local backup_file="vault-secrets-backup-$(date +%Y%m%d-%H%M%S).json"
    log_info "Backing up secrets to $backup_file"
    
    curl -s -X GET \
        "$VAULT_ADDR/v1/data/secret/stellar" \
        -H "X-Vault-Token: $VAULT_TOKEN" > "$backup_file"
    
    echo "$backup_file"
}

# Rotate JWT_SECRET
rotate_jwt_secret() {
    log_info "Rotating JWT_SECRET..."
    
    # Generate new secret (64 random characters)
    NEW_SECRET=$(openssl rand -hex 32)
    
    # Update in Vault
    curl -s -X POST \
        "$VAULT_ADDR/v1/data/secret/stellar/jwt_secret" \
        -H "X-Vault-Token: $VAULT_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{\"data\": {\"value\": \"$NEW_SECRET\"}}" > /dev/null
    
    log_info "JWT_SECRET rotated successfully"
    log_warn "Remember to restart the application to use new secret"
}

# Rotate OAuth secrets
rotate_oauth_secrets() {
    log_info "Rotating OAuth client secret..."
    
    # Generate new client secret
    NEW_SECRET=$(openssl rand -hex 32)
    
    curl -s -X POST \
        "$VAULT_ADDR/v1/data/secret/stellar/oauth_client_secret" \
        -H "X-Vault-Token: $VAULT_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{\"data\": {\"value\": \"$NEW_SECRET\"}}" > /dev/null
    
    log_info "OAuth client secret rotated successfully"
    log_warn "Update your OAuth app configuration with new secret"
}

# Rotate API keys
rotate_api_keys() {
    log_info "Rotating API keys..."
    
    log_warn "Manual API key rotation required for:"
    log_warn "  - Price feed API key (see provider documentation)"
    log_warn "  - AWS credentials (use AWS console)"
    log_warn "  - Stellar network keys (coordinate with team)"
}

# Check DB credential TTL
check_db_credentials() {
    log_info "Checking PostgreSQL dynamic credential status..."
    
    RESPONSE=$(curl -s -X GET \
        "$VAULT_ADDR/v1/database/creds/stellar-app" \
        -H "X-Vault-Token: $VAULT_TOKEN")
    
    LEASE_ID=$(echo "$RESPONSE" | jq -r '.lease_id')
    TTL=$(echo "$RESPONSE" | jq -r '.lease_duration')
    
    if [ "$LEASE_ID" != "null" ]; then
        log_info "Dynamic DB credentials valid for $TTL seconds"
        log_info "Lease ID: $LEASE_ID"
    else
        log_error "Failed to get dynamic credentials"
        return 1
    fi
}

# Audit log
audit_rotation() {
    local secret_name=$1
    local timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    curl -s -X POST \
        "$VAULT_ADDR/v1/data/secret/stellar/_audit" \
        -H "X-Vault-Token: $VAULT_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{
            \"data\": {
                \"rotated_secret\": \"$secret_name\",
                \"timestamp\": \"$timestamp\",
                \"operator\": \"$USER\"
            }
        }" > /dev/null 2>&1 || true
}

main() {
    log_info "Starting Vault secret rotation process"
    
    # Backup first
    BACKUP_FILE=$(backup_secrets)
    log_info "Backup created: $BACKUP_FILE"
    
    case "$SECRET_TO_ROTATE" in
        jwt)
            rotate_jwt_secret
            audit_rotation "jwt_secret"
            ;;
        oauth)
            rotate_oauth_secrets
            audit_rotation "oauth_client_secret"
            ;;
        api_keys)
            rotate_api_keys
            audit_rotation "api_keys"
            ;;
        all)
            rotate_jwt_secret
            rotate_oauth_secrets
            rotate_api_keys
            check_db_credentials
            audit_rotation "all_secrets"
            ;;
        *)
            log_error "Unknown secret: $SECRET_TO_ROTATE"
            echo "Usage: $0 [jwt|oauth|api_keys|all]"
            exit 1
            ;;
    esac
    
    log_info "Secret rotation complete"
    log_warn "Backup file saved: $BACKUP_FILE"
    log_warn "Keep this file secure for recovery purposes"
}

main "$@"
