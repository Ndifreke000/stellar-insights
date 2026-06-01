#!/bin/bash

# Seed sample anchor data for testing
# Usage: ./seed_data.sh [base_url]

BASE_URL="${1:-http://localhost:8080}"

echo "🌱 Seeding sample anchor data to $BASE_URL"
echo ""

# Function to create anchor and get ID
create_anchor() {
    local name=$1
    local account=$2
    local domain=$3
    
    echo "Creating anchor: $name"
    response=$(curl -s -X POST "$BASE_URL/api/anchors" \
        -H "Content-Type: application/json" \
        -d "{
            \"name\": \"$name\",
            \"stellar_account\": \"$account\",
            \"home_domain\": \"$domain\"
        }")
    
    anchor_id=$(echo "$response" | jq -r '.id')
    echo "✓ Created anchor: $name (ID: $anchor_id)"
    echo "$anchor_id"
}

# Function to add asset
add_asset() {
    local anchor_id=$1
    local code=$2
    local issuer=$3
    
    echo "  Adding asset: $code"
    curl -s -X POST "$BASE_URL/api/anchors/$anchor_id/assets" \
        -H "Content-Type: application/json" \
        -d "{
            \"asset_code\": \"$code\",
            \"asset_issuer\": \"$issuer\"
        }" > /dev/null
    echo "  ✓ Added asset: $code"
}

# Function to update metrics
update_metrics() {
    local anchor_id=$1
    local total=$2
    local success=$3
    local failed=$4
    local settlement=$5
    local volume=$6
    
    echo "  Updating metrics..."
    curl -s -X PUT "$BASE_URL/api/anchors/$anchor_id/metrics" \
        -H "Content-Type: application/json" \
        -d "{
            \"total_transactions\": $total,
            \"successful_transactions\": $success,
            \"failed_transactions\": $failed,
            \"avg_settlement_time_ms\": $settlement,
            \"volume_usd\": $volume
        }" > /dev/null
    echo "  ✓ Updated metrics"
}

# Create Circle (Green status - highly reliable)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
circle_id=$(create_anchor "Circle" "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN" "circle.com")
add_asset "$circle_id" "USDC" "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
add_asset "$circle_id" "EURC" "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
update_metrics "$circle_id" 10000 9900 100 2000 1500000.00
echo ""

# Create AnchorUSD (Yellow status - caution)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
anchorusd_id=$(create_anchor "AnchorUSD" "GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX" "anchorusd.com")
add_asset "$anchorusd_id" "USD" "GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX"
update_metrics "$anchorusd_id" 5000 4800 200 4500 750000.00
echo ""

# Create MoneyGram (Green status)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
moneygram_id=$(create_anchor "MoneyGram Access" "GA7FCCMTTSUIC37PODEL6EOOSPDRILP6OQI5FWCWDDVDBLJV72W6RINZ" "moneygram.com")
add_asset "$moneygram_id" "MGI" "GA7FCCMTTSUIC37PODEL6EOOSPDRILP6OQI5FWCWDDVDBLJV72W6RINZ"
update_metrics "$moneygram_id" 8500 8415 85 1800 980000.00
echo ""

# Create Vibrant (Yellow status)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
vibrant_id=$(create_anchor "Vibrant" "GBHFGY3ZNEJWLNO4LBUKLYOCEK4V7ENEBJGPRHHX7JU47GWHBREH37UR" "vibrantapp.com")
add_asset "$vibrant_id" "VELO" "GBHFGY3ZNEJWLNO4LBUKLYOCEK4V7ENEBJGPRHHX7JU47GWHBREH37UR"
update_metrics "$vibrant_id" 3200 3040 160 5200 420000.00
echo ""

# Create Stellar (Green status - native)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
stellar_id=$(create_anchor "Stellar Development Foundation" "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN7" "stellar.org")
add_asset "$stellar_id" "XLM" "native"
update_metrics "$stellar_id" 50000 49500 500 1500 5000000.00
echo ""

# Create a Red status anchor
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
test_id=$(create_anchor "TestNet Anchor" "GCTESTANCHOR12345678901234567890123456789012345" "testanchor.io")
add_asset "$test_id" "TEST" "GCTESTANCHOR12345678901234567890123456789012345"
update_metrics "$test_id" 2000 1800 200 8500 100000.00
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Sample data seeded successfully!"
echo ""
echo "View all anchors:"
echo "  curl $BASE_URL/api/anchors | jq"
echo ""
echo "View specific anchor:"
echo "  curl $BASE_URL/api/anchors/$circle_id | jq"
