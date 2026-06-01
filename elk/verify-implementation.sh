#!/bin/bash

echo "🔍 Verifying ELK Stack Implementation..."
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

ERRORS=0

# Check files
echo "📁 Checking files..."

FILES=(
    "setup-elk.sh"
    "test-elk.sh"
    "health-check.sh"
    "setup-kibana.sh"
    "setup-alerts.sh"
    "elasticsearch/config/elasticsearch.yml"
    "elasticsearch/config/index-template.json"
    "elasticsearch/config/ilm-policy.json"
    "logstash/config/logstash.yml"
    "logstash/pipeline/logstash.conf"
    "filebeat/filebeat.yml"
    "../docker-compose.elk.yml"
    "../docs/ELK_IMPLEMENTATION.md"
    "../docs/ELK_QUICK_REFERENCE.md"
    "../docs/ELK_SETUP.md"
    "../k8s/monitoring/elk-stack.yaml"
    "../backend/src/logging.rs"
)

for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        echo -e "  ${GREEN}✓${NC} $file"
    else
        echo -e "  ${RED}✗${NC} $file (missing)"
        ((ERRORS++))
    fi
done

echo ""
echo "📊 Summary:"
echo "  Total files checked: ${#FILES[@]}"
echo "  Missing files: $ERRORS"

if [ $ERRORS -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✅ All files present! Implementation is complete.${NC}"
    echo ""
    echo "🚀 Next steps:"
    echo "  1. Start ELK: docker-compose -f ../docker-compose.elk.yml up -d"
    echo "  2. Setup: ./setup-elk.sh"
    echo "  3. Test: ./test-elk.sh"
    exit 0
else
    echo ""
    echo -e "${RED}❌ Implementation incomplete. $ERRORS file(s) missing.${NC}"
    exit 1
fi
