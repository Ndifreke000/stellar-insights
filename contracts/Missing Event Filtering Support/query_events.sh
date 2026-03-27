#!/bin/bash
# Stellar Snapshot Contract - Event Filtering Query Helper
# This script provides common event query patterns

set -e

CONTRACT_ID="${1:?Error: CONTRACT_ID required as first argument}"
NETWORK="${2:-testnet}"

echo "Stellar Event Filtering Query Helper"
echo "====================================="
echo "Contract ID: $CONTRACT_ID"
echo "Network: $NETWORK"
echo ""

# Color codes for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Query 1: All snapshot events
echo -e "${BLUE}[1] Query all snapshot events:${NC}"
echo "stellar contract events --id $CONTRACT_ID \\"
echo "  --topic snapshot \\"
echo "  --network $NETWORK"
echo ""

# Query 2: Snapshots from specific submitter
echo -e "${BLUE}[2] Query snapshots from specific submitter:${NC}"
echo "stellar contract events --id $CONTRACT_ID \\"
echo "  --topic snapshot \\"
echo "  --topic <SUBMITTER_ADDRESS> \\"
echo "  --network $NETWORK"
echo ""

# Query 3: Snapshots for specific epoch
echo -e "${BLUE}[3] Query snapshots for specific epoch:${NC}"
echo "stellar contract events --id $CONTRACT_ID \\"
echo "  --topic snapshot \\"
echo "  --topic <SUBMITTER_ADDRESS> \\"
echo "  --topic <EPOCH_NUMBER> \\"
echo "  --network $NETWORK"
echo ""

# Query 4: Admin change events
echo -e "${BLUE}[4] Query admin change events:${NC}"
echo "stellar contract events --id $CONTRACT_ID \\"
echo "  --topic admin_ch \\"
echo "  --network $NETWORK"
echo ""

# Query 5: Pause events
echo -e "${BLUE}[5] Query pause events:${NC}"
echo "stellar contract events --id $CONTRACT_ID \\"
echo "  --topic pause \\"
echo "  --network $NETWORK"
echo ""

# Query 6: Governance set events
echo -e "${BLUE}[6] Query governance set events:${NC}"
echo "stellar contract events --id $CONTRACT_ID \\"
echo "  --topic gov_set \\"
echo "  --network $NETWORK"
echo ""

echo -e "${GREEN}✓ Common query patterns listed above${NC}"
echo ""
echo "Usage Notes:"
echo "- Replace <SUBMITTER_ADDRESS> with actual submitter address"
echo "- Replace <EPOCH_NUMBER> with actual epoch number"
echo "- Topics are matched in order (first topic must match 'snapshot', etc.)"
echo "- Omit topics for broader queries"
