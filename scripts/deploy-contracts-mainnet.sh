#!/usr/bin/env bash
# Deploy all Soroban contracts to mainnet with step-by-step human approval gates.
#
# Usage:
#   ./scripts/deploy-contracts-mainnet.sh [--source <identity>] [--fee <stroop>] [--dry-run]
#
# Prerequisites:
#   - stellar CLI installed and authenticated (stellar keys list)
#   - STELLAR_ACCOUNT env var set, or pass --source <identity>
#   - Network: mainnet (https://horizon.stellar.org)
#   - All contracts built for release first:
#       cd contracts && cargo build --release --target wasm32-unknown-unknown
#
# Deploy order (respects contract dependencies):
#   access-control → stellar_insights → analytics → governance →
#   escrow → token-swap → multi-sig-wallet → time-locked-transactions → upgrade
#
# Each contract is deployed only after the operator presses Enter.
# Ctrl-C at any gate aborts the run without deploying subsequent contracts.

set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────

CONTRACTS_DIR="$(cd "$(dirname "$0")/../contracts" && pwd)"
ENV_FILE="${CONTRACTS_DIR}/.env.mainnet"
NETWORK="mainnet"
RPC_URL="https://mainnet.sorobanrpc.com"
NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
FEE="${STELLAR_FEE:-1000}"

# ── Parse arguments ────────────────────────────────────────────────────────────

SOURCE="${STELLAR_ACCOUNT:-}"
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --source)  SOURCE="$2";   shift 2 ;;
        --fee)     FEE="$2";      shift 2 ;;
        --dry-run) DRY_RUN=true;  shift   ;;
        *) echo "Unknown argument: $1"; exit 1 ;;
    esac
done

if [[ -z "${SOURCE}" ]]; then
    echo "Error: deployer identity not set."
    echo "  Set STELLAR_ACCOUNT env var or pass --source <identity>"
    exit 1
fi

# ── Helpers ────────────────────────────────────────────────────────────────────

RED='\033[0;31m'
YEL='\033[1;33m'
GRN='\033[0;32m'
NC='\033[0m'

log()   { echo "[$(date -u +%H:%M:%S)] $*"; }
info()  { echo "  $*"; }
warn()  { echo -e "  ${YEL}WARNING:${NC} $*"; }
success(){ echo -e "  ${GRN}OK${NC} $*"; }

# Print a prominent gate and wait for operator confirmation.
approval_gate() {
    local label="$1"
    echo ""
    echo -e "${YEL}──────────────────────────────────────────────────────────${NC}"
    echo -e "${YEL}  APPROVAL GATE — ${label}${NC}"
    echo -e "${YEL}──────────────────────────────────────────────────────────${NC}"
    echo "  Review the deployment above before continuing."
    echo "  Press ENTER to proceed or Ctrl-C to abort."
    read -r
}

deploy_contract() {
    local name="$1"
    local wasm="$2"
    local alias="$3"

    log "Deploying ${name}..."

    if [[ ! -f "${wasm}" ]]; then
        echo "  Error: wasm not found at ${wasm}"
        echo "  Run 'cd contracts && cargo build --release --target wasm32-unknown-unknown' first."
        exit 1
    fi

    if [[ "${DRY_RUN}" == "true" ]]; then
        local fake_id="DRYRUN_${alias}"
        info "[dry-run] would deploy ${name} → ${alias}=${fake_id}"
        echo "${alias}=${fake_id}" >> "${ENV_FILE}"
        echo "${fake_id}"
        return
    fi

    local contract_id
    contract_id=$(stellar contract deploy \
        --wasm "${wasm}" \
        --source "${SOURCE}" \
        --network "${NETWORK}" \
        --rpc-url "${RPC_URL}" \
        --network-passphrase "${NETWORK_PASSPHRASE}" \
        --fee "${FEE}" \
        2>&1 | grep -E '^[A-Z0-9]{56}$' | tail -1)

    if [[ -z "${contract_id}" ]]; then
        echo "  Error: failed to deploy ${name} — no contract ID returned."
        exit 1
    fi

    success "${name} deployed: ${contract_id}"
    info "${alias}=${contract_id}"
    echo "${alias}=${contract_id}" >> "${ENV_FILE}"
    echo "${contract_id}"
}

# ── Pre-flight checks ──────────────────────────────────────────────────────────

echo ""
echo -e "${RED}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${RED}║        MAINNET DEPLOYMENT — IRREVERSIBLE OPERATION       ║${NC}"
echo -e "${RED}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo "  Network:   ${NETWORK}"
echo "  RPC URL:   ${RPC_URL}"
echo "  Deployer:  ${SOURCE}"
echo "  Fee:       ${FEE} stroops per operation"
echo "  Env file:  ${ENV_FILE}"
if [[ "${DRY_RUN}" == "true" ]]; then
    warn "DRY-RUN mode — no transactions will be submitted"
fi
echo ""
warn "Mainnet deployments cannot be undone."
warn "Ensure all contracts have passed security review and testnet verification."
warn "Ensure the deployer account is funded with sufficient XLM."
echo ""
echo "  Press ENTER to begin or Ctrl-C to abort."
read -r

# Verify stellar CLI is available
if ! command -v stellar &>/dev/null; then
    echo "Error: 'stellar' CLI not found. Install it and try again."
    exit 1
fi

# Verify deployer identity exists
if ! stellar keys show "${SOURCE}" &>/dev/null; then
    echo "Error: identity '${SOURCE}' not found. Run 'stellar keys list' to see available keys."
    exit 1
fi

# ── Build ──────────────────────────────────────────────────────────────────────

log "Building all contracts for release..."
(cd "${CONTRACTS_DIR}" && cargo build --release --target wasm32-unknown-unknown 2>&1 | tail -5)
log "Build complete."
echo ""

# ── Start fresh env file ───────────────────────────────────────────────────────

WASM_DIR="${CONTRACTS_DIR}/target/wasm32-unknown-unknown/release"

cat > "${ENV_FILE}" <<EOF
# Mainnet contract IDs — generated by deploy-contracts-mainnet.sh
# Network:    ${NETWORK}
# RPC URL:    ${RPC_URL}
# Passphrase: ${NETWORK_PASSPHRASE}
# Deployer:   ${SOURCE}
# Deployed:   $(date -u +"%Y-%m-%dT%H:%M:%SZ")

EOF

# ── Deploy with approval gates ─────────────────────────────────────────────────

log "Starting mainnet deployment..."

# 1. access-control — no dependencies
echo ""
info "Contract 1/9: access-control (no dependencies)"
approval_gate "access-control"
ACCESS_CONTROL_ID=$(deploy_contract \
    "access-control" \
    "${WASM_DIR}/access_control.wasm" \
    "ACCESS_CONTROL_CONTRACT_ID")

# 2. stellar_insights — depends on access-control
echo ""
info "Contract 2/9: stellar_insights (depends on access-control)"
info "  ACCESS_CONTROL_CONTRACT_ID=${ACCESS_CONTROL_ID}"
approval_gate "stellar_insights"
STELLAR_INSIGHTS_ID=$(deploy_contract \
    "stellar_insights" \
    "${WASM_DIR}/stellar_insights.wasm" \
    "STELLAR_INSIGHTS_CONTRACT_ID")

# 3. analytics — depends on stellar_insights
echo ""
info "Contract 3/9: analytics (depends on stellar_insights)"
info "  STELLAR_INSIGHTS_CONTRACT_ID=${STELLAR_INSIGHTS_ID}"
approval_gate "analytics"
ANALYTICS_ID=$(deploy_contract \
    "analytics" \
    "${WASM_DIR}/analytics.wasm" \
    "ANALYTICS_CONTRACT_ID")

# 4. governance — depends on analytics
echo ""
info "Contract 4/9: governance (depends on analytics)"
info "  ANALYTICS_CONTRACT_ID=${ANALYTICS_ID}"
approval_gate "governance"
GOVERNANCE_ID=$(deploy_contract \
    "governance" \
    "${WASM_DIR}/governance.wasm" \
    "GOVERNANCE_CONTRACT_ID")

# 5. escrow — depends on stellar_insights
echo ""
info "Contract 5/9: escrow (depends on stellar_insights)"
approval_gate "escrow"
ESCROW_ID=$(deploy_contract \
    "escrow" \
    "${WASM_DIR}/escrow.wasm" \
    "ESCROW_CONTRACT_ID")

# 6. token-swap — depends on stellar_insights
echo ""
info "Contract 6/9: token-swap (depends on stellar_insights)"
approval_gate "token-swap"
TOKEN_SWAP_ID=$(deploy_contract \
    "token-swap" \
    "${WASM_DIR}/token_swap.wasm" \
    "TOKEN_SWAP_CONTRACT_ID")

# 7. multi-sig-wallet — depends on access-control
echo ""
info "Contract 7/9: multi-sig-wallet (depends on access-control)"
approval_gate "multi-sig-wallet"
MULTI_SIG_ID=$(deploy_contract \
    "multi-sig-wallet" \
    "${WASM_DIR}/multi_sig_wallet.wasm" \
    "MULTI_SIG_WALLET_CONTRACT_ID")

# 8. time-locked-transactions — depends on access-control
echo ""
info "Contract 8/9: time-locked-transactions (depends on access-control)"
approval_gate "time-locked-transactions"
TIME_LOCKED_ID=$(deploy_contract \
    "time-locked-transactions" \
    "${WASM_DIR}/time_locked_transactions.wasm" \
    "TIME_LOCKED_TRANSACTIONS_CONTRACT_ID")

# 9. upgrade — depends on governance
echo ""
info "Contract 9/9: upgrade (depends on governance)"
info "  GOVERNANCE_CONTRACT_ID=${GOVERNANCE_ID}"
approval_gate "upgrade"
UPGRADE_ID=$(deploy_contract \
    "upgrade" \
    "${WASM_DIR}/upgrade.wasm" \
    "UPGRADE_CONTRACT_ID")

# ── Summary ────────────────────────────────────────────────────────────────────

echo ""
echo -e "${GRN}══════════════════════════════════════════════════════════${NC}"
log "All contracts deployed successfully."
log "Contract IDs written to: ${ENV_FILE}"
echo ""
echo "  ACCESS_CONTROL_CONTRACT_ID=${ACCESS_CONTROL_ID}"
echo "  STELLAR_INSIGHTS_CONTRACT_ID=${STELLAR_INSIGHTS_ID}"
echo "  ANALYTICS_CONTRACT_ID=${ANALYTICS_ID}"
echo "  GOVERNANCE_CONTRACT_ID=${GOVERNANCE_ID}"
echo "  ESCROW_CONTRACT_ID=${ESCROW_ID}"
echo "  TOKEN_SWAP_CONTRACT_ID=${TOKEN_SWAP_ID}"
echo "  MULTI_SIG_WALLET_CONTRACT_ID=${MULTI_SIG_ID}"
echo "  TIME_LOCKED_TRANSACTIONS_CONTRACT_ID=${TIME_LOCKED_ID}"
echo "  UPGRADE_CONTRACT_ID=${UPGRADE_ID}"
echo ""
log "Source the env file with: source ${ENV_FILE}"
log "Run verification:         ./scripts/verify-contract-mainnet.sh"
echo -e "${GRN}══════════════════════════════════════════════════════════${NC}"
