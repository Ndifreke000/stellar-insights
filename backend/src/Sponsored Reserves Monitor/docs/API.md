# API Documentation

## Base URL

```
http://localhost:3000
```

## Response Format

All API responses are in JSON format. Successful responses include the data, and error responses include an error message.

### Success Response

```json
{
  "data": {...}
}
```

### Error Response

```json
{
  "error": "Error message"
}
```

## Endpoints

### 1. Get All Sponsorships

**Endpoint**: `GET /api/sponsorships`

**Query Parameters**:

- `limit` (optional): Maximum number of results to return (default: 50, max: 1000)
- `offset` (optional): Number of results to skip (default: 0)

**Example Request**:

```bash
GET /api/sponsorships?limit=50&offset=0
```

**Response**:

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "sponsor": "GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S",
    "sponsored_account": "GCKFBEOZXN3IXMF64APXMC7FXZUQZU6ZGWKD2XXWQVM3CWDUSXRKQFB",
    "sponsored_reserves": 2,
    "total_amount": "100.00",
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
]
```

---

### 2. Create Sponsorship

**Endpoint**: `POST /api/sponsorships`

**Request Body**:

```json
{
  "sponsor": "GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S",
  "sponsored_account": "GCKFBEOZXN3IXMF64APXMC7FXZUQZU6ZGWKD2XXWQVM3CWDUSXRKQFB",
  "sponsored_reserves": 2,
  "total_amount": "100.00"
}
```

**Response Status**: `201 Created`

**Response Body**:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "sponsor": "GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S",
  "sponsored_account": "GCKFBEOZXN3IXMF64APXMC7FXZUQZU6ZGWKD2XXWQVM3CWDUSXRKQFB",
  "sponsored_reserves": 2,
  "total_amount": "100.00",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

**Errors**:

- `400 Bad Request`: Invalid request body
- `500 Internal Server Error`: Database error

---

### 3. Get Sponsorship by ID

**Endpoint**: `GET /api/sponsorships/:id`

**Path Parameters**:

- `id`: Sponsorship UUID

**Example Request**:

```bash
GET /api/sponsorships/550e8400-e29b-41d4-a716-446655440000
```

**Response**:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "sponsor": "GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S",
  "sponsored_account": "GCKFBEOZXN3IXMF64APXMC7FXZUQZU6ZGWKD2XXWQVM3CWDUSXRKQFB",
  "sponsored_reserves": 2,
  "total_amount": "100.00",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

**Errors**:

- `404 Not Found`: Sponsorship not found

---

### 4. Get Sponsorship History

**Endpoint**: `GET /api/sponsorships/:id/history`

**Path Parameters**:

- `id`: Sponsorship UUID

**Example Request**:

```bash
GET /api/sponsorships/550e8400-e29b-41d4-a716-446655440000/history
```

**Response**:

```json
[
  {
    "id": "660e8400-e29b-41d4-a716-446655440001",
    "sponsorship_id": "550e8400-e29b-41d4-a716-446655440000",
    "change_type": "CREATED",
    "previous_amount": null,
    "new_amount": "100.00",
    "created_at": "2024-01-15T10:30:00Z"
  },
  {
    "id": "660e8400-e29b-41d4-a716-446655440002",
    "sponsorship_id": "550e8400-e29b-41d4-a716-446655440000",
    "change_type": "UPDATED",
    "previous_amount": "100.00",
    "new_amount": "150.00",
    "created_at": "2024-01-15T11:00:00Z"
  }
]
```

---

### 5. Get Sponsor Leaderboard

**Endpoint**: `GET /api/sponsors/leaderboard`

**Query Parameters**:

- `limit` (optional): Number of top sponsors to return (default: 100, max: 1000)

**Example Request**:

```bash
GET /api/sponsors/leaderboard?limit=50
```

**Response**:

```json
[
  {
    "sponsor": "GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S",
    "total_sponsored_amount": "500.00",
    "sponsored_accounts_count": 5,
    "rank": 1
  },
  {
    "sponsor": "GCKFBEOZXN3IXMF64APXMC7FXZUQZU6ZGWKD2XXWQVM3CWDUSXRKQFB",
    "total_sponsored_amount": "300.00",
    "sponsored_accounts_count": 3,
    "rank": 2
  }
]
```

---

### 6. Get Sponsorships by Account

**Endpoint**: `GET /api/sponsorships/by-account/:account`

**Path Parameters**:

- `account`: Stellar account address

**Example Request**:

```bash
GET /api/sponsorships/by-account/GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S
```

**Response**:

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "sponsor": "GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S",
    "sponsored_account": "GCKFBEOZXN3IXMF64APXMC7FXZUQZU6ZGWKD2XXWQVM3CWDUSXRKQFB",
    "sponsored_reserves": 2,
    "total_amount": "100.00",
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
]
```

---

### 7. Get Analytics Summary

**Endpoint**: `GET /api/analytics/summary`

**Example Request**:

```bash
GET /api/analytics/summary
```

**Response**:

```json
{
  "total_sponsorships": 150,
  "total_amount_sponsored": "15000.00",
  "unique_sponsors": 45,
  "unique_sponsored_accounts": 120,
  "average_sponsorship": "100.00",
  "largest_sponsorship": "500.00",
  "smallest_sponsorship": "10.00"
}
```

---

### 8. Health Check

**Endpoint**: `GET /health`

**Example Request**:

```bash
GET /health
```

**Response**:

```
OK
```

---

## Error Codes

| Code | Message               | Description                   |
| ---- | --------------------- | ----------------------------- |
| 200  | OK                    | Request successful            |
| 201  | Created               | Resource created successfully |
| 400  | Bad Request           | Invalid request parameters    |
| 404  | Not Found             | Resource not found            |
| 500  | Internal Server Error | Server-side error             |

---

## Rate Limiting

Currently, there are no rate limits implemented. For production deployments, consider implementing rate limiting to prevent abuse.

---

## Authentication

Currently, the API does not require authentication. For production use, consider implementing API key authentication or OAuth2.

---

## Pagination

Results are paginated using `limit` and `offset` parameters:

- `limit`: Maximum number of results (max 1000)
- `offset`: Number of results to skip

Example:

```bash
GET /api/sponsorships?limit=50&offset=0
```

---

## Filtering and Sorting

FilteringOptimization options:

- By account: `/api/sponsorships/by-account/:account`
- By sponsor: Use client-side filtering on list results
- Default sorting: By `created_at` descending

---

## Example Usage with cURL

```bash
# Create a sponsorship
curl -X POST http://localhost:3000/api/sponsorships \
  -H "Content-Type: application/json" \
  -d '{
    "sponsor": "GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S",
    "sponsored_account": "GCKFBEOZXN3IXMF64APXMC7FXZUQZU6ZGWKD2XXWQVM3CWDUSXRKQFB",
    "sponsored_reserves": 2,
    "total_amount": "100.00"
  }'

# Get all sponsorships
curl http://localhost:3000/api/sponsorships?limit=10

# Get sponsor leaderboard
curl http://localhost:3000/api/sponsors/leaderboard?limit=50

# Get analytics
curl http://localhost:3000/api/analytics/summary
```

---

## Example Usage with JavaScript/Fetch

```javascript
// Create a sponsorship
fetch("http://localhost:3000/api/sponsorships", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    sponsor: "GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S",
    sponsored_account:
      "GCKFBEOZXN3IXMF64APXMC7FXZUQZU6ZGWKD2XXWQVM3CWDUSXRKQFB",
    sponsored_reserves: 2,
    total_amount: "100.00",
  }),
})
  .then((res) => res.json())
  .then((data) => console.log(data));

// Get all sponsorships
fetch("http://localhost:3000/api/sponsorships?limit=10")
  .then((res) => res.json())
  .then((data) => console.log(data));
```
