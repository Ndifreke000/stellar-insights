# GraphQL Example Queries for Testing

## Quick Test Queries

### 1. Health Check - Get First Anchor
```graphql
query {
  anchors(pagination: { limit: 1 }) {
    nodes {
      id
      name
    }
    totalCount
  }
}
```

### 2. Get Anchor Details
```graphql
query {
  anchor(id: "YOUR_ANCHOR_ID") {
    id
    name
    stellarAccount
    reliabilityScore
    status
  }
}
```

### 3. Search Test
```graphql
query {
  search(query: "USD", limit: 5) {
    anchors {
      id
      name
    }
    corridors {
      id
      sourceAssetCode
      destinationAssetCode
    }
  }
}
```

### 4. Filtered Anchors
```graphql
query {
  anchors(
    filter: {
      status: "green"
      minReliabilityScore: 50.0
    }
    pagination: { limit: 5 }
  ) {
    nodes {
      name
      reliabilityScore
      status
    }
    totalCount
    hasNextPage
  }
}
```

### 5. Corridors with Pagination
```graphql
query {
  corridors(pagination: { limit: 10, offset: 0 }) {
    nodes {
      id
      sourceAssetCode
      destinationAssetCode
      reliabilityScore
    }
    totalCount
    hasNextPage
  }
}
```

## cURL Test Commands

### Test GraphQL Endpoint
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ anchors(pagination: {limit: 1}) { nodes { id name } totalCount } }"}'
```

### Test with Variables
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query GetAnchors($limit: Int!) { anchors(pagination: {limit: $limit}) { nodes { id name } } }",
    "variables": {"limit": 5}
  }'
```

### Test Playground Access
```bash
curl http://localhost:8080/graphql/playground
```

## Expected Responses

### Success Response
```json
{
  "data": {
    "anchors": {
      "nodes": [
        {
          "id": "123",
          "name": "Example Anchor"
        }
      ],
      "totalCount": 1
    }
  }
}
```

### Error Response
```json
{
  "errors": [
    {
      "message": "Field 'invalidField' not found",
      "locations": [{"line": 2, "column": 5}]
    }
  ]
}
```

## Testing Checklist

- [ ] GraphQL endpoint responds at `/graphql`
- [ ] Playground loads at `/graphql/playground`
- [ ] Basic anchor query works
- [ ] Pagination works correctly
- [ ] Filtering works correctly
- [ ] Search functionality works
- [ ] Error handling works properly
- [ ] CORS headers are present
- [ ] Rate limiting applies to GraphQL
