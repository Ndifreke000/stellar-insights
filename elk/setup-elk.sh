#!/bin/bash
set -e

echo "🚀 Setting up ELK Stack for PayRaider..."

# Wait for Elasticsearch to be ready
echo "⏳ Waiting for Elasticsearch..."
until curl -s http://localhost:9200/_cluster/health | grep -q '"status":"green"\|"status":"yellow"'; do
  sleep 5
  echo "   Still waiting..."
done
echo "✅ Elasticsearch is ready"

# Create ILM policy
echo "📋 Creating Index Lifecycle Management policy..."
curl -X PUT "http://localhost:9200/_ilm/policy/payraider-policy" \
  -H 'Content-Type: application/json' \
  -d @elk/elasticsearch/config/ilm-policy.json
echo "✅ ILM policy created"

# Create index template
echo "📝 Creating index template..."
curl -X PUT "http://localhost:9200/_index_template/payraider-template" \
  -H 'Content-Type: application/json' \
  -d @elk/elasticsearch/config/index-template.json
echo "✅ Index template created"

# Create initial index with alias
echo "🔗 Creating initial index..."
curl -X PUT "http://localhost:9200/payraider-000001" \
  -H 'Content-Type: application/json' \
  -d '{
  "aliases": {
    "payraider": {
      "is_write_index": true
    }
  }
}'
echo "✅ Initial index created"

# Wait for Kibana to be ready
echo "⏳ Waiting for Kibana..."
until curl -s http://localhost:5601/api/status | grep -q '"level":"available"'; do
  sleep 5
  echo "   Still waiting..."
done
echo "✅ Kibana is ready"

# Import Kibana dashboards
echo "📊 Importing Kibana dashboards..."
curl -X POST "http://localhost:5601/api/saved_objects/_import?overwrite=true" \
  -H "kbn-xsrf: true" \
  --form file=@elk/kibana-dashboards.ndjson
echo "✅ Dashboards imported"

# Create index pattern
echo "🔍 Creating index pattern..."
curl -X POST "http://localhost:5601/api/saved_objects/index-pattern/payraider-pattern" \
  -H "kbn-xsrf: true" \
  -H "Content-Type: application/json" \
  -d '{
  "attributes": {
    "title": "payraider-*",
    "timeFieldName": "@timestamp"
  }
}'
echo "✅ Index pattern created"

echo ""
echo "🎉 ELK Stack setup complete!"
echo ""
echo "📍 Access points:"
echo "   Elasticsearch: http://localhost:9200"
echo "   Kibana:        http://localhost:5601"
echo "   Logstash:      tcp://localhost:5000"
echo ""
echo "📖 Next steps:"
echo "   1. Start your backend: cd backend && cargo run"
echo "   2. View logs in Kibana: http://localhost:5601/app/discover"
echo ""
