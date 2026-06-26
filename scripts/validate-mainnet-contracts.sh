#!/usr/bin/env bash
set -euo pipefail

# Post-deployment canary: validate mainnet stellar_insights contract state.
#
# Required env:
#   STELLAR_INSIGHTS_CONTRACT_ID (or SNAPSHOT_CONTRACT_ID)
# Optional env:
#   STELLAR_NETWORK              (default: mainnet)
#   EXPECTED_CONTRACT_VERSION    (default: contracts/stellar_insights/Cargo.toml version)
#   EXPECTED_CONTRACT_ADMIN      (required for admin check)
#   EPOCH_BASELINE_FILE          (path to store/compare latest epoch between runs)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

CONTRACT_ID="${STELLAR_INSIGHTS_CONTRACT_ID:-${SNAPSHOT_CONTRACT_ID:-}}"
NETWORK="${STELLAR_NETWORK:-mainnet}"
EXPECTED_ADMIN="${EXPECTED_CONTRACT_ADMIN:-}"
EPOCH_BASELINE_FILE="${EPOCH_BASELINE_FILE:-${REPO_ROOT}/.mainnet-epoch-baseline}"

if [[ -z "${CONTRACT_ID}" ]]; then
  echo "ERROR: Set STELLAR_INSIGHTS_CONTRACT_ID or SNAPSHOT_CONTRACT_ID" >&2
  exit 1
fi

if ! command -v stellar >/dev/null 2>&1; then
  echo "ERROR: stellar CLI is required" >&2
  exit 1
fi

if [[ -z "${EXPECTED_CONTRACT_VERSION:-}" ]]; then
  EXPECTED_CONTRACT_VERSION="$(
    grep '^version' "${REPO_ROOT}/contracts/stellar_insights/Cargo.toml" \
      | head -1 \
      | cut -d'"' -f2
  )"
fi

invoke_readonly() {
  local fn="$1"
  shift
  stellar contract invoke \
    --id "${CONTRACT_ID}" \
    --network "${NETWORK}" \
    --send=no \
    -- "${fn}" "$@"
}

echo "Validating contract ${CONTRACT_ID} on ${NETWORK}..."

INFO_JSON="$(invoke_readonly get_contract_info)"
echo "Contract info: ${INFO_JSON}"

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq is required to parse contract output" >&2
  exit 1
fi

INITIALIZED="$(echo "${INFO_JSON}" | jq -r '.initialized // false')"
PAUSED="$(echo "${INFO_JSON}" | jq -r '.paused // true')"
VERSION="$(echo "${INFO_JSON}" | jq -r '.metadata.version // empty')"
ADMIN="$(echo "${INFO_JSON}" | jq -r 'if .admin == null then empty elif (.admin | type) == "string" then .admin elif .admin.Some then .admin.Some else empty end')"

if [[ "${INITIALIZED}" != "true" ]]; then
  echo "FAIL: contract is not initialized (expected initialized: true)" >&2
  exit 1
fi

if [[ "${PAUSED}" != "false" ]]; then
  echo "FAIL: contract is paused (expected paused: false)" >&2
  exit 1
fi

if [[ -z "${VERSION}" || "${VERSION}" != "${EXPECTED_CONTRACT_VERSION}" ]]; then
  echo "FAIL: version mismatch (got '${VERSION}', expected '${EXPECTED_CONTRACT_VERSION}')" >&2
  exit 1
fi

if [[ -n "${EXPECTED_ADMIN}" ]]; then
  if [[ "${ADMIN}" != "${EXPECTED_ADMIN}" ]]; then
    echo "FAIL: admin mismatch (got '${ADMIN}', expected '${EXPECTED_ADMIN}')" >&2
    exit 1
  fi
else
  echo "WARN: EXPECTED_CONTRACT_ADMIN not set; skipping admin address check"
fi

LATEST_EPOCH_JSON="$(invoke_readonly get_latest_epoch)"
LATEST_EPOCH="$(echo "${LATEST_EPOCH_JSON}" | jq -r 'if type == "number" then . else .result // . end')"
echo "Latest epoch: ${LATEST_EPOCH}"

if [[ "${LATEST_EPOCH}" == "0" || "${LATEST_EPOCH}" == "null" || -z "${LATEST_EPOCH}" ]]; then
  echo "FAIL: latest epoch is not advancing (value: ${LATEST_EPOCH})" >&2
  exit 1
fi

if [[ -f "${EPOCH_BASELINE_FILE}" ]]; then
  PREVIOUS_EPOCH="$(tr -d '[:space:]' < "${EPOCH_BASELINE_FILE}")"
  if [[ "${LATEST_EPOCH}" =~ ^[0-9]+$ && "${PREVIOUS_EPOCH}" =~ ^[0-9]+$ ]]; then
    if (( LATEST_EPOCH < PREVIOUS_EPOCH )); then
      echo "FAIL: latest epoch regressed (${LATEST_EPOCH} < ${PREVIOUS_EPOCH})" >&2
      exit 1
    fi
    if (( LATEST_EPOCH == PREVIOUS_EPOCH )); then
      echo "WARN: latest epoch unchanged since last canary run (${LATEST_EPOCH})"
    else
      echo "PASS: latest epoch advanced (${PREVIOUS_EPOCH} -> ${LATEST_EPOCH})"
    fi
  fi
else
  echo "INFO: no epoch baseline file yet; storing ${LATEST_EPOCH}"
fi

echo "${LATEST_EPOCH}" > "${EPOCH_BASELINE_FILE}"

echo "SUCCESS: mainnet contract canary checks passed"
