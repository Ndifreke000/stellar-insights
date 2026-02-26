#!/bin/bash

echo "Starting ELK Stack for Stellar Insights..."

# Start ELK services
docker-compose -f docker-compose.elk.yml up -d

echo "Waiting for Elasticsearch to be ready..."
until curl -s http://localhost:9200/_cluster/health | grep -q '"status":"green\|yellow"'; do
  sleep 2
done

echo "✓ Elasticsearch is ready"
echo "✓ Kibana available at: http://localhost:5601"
echo "✓ Logstash listening on: localhost:5000"
echo ""
echo "Next steps:"
echo "1. Add LOGSTASH_HOST=localhost:5000 to backend/.env"
echo "2. Start backend: cd backend && cargo run"
echo "3. View logs in Kibana: http://localhost:5601"
