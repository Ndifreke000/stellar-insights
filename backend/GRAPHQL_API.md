# GraphQL API Documentation

The Stellar Insights backend now includes a GraphQL API layer alongside the existing REST API, enabling clients to request exactly the data they need with flexible querying capabilities.

## Overview

GraphQL provides a more efficient alternative to REST by allowing clients to:
- Request only the fields they need
- Fetch multiple resources in a single request
- Explore the API through introspection
- Benefit from strong typing and validation

## Endpoints

### GraphQL Endpoint
- **URL**: `POST /graphql`
- **Content-Type**: `application/json`
- **Purpose**: Execute GraphQL queries and mutations

### GraphQL Playground
- **URL**: `GET /graphql/playground`
- **Purpose**: Interactive GraphQL IDE for exploring and testing queries
- **Access**: Open in browser at `http://localhost:8080/graphql/playground`

## Available Queries

### Anchors

#### Get Single Anchor
```graphql
query {
  anchor(id: "550e8400-e29b-41d4-a716-446655440000") {
    id
    name
    stellarAccount
    homeDomain
    totalTransactions
    successfulTransactions
    failedTransactions
    totalVolumeUsd
    avgSettlementTimeMs
    reliabilityScore
    status
    createdAt
    updatedAt
  }
}
```

#### Get All Anchors with Filtering
```graphql
query {
  anchors(
    filter: {
      status: "green"
      minReliabilityScore: 80.0
      search: "Circle"
    }
    pagination: {
      limit: 10
      offset: 0
    }
  ) {
    nodes {
      id
      name
      stellarAccount
      reliabilityScore
      status
    }
    totalCount
    hasNextPage
  }
}
```

### Corridors

#### Get Single Corridor
```graphql
query {
  corridor(id: "corridor-123") {
    id
    sourceAssetCode
    sourceAssetIssuer
    destinationAssetCode
    destinationAssetIssuer
    reliabilityScore
    status
    createdAt
    updatedAt
  }
}
```

#### Get All Corridors with Filtering
```graphql
query {
  corridors(
    filter: {
      sourceAssetCode: "USDC"
      destinationAssetCode: "EUR"
      status: "active"
      minReliabilityScore: 75.0
    }
    pagination: {
      limit: 20
      offset: 0
    }
  ) {
    nodes {
      id
      sourceAssetCode
      destinationAssetCode
      reliabilityScore
      status
    }
    totalCount
    hasNextPage
  }
}
```

### Assets

#### Get Assets by Anchor
```graphql
query {
  assetsByAnchor(anchorId: "anchor-123") {
    id
    assetCode
    assetIssuer
    totalSupply
    numHolders
    createdAt
  }
}
```

### Metrics

#### Get Metrics with Time Range
```graphql
query {
  metrics(
    entityId: "anchor-123"
    entityType: "anchor"
    timeRange: {
      start: "2024-01-01T00:00:00Z"
      end: "2024-01-31T23:59:59Z"
    }
    pagination: {
      limit: 100
      offset: 0
    }
  ) {
    id
    name
    value
    entityId
    entityType
    timestamp
  }
}
```

### Snapshots

#### Get Latest Snapshot
```graphql
query {
  latestSnapshot(
    entityId: "anchor-123"
    entityType: "anchor"
  ) {
    id
    entityId
    entityType
    data
    hash
    epoch
    timestamp
  }
}
```

### Search

#### Search Across Multiple Entity Types
```graphql
query {
  search(query: "USDC", limit: 10) {
    anchors {
      id
      name
      stellarAccount
    }
    corridors {
      id
      sourceAssetCode
      destinationAssetCode
    }
  }
}
```

## Complex Queries

### Fetch Anchor with Assets and Metrics
```graphql
query {
  anchor(id: "anchor-123") {
    id
    name
    reliabilityScore
    status
  }
  
  assetsByAnchor(anchorId: "anchor-123") {
    assetCode
    numHolders
    totalSupply
  }
  
  metrics(
    entityId: "anchor-123"
    entityType: "anchor"
    pagination: { limit: 10 }
  ) {
    name
    value
    timestamp
  }
}
```

### Dashboard Query
```graphql
query Dashboard {
  topAnchors: anchors(
    filter: { minReliabilityScore: 90.0 }
    pagination: { limit: 5 }
  ) {
    nodes {
      name
      reliabilityScore
      totalTransactions
      totalVolumeUsd
    }
  }
  
  activeCorridors: corridors(
    filter: { status: "active" }
    pagination: { limit: 10 }
  ) {
    nodes {
      sourceAssetCode
      destinationAssetCode
      reliabilityScore
    }
    totalCount
  }
}
```

## Field Selection

One of GraphQL's key benefits is requesting only the fields you need:

```graphql
# Minimal query - only IDs and names
query {
  anchors(pagination: { limit: 100 }) {
    nodes {
      id
      name
    }
  }
}

# Full query - all available fields
query {
  anchors(pagination: { limit: 10 }) {
    nodes {
      id
      name
      stellarAccount
      homeDomain
      totalTransactions
      successfulTransactions
      failedTransactions
      totalVolumeUsd
      avgSettlementTimeMs
      reliabilityScore
      status
      createdAt
      updatedAt
    }
    totalCount
    hasNextPage
  }
}
```

## Pagination

All list queries support pagination:

```graphql
query {
  anchors(
    pagination: {
      limit: 20    # Items per page (max: 100)
      offset: 40   # Skip first 40 items (page 3)
    }
  ) {
    nodes {
      id
      name
    }
    totalCount      # Total items available
    hasNextPage     # Whether more items exist
  }
}
```

## Filtering

### Anchor Filters
- `status`: Filter by status (green, yellow, red)
- `minReliabilityScore`: Minimum reliability score (0-100)
- `search`: Search by name or stellar account

### Corridor Filters
- `sourceAssetCode`: Filter by source asset
- `destinationAssetCode`: Filter by destination asset
- `status`: Filter by status (active, inactive)
- `minReliabilityScore`: Minimum reliability score (0-100)

## Client Examples

### JavaScript/TypeScript (fetch)
```javascript
const query = `
  query {
    anchors(pagination: { limit: 10 }) {
      nodes {
        id
        name
        reliabilityScore
      }
    }
  }
`;

const response = await fetch('http://localhost:8080/graphql', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({ query }),
});

const data = await response.json();
console.log(data.data.anchors);
```

### JavaScript/TypeScript (Apollo Client)
```typescript
import { ApolloClient, InMemoryCache, gql } from '@apollo/client';

const client = new ApolloClient({
  uri: 'http://localhost:8080/graphql',
  cache: new InMemoryCache(),
});

const GET_ANCHORS = gql`
  query GetAnchors($limit: Int!) {
    anchors(pagination: { limit: $limit }) {
      nodes {
        id
        name
        reliabilityScore
      }
      totalCount
    }
  }
`;

const { data } = await client.query({
  query: GET_ANCHORS,
  variables: { limit: 10 },
});
```

### Python (requests)
```python
import requests

query = """
query {
  anchors(pagination: { limit: 10 }) {
    nodes {
      id
      name
      reliabilityScore
    }
  }
}
"""

response = requests.post(
    'http://localhost:8080/graphql',
    json={'query': query},
    headers={'Content-Type': 'application/json'}
)

data = response.json()
print(data['data']['anchors'])
```

### Rust (reqwest)
```rust
use serde_json::json;

let query = r#"
query {
  anchors(pagination: { limit: 10 }) {
    nodes {
      id
      name
      reliabilityScore
    }
  }
}
"#;

let client = reqwest::Client::new();
let response = client
    .post("http://localhost:8080/graphql")
    .json(&json!({ "query": query }))
    .send()
    .await?
    .json::<serde_json::Value>()
    .await?;

println!("{:?}", response["data"]["anchors"]);
```

### cURL
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query { anchors(pagination: { limit: 5 }) { nodes { id name reliabilityScore } } }"
  }'
```

## Error Handling

GraphQL returns errors in a structured format:

```json
{
  "data": null,
  "errors": [
    {
      "message": "Field 'invalidField' not found on type 'Anchor'",
      "locations": [{ "line": 3, "column": 5 }],
      "path": ["anchor", "invalidField"]
    }
  ]
}
```

## Performance Considerations

### Advantages
- **Reduced Over-fetching**: Request only needed fields
- **Reduced Under-fetching**: Get related data in one request
- **Efficient Queries**: Combine multiple queries into one request

### Best Practices
1. Use pagination for large datasets
2. Request only necessary fields
3. Leverage filtering to reduce data transfer
4. Use the playground to test queries before implementation
5. Monitor query complexity in production

## Schema Introspection

GraphQL supports introspection to explore the schema:

```graphql
query {
  __schema {
    types {
      name
      description
    }
  }
}
```

Or query specific types:

```graphql
query {
  __type(name: "Anchor") {
    name
    fields {
      name
      type {
        name
      }
    }
  }
}
```

## Comparison: REST vs GraphQL

### REST API
```bash
# Multiple requests needed
GET /api/anchors/123
GET /api/anchors/123/assets
GET /api/metrics?entity_id=123&entity_type=anchor
```

### GraphQL API
```graphql
# Single request
query {
  anchor(id: "123") {
    id
    name
    reliabilityScore
  }
  assetsByAnchor(anchorId: "123") {
    assetCode
    numHolders
  }
  metrics(entityId: "123", entityType: "anchor") {
    name
    value
  }
}
```

## Rate Limiting

GraphQL endpoints are subject to the same rate limiting as REST endpoints:
- Default: 100 requests per minute
- Configurable per endpoint
- Whitelisting available for trusted IPs

## Future Enhancements

Planned features for future releases:
- **Subscriptions**: Real-time updates via WebSocket
- **Mutations**: Create and update operations
- **DataLoader**: Batch and cache database queries
- **Query Complexity Analysis**: Prevent expensive queries
- **Persisted Queries**: Improve performance and security
- **Federation**: Combine multiple GraphQL services

## Troubleshooting

### Common Issues

**Issue**: "Field not found" error
**Solution**: Check field names match schema (use camelCase)

**Issue**: Slow queries
**Solution**: Use pagination and request fewer fields

**Issue**: CORS errors
**Solution**: CORS is enabled by default; check client configuration

**Issue**: Authentication required
**Solution**: Some queries may require authentication headers

## Support

For questions or issues:
- Use the GraphQL Playground for interactive exploration
- Check the schema documentation via introspection
- Review query examples in this documentation
- Open GitHub issues for bugs or feature requests
