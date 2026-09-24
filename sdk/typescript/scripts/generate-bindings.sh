#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$SDK_ROOT/generated"

NETWORK="${NETWORK:-testnet}"
CONTRACT_ID="${CONTRACT_ID:-}"
RPC_URL="${RPC_URL:-https://soroban-testnet.stellar.org}"

usage() {
  echo "Usage: $0 [--contract-id <id>] [--network <network>] [--rpc-url <url>]"
  echo ""
  echo "Generates TypeScript bindings from deployed Soroban contracts."
  echo ""
  echo "Options:"
  echo "  --contract-id   Deployed contract ID (required unless CONTRACT_ID env var set)"
  echo "  --network       Stellar network (default: testnet)"
  echo "  --rpc-url       Soroban RPC URL (default: https://soroban-testnet.stellar.org)"
  echo ""
  echo "Environment variables:"
  echo "  CONTRACT_ID     Alternative to --contract-id flag"
  echo "  NETWORK         Alternative to --network flag"
  echo "  RPC_URL         Alternative to --rpc-url flag"
  exit 1
}

while [[ $# -gt 0 ]]; do
  case $1 in
    --contract-id) CONTRACT_ID="$2"; shift 2 ;;
    --network) NETWORK="$2"; shift 2 ;;
    --rpc-url) RPC_URL="$2"; shift 2 ;;
    --help|-h) usage ;;
    *) echo "Unknown option: $1"; usage ;;
  esac
done

if [[ -z "$CONTRACT_ID" ]]; then
  echo "Error: --contract-id is required (or set CONTRACT_ID env var)"
  exit 1
fi

if ! command -v stellar &>/dev/null; then
  echo "Error: 'stellar' CLI not found. Install from https://soroban.stellar.org/docs/getting-started/setup"
  exit 1
fi

echo "Generating TypeScript bindings..."
echo "  Contract ID: $CONTRACT_ID"
echo "  Network:     $NETWORK"
echo "  RPC URL:     $RPC_URL"
echo "  Output:      $OUTPUT_DIR"

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

stellar contract bindings typescript \
  --contract-id "$CONTRACT_ID" \
  --network "$NETWORK" \
  --rpc-url "$RPC_URL" \
  --output-dir "$OUTPUT_DIR"

echo "Bindings generated successfully in $OUTPUT_DIR"
