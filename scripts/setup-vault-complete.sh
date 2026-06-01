#!/bin/bash
# Complete Vault Setup Script for stellar-insights
# Automates all Vault configuration including:
# - KV v2 secrets engine
# - Database dynamic secrets (PostgreSQL)
# - AppRole authentication
# - Policies
# - Audit logging
#
# Usage: ./scripts/setup-vault-complete.sh
# Requires: VAULT_ADDR and VAULT_TOKEN set

set -e

VAULT_ADDR="${VAULT_ADDR:-}"
VAULT_TOKEN="${VAULT_TOKEN:-}"
DB_HOST="${DB_HOST:-db.example.com}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-stellar}"
DB_ADMIN_USER="${DB_ADMIN_USER:-vault_admin}"
DB_ADMIN_PASS="${DB_ADMIN_PASS:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_step() { echo -e "\n${BLUE}==>${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}!${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

# Validation
validate_env() {
    log_step "Validating environment..."
    
    if [ -z "$VAULT_ADDR" ]; then
        read -p "Enter VAULT_ADDR (e.g., https://vault.example.com:8200): " VAULT_ADDR
    fi
    
    if [ -z "$VAULT_TOKEN" ]; then
        read -sp "Enter VAULT_TOKEN: " VAULT_TOKEN
        echo
    fi
    
    export VAULT_ADDR VAULT_TOKEN
    log_success "Environment validated"
}

# Test Vault connectivity
test_vault_connection() {
    log_step "Testing Vault connectivity..."
    
    if curl -s -H "X-Vault-Token: $VAULT_TOKEN" "$VAULT_ADDR/v1/sys/health" > /dev/null; then
        log_success "Connected to Vault"
    else
        log_error "Cannot connect to Vault at $VAULT_ADDR"
        exit 1
    fi
}

# Enable KV v2 secrets engine
setup_kv_engine() {
    log_step "Setting up KV v2 secrets engine..."
    
    vault secrets enable -path=secret kv-v2 2>/dev/null || {
        log_warn "KV v2 already enabled"
    }
    
    log_success "KV v2 secrets engine ready"
}

# Create base secrets structure
create_secrets() {
    log_step "Creating base secrets..."
    
    # JWT Secret
    JWT_SECRET=$(openssl rand -hex 32)
    vault kv put secret/stellar/jwt_secret value="$JWT_SECRET"
    log_success "Created jwt_secret"
    
    # OAuth Client Secret
    OAUTH_SECRET=$(openssl rand -hex 32)
    vault kv put secret/stellar/oauth_client_secret value="$OAUTH_SECRET"
    log_success "Created oauth_client_secret"
    
    # Price Feed API Key (placeholder)
    vault kv put secret/stellar/price_feed_api_key key="PLACEHOLDER_API_KEY"
    log_success "Created price_feed_api_key (placeholder)"
    
    # Redis URL
    vault kv put secret/stellar/redis_url url="redis://localhost:6379"
    log_success "Created redis_url"
    
    # Stellar Source Secret Key (placeholder)
    vault kv put secret/stellar/stellar_source_secret_key key="PLACEHOLDER_STELLAR_KEY"
    log_success "Created stellar_source_secret_key (placeholder)"
    
    echo ""
    log_warn "Update these placeholder secrets:"
    log_warn "  vault kv put secret/stellar/price_feed_api_key key='YOUR_API_KEY'"
    log_warn "  vault kv put secret/stellar/stellar_source_secret_key key='SBXXXXXX...'"
}

# Setup PostgreSQL dynamic secrets
setup_postgres_secrets() {
    log_step "Setting up PostgreSQL dynamic secrets..."
    
    # Check if DB_ADMIN_PASS is set
    if [ -z "$DB_ADMIN_PASS" ]; then
        log_warn "DB_ADMIN_PASS not provided, skipping PostgreSQL setup"
        log_warn "To complete: configure database secret engine manually"
        return
    fi
    
    # Enable database secret engine
    vault secrets enable database 2>/dev/null || {
        log_warn "Database secrets engine already enabled"
    }
    
    # Configure PostgreSQL connection
    vault write database/config/postgresql \
        plugin_name=postgresql-database-plugin \
        allowed_roles="stellar-app,read-only" \
        connection_url="postgresql://{{username}}:{{password}}@${DB_HOST}:${DB_PORT}/${DB_NAME}" \
        username="$DB_ADMIN_USER" \
        password="$DB_ADMIN_PASS"
    
    log_success "PostgreSQL configured"
    
    # Create stellar-app role
    vault write database/roles/stellar-app \
        db_name=postgresql \
        creation_statements="CREATE USER \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' VALID UNTIL '{{expiration}}'; GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO \"{{name}}\";" \
        default_ttl="1h" \
        max_ttl="24h"
    
    log_success "stellar-app database role created"
    
    # Create read-only role
    vault write database/roles/read-only \
        db_name=postgresql \
        creation_statements="CREATE USER \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' VALID UNTIL '{{expiration}}'; GRANT CONNECT ON DATABASE $DB_NAME TO \"{{name}}\"; GRANT USAGE ON SCHEMA public TO \"{{name}}\"; GRANT SELECT ON ALL TABLES IN SCHEMA public TO \"{{name}}\";" \
        default_ttl="1h" \
        max_ttl="24h"
    
    log_success "read-only database role created"
}

# Setup Vault policy
setup_policy() {
    log_step "Setting up Vault policy..."
    
    # Create policy file
    cat > /tmp/stellar-app-policy.hcl <<'EOF'
# Stellar Insights Application Policy

# Allow reading KV v2 secrets
path "secret/data/stellar/*" {
  capabilities = ["read", "list"]
}

# Allow reading metadata
path "secret/metadata/stellar/*" {
  capabilities = ["read", "list"]
}

# Allow getting database credentials
path "database/creds/stellar-app" {
  capabilities = ["read"]
}

path "database/creds/read-only" {
  capabilities = ["read"]
}

# Allow lease renewal and revocation
path "sys/leases/renew" {
  capabilities = ["update"]
}

path "sys/leases/revoke" {
  capabilities = ["update"]
}

path "sys/leases/lookup" {
  capabilities = ["read"]
}

# Allow token renewal (for self)
path "auth/token/renew-self" {
  capabilities = ["update"]
}

# Allow token lookup (for self)
path "auth/token/lookup-self" {
  capabilities = ["read"]
}
EOF
    
    vault policy write stellar-app /tmp/stellar-app-policy.hcl
    log_success "Policy created: stellar-app"
    rm /tmp/stellar-app-policy.hcl
}

# Setup AppRole authentication
setup_approle() {
    log_step "Setting up AppRole authentication..."
    
    # Enable AppRole
    vault auth enable approle 2>/dev/null || {
        log_warn "AppRole already enabled"
    }
    
    # Create AppRole role
    vault write auth/approle/role/stellar-app \
        bind_secret_id=true \
        secret_id_ttl=8760h \
        secret_id_num_uses=0 \
        token_ttl=1h \
        token_max_ttl=24h \
        policies="stellar-app"
    
    log_success "AppRole created: stellar-app"
    
    # Get Role ID
    ROLE_ID=$(vault read -field=role_id auth/approle/role/stellar-app/role-id)
    echo ""
    log_success "Role ID: $ROLE_ID"
    
    # Generate Secret ID
    SECRET_ID=$(vault write -field=secret_id -f auth/approle/role/stellar-app/secret-id)
    log_success "Secret ID generated"
    
    echo ""
    log_warn "Save these credentials securely:"
    echo "  VAULT_ROLE_ID=$ROLE_ID"
    echo "  VAULT_SECRET_ID=$SECRET_ID"
    echo ""
}

# Setup GitHub OIDC authentication
setup_github_oidc() {
    log_step "Setting up GitHub OIDC authentication..."
    
    # Enable JWT auth method
    vault auth enable jwt 2>/dev/null || {
        log_warn "JWT auth method already enabled"
    }
    
    # Configure GitHub OIDC
    vault write auth/jwt/config \
        jwks_url="https://token.actions.githubusercontent.com/.well-known/jwks" \
        bound_audiences="https://github.com/Ndifreke000"
    
    log_success "GitHub OIDC configured"
    
    # Create JWT role for GitHub Actions
    vault write auth/jwt/role/stellar-github \
        bound_audiences="https://github.com/Ndifreke000" \
        user_claim="actor" \
        role_type_at_claim="iss" \
        policies="stellar-app"
    
    log_success "GitHub Actions role created: stellar-github"
}

# Enable audit logging
setup_audit_logging() {
    log_step "Setting up audit logging..."
    
    # Enable file audit backend
    vault audit enable file file_path=/var/log/vault/audit.log 2>/dev/null || {
        log_warn "File audit backend already enabled"
    }
    
    log_success "Audit logging enabled"
}

# Verify setup
verify_setup() {
    log_step "Verifying setup..."
    
    echo ""
    log_step "Secret paths:"
    vault kv list secret/stellar/ || log_error "Failed to list secrets"
    
    echo ""
    log_step "Database roles:"
    vault list database/roles/ || log_error "Failed to list database roles"
    
    echo ""
    log_step "Auth methods:"
    vault auth list
    
    echo ""
    log_success "Setup complete!"
}

main() {
    echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}   Vault Setup for stellar-insights                      ${BLUE}║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
    
    validate_env
    test_vault_connection
    setup_kv_engine
    create_secrets
    setup_postgres_secrets
    setup_policy
    setup_approle
    setup_github_oidc
    setup_audit_logging
    verify_setup
    
    echo ""
    echo -e "${GREEN}✓ Vault is ready for stellar-insights!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Update secret placeholders (price_feed_api_key, stellar_source_secret_key)"
    echo "  2. Add VAULT_ADDR and VAULT_ROLE_ID/VAULT_SECRET_ID to GitHub Secrets"
    echo "  3. Add VAULT_ADDR and VAULT_TOKEN to .env for development"
    echo "  4. Start the backend: cargo run"
    echo ""
}

main "$@"
