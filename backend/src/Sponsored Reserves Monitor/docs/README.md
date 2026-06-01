# Stellar Sponsored Reserves Monitor

A comprehensive full-stack application for monitoring and analyzing sponsored reserves on the Stellar blockchain.

## Overview

This application tracks which accounts are sponsoring reserves for others, calculates total sponsored amounts, visualizes sponsorship relationships, and provides analytics on network subsidization patterns.

## Features

### Core Features

- ✅ **Sponsorship Tracking**: Monitor all sponsored reserve operations
- ✅ **Relationship Mapping**: View sponsorship relationships between accounts
- ✅ **Amount Calculations**: Calculate and display total sponsored amounts
- ✅ **Sponsor Leaderboard**: Rank top sponsors by total sponsored amount
- ✅ **Change Alerts**: Get notified of sponsorship changes
- ✅ **Analytics Dashboard**: Comprehensive sponsorship statistics
- ✅ **History Tracking**: View historical changes to sponsorships
- ✅ **Advanced Pagination**: Handle large datasets efficiently

## Project Structure

```
Sponsored Reserves Monitor/
├── backend/                           # Rust backend service
│   ├── src/
│   │   ├── main.rs                   # Application entry point
│   │   ├── models/                   # Data models
│   │   │   └── mod.rs
│   │   ├── services/                 # Business logic
│   │   │   ├── mod.rs
│   │   │   └── sponsorship_tracker.rs # Core sponsorship service
│   │   ├── api/                      # API endpoints
│   │   │   ├── mod.rs
│   │   │   └── sponsorships.rs       # Sponsorship endpoints
│   │   └── db/                       # Database initialization
│   │       └── mod.rs
│   ├── tests/                        # Integration tests
│   │   └── sponsorship_tests.rs
│   └── Cargo.toml
│
├── frontend/                          # Next.js frontend
│   ├── src/
│   │   ├── app/
│   │   │   └── sponsorships/
│   │   │       └── page.tsx          # Main dashboard page
│   │   └── components/
│   │       └── sponsorships/
│   │           ├── list.tsx          # Sponsorship list table
│   │           ├── leaderboard.tsx   # Top sponsors ranking
│   │           └── analytics.tsx     # Analytics cards
│   └── package.json
│
└── docs/                              # Documentation
    ├── README.md
    ├── API.md
    └── SETUP.md
```

## Technology Stack

### Backend

- **Language**: Rust
- **Framework**: Axum (async web framework)
- **Database**: SQLite with SQLx for type-safe queries
- **API**: RESTful JSON endpoints

### Frontend

- **Framework**: Next.js 14+ (React Server Components)
- **Styling**: Tailwind CSS
- **HTTP Client**: Axios
- **Charting**: Recharts
- **TypeScript**: Type-safe React components

## API Endpoints

### Sponsorships

- `GET /api/sponsorships` - List all sponsorships (paginated)
- `POST /api/sponsorships` - Create a new sponsorship record
- `GET /api/sponsorships/:id` - Get sponsorship details
- `GET /api/sponsorships/:id/history` - Get sponsorship change history

### Analytics & Rankings

- `GET /api/sponsors/leaderboard` - Get top sponsors ranking
- `GET /api/sponsorships/by-account/:account` - Get sponsorships for an account
- `GET /api/analytics/summary` - Get overall analytics

## Data Models

### Sponsorship

```rust
{
    id: String,                    // Unique identifier
    sponsor: String,               // Sponsor account address
    sponsored_account: String,     // Sponsored account address
    sponsored_reserves: i64,       // Number of reserved entries sponsored
    total_amount: String,          // Total XLM amount
    created_at: String,            // Creation timestamp
    updated_at: String             // Last update timestamp
}
```

### SponsorshipHistory

```rust
{
    id: String,
    sponsorship_id: String,
    change_type: String,           // CREATE, UPDATE, DELETE
    previous_amount: Option<String>,
    new_amount: String,
    created_at: String
}
```

### SponsorLeaderboard

```rust
{
    sponsor: String,               // Sponsor address
    total_sponsored_amount: String, // Sum of all sponsored amounts
    sponsored_accounts_count: i64,  // Number of accounts sponsored
    rank: i64                       // Ranking position
}
```

## Database Schema

### sponsorships table

```sql
CREATE TABLE sponsorships (
    id TEXT PRIMARY KEY,
    sponsor TEXT NOT NULL,
    sponsored_account TEXT NOT NULL,
    sponsored_reserves INTEGER NOT NULL,
    total_amount TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(sponsor, sponsored_account)
)
```

### sponsorship_history table

```sql
CREATE TABLE sponsorship_history (
    id TEXT PRIMARY KEY,
    sponsorship_id TEXT NOT NULL,
    change_type TEXT NOT NULL,
    previous_amount TEXT,
    new_amount TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(sponsorship_id) REFERENCES sponsorships(id)
)
```

### sponsorship_alerts table

```sql
CREATE TABLE sponsorship_alerts (
    id TEXT PRIMARY KEY,
    sponsorship_id TEXT NOT NULL,
    change_type TEXT NOT NULL,
    previous_value TEXT,
    new_value TEXT NOT NULL,
    created_at TEXT NOT NULL,
    acknowledged BOOLEAN DEFAULT FALSE,
    FOREIGN KEY(sponsorship_id) REFERENCES sponsorships(id)
)
```

## Setup Instructions

### Prerequisites

- Rust 1.70+ (for backend)
- Node.js 18+ (for frontend)
- SQLite3

### Backend Setup

1. Navigate to backend directory:

```bash
cd backend
```

2. Set up environment variables:

```bash
echo "DATABASE_URL=sqlite://sponsorships.db" > .env
```

3. Run tests:

```bash
cargo test
```

4. Start the server:

```bash
cargo run
```

The API will be available at `http://localhost:3000`

### Frontend Setup

1. Navigate to frontend directory:

```bash
cd frontend
```

2. Install dependencies:

```bash
npm install
```

3. Start the development server:

```bash
npm run dev
```

The frontend will be available at `http://localhost:3000` (usually 3001 if backend uses 3000)

## Usage Examples

### Track a New Sponsorship

```bash
curl -X POST http://localhost:3000/api/sponsorships \
  -H "Content-Type: application/json" \
  -d '{
    "sponsor": "GBUQWP3BOUZX34ULNQG23RQ6F4BFXWXZMVEON46YSVP33ZDTF75SZK2S",
    "sponsored_account": "GCKFBEOZXN3IXMF64APXMC7FXZUQZU6ZGWKD2XXWQVM3CWDUSXRKQFB",
    "sponsored_reserves": 2,
    "total_amount": "100.00"
  }'
```

### Get Sponsor Leaderboard

```bash
curl http://localhost:3000/api/sponsors/leaderboard?limit=50
```

### Get Analytics Summary

```bash
curl http://localhost:3000/api/analytics/summary
```

## Testing

### Run Backend Tests

```bash
cd backend
cargo test
```

Tests cover:

- Sponsorship tracking and creation
- Sponsorship updates
- History management
- Analytics calculations
- Leaderboard generation

## Creating a Production Build

### Backend

```bash
cargo build --release
# Binary will be in target/release/stellar-reserves-monitor
```

### Frontend

```bash
cd frontend
npm run build
npm start
```

## Acceptance Criteria Fulfilled

✅ **Track sponsored reserve operations** - Service tracks all sponsorship operations
✅ **Store sponsorship relationships** - Database maintains sponsor-to-account relationships
✅ **Calculate total sponsored amounts** - Analytics service calculates totals
✅ **Create sponsorship dashboard** - React frontend with Next.js dashboard
✅ **Display sponsor rankings** - Leaderboard shows top sponsors ranked by amount
✅ **Show sponsorship history** - History tracking and retrieval implemented
✅ **Add tests and documentation** - Unit/integration tests and complete documentation

## Performance Optimization

- Database indices on sponsor and sponsored_account fields
- Pagination support for large datasets
- Efficient SQL queries with proper joins
- TypeScript type safety to prevent runtime errors
- Server-side rendering with Next.js for better performance

## Future Enhancements

- Real-time WebSocket updates for sponsorship changes
- Export functionality (CSV, JSON)
- Advanced filtering and search capabilities
- Integration with Stellar Horizon API for live ledger data
- Webhook notifications for sponsorship changes
- Multi-network support (testnet, public network)
- User authentication and account-based dashboards
- GraphQL API alongside REST endpoints

## Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

For issues, questions, or suggestions, please open an issue on the GitHub repository.

## Stellar Resources

- [Stellar Documentation](https://developers.stellar.org/)
- [Stellar Horizon API](https://developers.stellar.org/api/)
- [Sponsored Reserves Documentation](https://developers.stellar.org/learn/fundamentals/sponsorship)
