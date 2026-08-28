#!/usr/bin/env bash
# Verify deployed mainnet contracts by fetching their on-chain version string.
#
# Usage:
#   ./scripts/verify-contract-mainnet.sh [--env <path>] [--source <identity>]
#
# Prerequisites:
#   - stellar CLI installed
#   - contracts/.env.mainnet present (produced by deploy-contracts-mainnet.sh)
#   - STELLAR_ACCOUNT env var set, or pass --source <identity>

set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────

CONTRACTS_DIR="$(cd "$(dirname "$0")/../contracts" && pwd)"
ENV_FILE="${CONTRACTS_DIR}/.env.mainnet"
NETWORK="mainnet"
RPC_URL="https://mainnet.sorobanrpc.com"
NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"

# ── Parse arguments ────────────────────────────────────────────────────────────

SOURCE="${STELLAR_ACCOUNT:-}"

while [[ $# -gt 0 ]]; do
    case $1 in
        --env)    ENV_FILE="$2"; shift 2 ;;
        --source) SOURCE="$2";   shift 2 ;;
        *) echo "Unknown argument: $1"; exit 1 ;;
    esac
done

if [[ -z "${SOURCE}" ]]; then
    echo "Error: source identity not set."
    echo "  Set STELLAR_ACCOUNT env var or pass --source <identity>"
    exit 1
fi

# ── Helpers ────────────────────────────────────────────────────────────────────

GRN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

log()     { echo "[$(date -u +%H:%M:%S)] $*"; }
pass()    { echo -e "  ${GRN}PASS${NC}  $*"; }
fail()    { echo -e "  ${RED}FAIL${NC}  $*"; FAILURES=$((FAILURES + 1)); }
FAILURES=0

# Load env file
if [[ ! -f "${ENV_FILE}" ]]; then
    echo "Error: env file not found: ${ENV_FILE}"
    echo "  Run deploy-contracts-mainnet.sh first."
    exit 1
fi

# shellcheck disable=SC1090
source "${ENV_FILE}"

invoke_view() {
    local contract_id="$1"
    local fn_name="$2"

    stellar contract invoke \
        --id "${contract_id}" \
        --source "${SOURCE}" \
        --network "${NETWORK}" \
        --rpc-url "${RPC_URL}" \
        --network-passphrase "${NETWORK_PASSPHRASE}" \
        -- "${fn_name}" 2>/dev/null || true
}

verify_contract() {
    local name="$1"
    local contract_id_var="$2"
    local fn_name="${3:-get_version}"

    local contract_id="${!contract_id_var:-}"
    if [[ -z "${contract_id}" ]]; then
        fail "${name}: ${contract_id_var} not set in env file"
        return
    fi

    local result
    result=$(invoke_view "${contract_id}" "${fn_name}" 2>&1)

    if [[ -n "${result}" && "${result}" != *"error"* && "${result}" != *"Error"* ]]; then
        pass "${name} (${contract_id}): ${result}"
    else
        fail "${name} (${contract_id}): no version returned — ${result}"
    fi
}

# ── Verify each contract ───────────────────────────────────────────────────────

echo ""
log "Verifying mainnet contracts from: ${ENV_FILE}"
echo ""

verify_contract "access-control"          "ACCESS_CONTROL_CONTRACT_ID"
verify_contract "stellar_insights"        "STELLAR_INSIGHTS_CONTRACT_ID"
verify_contract "analytics"               "ANALYTICS_CONTRACT_ID"
verify_contract "governance"              "GOVERNANCE_CONTRACT_ID"
verify_contract "escrow"                  "ESCROW_CONTRACT_ID"
verify_contract "token-swap"              "TOKEN_SWAP_CONTRACT_ID"
verify_contract "multi-sig-wallet"        "MULTI_SIG_WALLET_CONTRACT_ID"
verify_contract "time-locked-transactions" "TIME_LOCKED_TRANSACTIONS_CONTRACT_ID"
verify_contract "upgrade"                 "UPGRADE_CONTRACT_ID"

# ── Summary ────────────────────────────────────────────────────────────────────

echo ""
if [[ "${FAILURES}" -eq 0 ]]; then
    echo -e "${GRN}All contracts verified successfully.${NC}"
else
    echo -e "${RED}${FAILURES} contract(s) failed verification. Review output above.${NC}"
    exit 1
fi
