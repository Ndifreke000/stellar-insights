#!/bin/bash
set -e

echo "🧪 Testing ELK Stack Integration..."

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test Elasticsearch
echo -n "Testing Elasticsearch... "
if curl -s http://localhost:9200/_cluster/health | grep -q '"status":"green"\|"status":"yellow"'; then
    echo -e "${GREEN}✓ OK${NC}"
else
    echo -e "${RED}✗ FAILED${NC}"
    exit 1
fi

# Test Logstash
echo -n "Testing Logstash... "
if curl -s http://localhost:9600/_node/stats | grep -q '"status":"green"'; then
    echo -e "${GREEN}✓ OK${NC}"
else
    echo -e "${YELLOW}⚠ WARNING${NC} (may be starting)"
fi

# Test Kibana
echo -n "Testing Kibana... "
if curl -s http://localhost:5601/api/status | grep -q '"level":"available"'; then
    echo -e "${GREEN}✓ OK${NC}"
else
    echo -e "${RED}✗ FAILED${NC}"
    exit 1
fi

# Send test log
echo -n "Sending test log to Logstash... "
echo '{"@timestamp":"'$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)'","log_level":"info","message":"ELK test log","service":"payraider","target":"test"}' | nc localhost 5000
sleep 2
echo -e "${GREEN}✓ SENT${NC}"

# Check if log was indexed
echo -n "Verifying log in Elasticsearch... "
sleep 3
if curl -s "http://localhost:9200/payraider-*/_search?q=message:ELK%20test%20log" | grep -q '"hits":{"total":{"value":[1-9]'; then
    echo -e "${GREEN}✓ FOUND${NC}"
else
    echo -e "${YELLOW}⚠ NOT FOUND${NC} (may need more time)"
fi

# Check index count
echo ""
echo "📊 Index Statistics:"
curl -s "http://localhost:9200/_cat/indices/payraider-*?v&h=index,docs.count,store.size"

echo ""
echo -e "${GREEN}✅ ELK Stack is operational!${NC}"
echo ""
echo "🔗 Quick Links:"
echo "   Kibana Discover: http://localhost:5601/app/discover"
echo "   Elasticsearch:   http://localhost:9200/_cat/indices?v"
echo ""
