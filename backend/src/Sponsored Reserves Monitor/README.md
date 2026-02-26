# Stellar Sponsored Reserves Monitor

A full-stack application for monitoring and analyzing sponsored reserves on the Stellar blockchain network.

## Quick Start

### Prerequisites

- Rust 1.70+
- Node.js 18+
- SQLite3

### Installation

See [SETUP.md](docs/SETUP.md) for detailed setup instructions.

### Running the Application

**Backend (Terminal 1)**:

```bash
cd backend
cargo run
```

**Frontend (Terminal 2)**:

```bash
cd frontend
npm run dev
```

Access the dashboard at `http://localhost:3001`

## Features

- 📊 Real-time sponsorship tracking
- 👥 Sponsor leaderboard with rankings
- 📈 Comprehensive analytics dashboard
- 🔄 Change history tracking
- ⚡ Fast SQLite database with indices
- 🎨 Modern React UI with Tailwind CSS
- 🧪 Full test coverage

## Documentation

- [README.md](docs/README.md) - Complete project documentation
- [API.md](docs/API.md) - API endpoint reference
- [SETUP.md](docs/SETUP.md) - Setup instructions

## Project Structure

```
├── backend/        # Rust backend service
├── frontend/       # Next.js React frontend
├── docs/          # Documentation
└── .gitignore
```

## API Quick Reference

- `GET /api/sponsorships` - List all sponsorships
- `POST /api/sponsorships` - Create sponsorship
- `GET /api/sponsors/leaderboard` - Top sponsors
- `GET /api/analytics/summary` - Analytics data
- `GET /health` - Health check

See [API.md](docs/API.md) for full documentation.

## Development

### Backend Tests

```bash
cd backend
cargo test
```

### Frontend Build

```bash
cd frontend
npm run build
npm start
```

## License

MIT License - See LICENSE file for details

## Support

For issues and questions, open an issue on the repository.
