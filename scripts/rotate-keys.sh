#!/usr/bin/env bash
set -euo pipefail

# ⚠️  WARNING: This script handles sensitive cryptographic keys.
# All secrets are printed to stdout only. NEVER write to log files or commit output.
# Rotate keys only during maintenance windows to avoid service disruption.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Color output for clarity
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Stellar Insights Key Rotation Utility ===${NC}"
echo ""
echo "This script assists with rotating three types of cryptographic keys:"
echo "  1. JWT_SECRET (short-lived session tokens)"
echo "  2. ENCRYPTION_KEY (persistent data encryption)"
echo "  3. SEP-10 server keypair (Stellar anchor registration)"
echo ""

# ============================================================================
# Section 1: JWT_SECRET Rotation with Dual-Validation Window
# ============================================================================

rotate_jwt_secret() {
  echo -e "${GREEN}[1/3] JWT_SECRET Rotation${NC}"
  echo "Rotating JWT_SECRET with dual-validation window for zero-downtime switchover"
  echo ""

  # Generate new JWT secret (64 characters base64 = ~48 bytes)
  NEW_JWT_SECRET=$(openssl rand -base64 48)
  echo "New JWT_SECRET generated: ${NEW_JWT_SECRET:0:16}..."
  echo ""

  # Read current secret for reference
  OLD_JWT_SECRET="${JWT_SECRET:-UNKNOWN}"

  echo -e "${YELLOW}Dual-Validation Window Procedure:${NC}"
  echo ""
  echo "Step 1: In your deployment configuration (e.g., .env or secrets manager):"
  echo "  - Set JWT_SECRET_OLD=${OLD_JWT_SECRET:0:16}... (keep old secret accessible)"
  echo "  - Set JWT_SECRET=${NEW_JWT_SECRET:0:16}... (new secret for signing)"
  echo ""
  echo "Step 2: Restart the backend to pick up new config:"
  echo "  - Backend must accept tokens signed with BOTH secrets during overlap"
  echo "  - Overlap window: 15 minutes (time for existing sessions to expire)"
  echo "  - Existing JWT tokens signed with old secret remain valid"
  echo ""
  echo "Step 3: After 15 minutes (overlap window expires):"
  echo "  - Unset JWT_SECRET_OLD from your deployment"
  echo "  - Backend now validates only against new JWT_SECRET"
  echo "  - Sessions using old secret will require re-authentication"
  echo ""
  echo -e "${GREEN}✓ JWT_SECRET Rotation Ready${NC}"
  echo ""
  echo "Full new secret (save securely, never commit):"
  echo "${NEW_JWT_SECRET}"
  echo ""
}

# ============================================================================
# Section 2: ENCRYPTION_KEY Rotation
# ============================================================================

rotate_encryption_key() {
  echo -e "${GREEN}[2/3] ENCRYPTION_KEY Rotation${NC}"
  echo "ENCRYPTION_KEY rotation requires a data migration and cannot be automated."
  echo ""
  echo -e "${YELLOW}⚠️  Before rotating ENCRYPTION_KEY:${NC}"
  echo "  - Read docs/architecture/key-rotation.md for complete procedure"
  echo "  - Schedule maintenance window (all services will be unavailable)"
  echo "  - Backup database before starting migration"
  echo "  - Test migration on staging environment first"
  echo ""
  echo "The rotation process:"
  echo "  1. Generate new ENCRYPTION_KEY: openssl rand -hex 32"
  echo "  2. Run data migration script (see documentation)"
  echo "  3. Re-encrypt all encrypted fields with new key"
  echo "  4. Verify all data integrity after migration"
  echo "  5. Deploy updated backend with new ENCRYPTION_KEY"
  echo ""
  echo -e "${RED}ENCRYPTION_KEY rotation requires a data migration.${NC}"
  echo "See docs/architecture/key-rotation.md before proceeding."
  echo ""
}

# ============================================================================
# Section 3: SEP-10 Server Keypair Rotation
# ============================================================================

rotate_sep10_keypair() {
  echo -e "${GREEN}[3/3] SEP-10 Server Keypair Rotation${NC}"
  echo "Generating new Stellar keypair for SEP-10 anchor authentication"
  echo ""

  # Try to generate keypair using Stellar CLI if available
  if command -v stellar &> /dev/null; then
    echo "Using Stellar CLI to generate keypair..."
    KEYPAIR_OUTPUT=$(stellar keys generate --format json 2>&1 || echo "")

    if [[ -z "$KEYPAIR_OUTPUT" ]]; then
      echo -e "${YELLOW}Stellar CLI found but keypair generation failed.${NC}"
      echo "Falling back to manual generation command..."
      _print_sep10_manual_generation
    else
      NEW_PUBLIC_KEY=$(echo "$KEYPAIR_OUTPUT" | jq -r '.public_key')
      NEW_SECRET_KEY=$(echo "$KEYPAIR_OUTPUT" | jq -r '.secret_key')

      echo -e "${GREEN}✓ New Stellar Keypair Generated${NC}"
      echo ""
      echo "Public Key (share with Stellar anchor registrar):"
      echo "${NEW_PUBLIC_KEY}"
      echo ""
      echo "Secret Key (keep secure in secrets manager, NEVER commit):"
      echo "${NEW_SECRET_KEY}"
      echo ""
    fi
  else
    echo "Stellar CLI not found on system."
    _print_sep10_manual_generation
  fi

  echo -e "${YELLOW}Next Steps:${NC}"
  echo "  1. Use stellar.expert or Stellar SDKs to generate keypair:"
  echo "     https://developers.stellar.org/docs/build/apps/security/sep-0010"
  echo "  2. Update SEP-10 configuration with new public key"
  echo "  3. Register new public key with Stellar anchor network"
  echo "  4. After network registration, deploy with new secret key"
  echo "  5. Only then rotate out old keypair (when fully propagated)"
  echo ""
}

_print_sep10_manual_generation() {
  echo "Manual keypair generation (choose one method):"
  echo ""
  echo "Option A: Using Stellar CLI (if installed):"
  echo "  stellar keys generate"
  echo ""
  echo "Option B: Using Stellar SDK (JavaScript):"
  echo "  const StellarSdk = require('stellar-sdk');"
  echo "  const pair = StellarSdk.Keypair.random();"
  echo "  console.log('Public:', pair.publicKey());"
  echo "  console.log('Secret:', pair.secret());"
  echo ""
  echo "Option C: Using curl + Horizon (public ledger only, for testing):"
  echo "  curl -s https://friendbot.stellar.org?addr=GBRPYHIL2CI3C7OHIXBBYLCA3VU2MXXIIVU2KHV25QVKNUKE6FSKIZB"
  echo ""
}

# ============================================================================
# Main Entry Point
# ============================================================================

main() {
  # Check if running in a context where secrets are safe
  if [[ -z "${JWT_SECRET:-}" ]]; then
    echo -e "${YELLOW}⚠️  JWT_SECRET not found in environment.${NC}"
    echo "Make sure you've sourced .env or secrets before running this script:"
    echo "  source .env.production"
    echo "  ./scripts/rotate-keys.sh"
    echo ""
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
      exit 1
    fi
  fi

  # Validate we're not in a production shell without proper safeguards
  if [[ "${ENVIRONMENT:-}" == "production" ]]; then
    echo -e "${RED}⚠️  WARNING: Running in PRODUCTION environment.${NC}"
    echo "Key rotation in production should only be done during maintenance windows."
    echo ""
    read -p "Do you want to proceed with production key rotation? (type 'yes' to confirm) " confirmation
    if [[ "$confirmation" != "yes" ]]; then
      echo "Aborted."
      exit 1
    fi
  fi

  # Run rotation steps
  rotate_jwt_secret
  echo ""
  rotate_encryption_key
  echo ""
  rotate_sep10_keypair
  echo ""

  # Summary
  echo -e "${GREEN}=== Key Rotation Summary ===${NC}"
  echo "✓ JWT_SECRET rotation ready (15-minute overlap window)"
  echo "✓ ENCRYPTION_KEY rotation instructions provided"
  echo "✓ SEP-10 keypair generation ready"
  echo ""
  echo -e "${YELLOW}⚠️  Security Reminders:${NC}"
  echo "  1. NEVER commit any secrets to git"
  echo "  2. Store secrets in a secure secrets manager (Vault, AWS Secrets, etc.)"
  echo "  3. Audit log which user rotated keys and when"
  echo "  4. Test key rotation on staging first"
  echo "  5. Schedule during low-traffic periods or maintenance windows"
  echo ""
}

main "$@"
