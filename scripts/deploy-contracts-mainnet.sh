#!/usr/bin/env bash
# Deploy the integrated Soroban contract to mainnet with a human approval gate.
#
# Only stellar_insights is deployed — see contracts/archive/README.md.

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
        echo "  Run 'cd contracts && cargo build --release --target wasm32v1-none' first."
        exit 1
    fi

    if [[ "${DRY_RUN}" == "true" ]]; then
        local fake_id="DRYRUN_${alias}"
        info "[dry-run] would deploy ${name} → ${alias}=${fake_id}"
        echo "${alias}=${fake_id}" >> "${ENV_FILE}"
        echo "${fake_id}"
        return
    fi

    # `|| true` on the pipeline: with `set -e`, a transient deploy failure
    # (grep finding no 56-char ID) would otherwise kill the script right here,
    # silently skipping the error message below — this way the `-z` check
    # gets a chance to report which contract failed before exiting.
    local contract_id
    contract_id=$(stellar contract deploy \
        --wasm "${wasm}" \
        --source "${SOURCE}" \
        --network "${NETWORK}" \
        --rpc-url "${RPC_URL}" \
        --network-passphrase "${NETWORK_PASSPHRASE}" \
        --fee "${FEE}" \
        2>&1 | grep -E '^[A-Z0-9]{56}$' | tail -1) || true

    if [[ -z "${contract_id}" ]]; then
        echo "  Error: failed to deploy ${name} — no contract ID returned."
        echo "  This can be a transient RPC/network hiccup — try re-running."
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
(cd "${CONTRACTS_DIR}" && cargo build --release --target wasm32v1-none 2>&1 | tail -5)
log "Build complete."
echo ""

# ── Start fresh env file ───────────────────────────────────────────────────────

WASM_DIR="${CONTRACTS_DIR}/target/wasm32v1-none/release"

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

echo ""
info "Contract: stellar_insights"
approval_gate "stellar_insights"
STELLAR_INSIGHTS_ID=$(deploy_contract \
    "stellar_insights" \
    "${WASM_DIR}/stellar_insights.wasm" \
    "STELLAR_INSIGHTS_CONTRACT_ID")

# ── Summary ────────────────────────────────────────────────────────────────────

echo ""
echo -e "${GRN}══════════════════════════════════════════════════════════${NC}"
log "Contract deployed successfully."
log "Contract IDs written to: ${ENV_FILE}"
echo ""
echo "  STELLAR_INSIGHTS_CONTRACT_ID=${STELLAR_INSIGHTS_ID}"
echo ""
log "Source the env file with: source ${ENV_FILE}"
log "Run verification:         ./scripts/verify-contract-mainnet.sh"
echo -e "${GRN}══════════════════════════════════════════════════════════${NC}"
