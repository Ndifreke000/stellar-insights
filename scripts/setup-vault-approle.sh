#!/bin/bash
# Setup script for Vault AppRole authentication (Alternative to JWT)
# Use this if your CI/CD platform doesn't support JWT OIDC
# 
# Prerequisites:
# - Vault CLI installed
# - Logged in to Vault with sufficient permissions
# - VAULT_ADDR and VAULT_TOKEN set

set -e

VAULT_ADDR="${VAULT_ADDR:-https://vault.example.com:8200}"
VAULT_TOKEN="${VAULT_TOKEN:-}"

if [ -z "$VAULT_TOKEN" ]; then
    echo "Error: VAULT_TOKEN not set"
    exit 1
fi

echo "Setting up AppRole authentication for stellar-app..."

# Enable AppRole auth method if not already enabled
echo "Enabling AppRole auth method..."
vault auth enable approle 2>/dev/null || echo "AppRole already enabled"

# Create or update AppRole role
echo "Creating AppRole role: stellar-app"
vault write auth/approle/role/stellar-app \
    bind_secret_id=true \
    secret_id_ttl=8760h \
    secret_id_num_uses=0 \
    token_ttl=1h \
    token_max_ttl=24h \
    policies="stellar-app"

# Get role ID
echo "Retrieving Role ID..."
ROLE_ID=$(vault read -field=role_id auth/approle/role/stellar-app/role-id)
echo "Role ID: $ROLE_ID"

# Generate secret ID
echo "Generating Secret ID..."
SECRET_ID=$(vault write -field=secret_id -f auth/approle/role/stellar-app/secret-id)
echo "Secret ID: $SECRET_ID"

echo ""
echo "========================================="
echo "AppRole Setup Complete"
echo "========================================="
echo ""
echo "Add these to your CI/CD environment:"
echo "  VAULT_ROLE_ID=$ROLE_ID"
echo "  VAULT_SECRET_ID=$SECRET_ID"
echo ""
echo "Or add to GitHub Secrets:"
echo "  gh secret set VAULT_ROLE_ID -b '$ROLE_ID'"
echo "  gh secret set VAULT_SECRET_ID -b '$SECRET_ID'"
echo ""
echo "Then use the GitHub Actions workflow: .github/workflows/vault-deploy-approle.yml"
echo ""
