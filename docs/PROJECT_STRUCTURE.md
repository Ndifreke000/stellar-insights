# Stellar Insights - Project Structure

This document outlines the organized structure of the Stellar Insights monorepo.

## 📁 Root Structure

```
stellar-insights/
├── 📱 mobile/           # React Native mobile app
├── 🌐 frontend/         # Next.js web application
├── ⚙️  backend/          # Rust API server
├── 📜 contracts/        # Soroban smart contracts
├── 📦 sdk/              # TypeScript + Python client SDKs
├── 📚 docs/             # All documentation
├── 🔧 scripts/          # Build, deployment, and maintenance scripts
├── ☁️  k8s/              # Kubernetes configs
├── 🐳 terraform/        # Infrastructure as code
├── 📊 elk/              # ELK stack configs (centralized logging)
├── 📈 monitoring/       # Prometheus alert rules, Alertmanager config
├── 🧪 tests/            # Repo-root acceptance, chaos, and e2e tests (Playwright)
├── 📋 README.md         # Main project documentation
└── 📋 package.json      # Repo-root tooling only (Playwright + commitlint)
```

Note: the repo root used to also have a second, disconnected Vite/React app
(`src/`, `index.html`, `vite.config.ts`, package name `"awpwrate"`) from a
single auto-generated commit, unrelated to the real Next.js app in
`frontend/`. It has been removed; the root `package.json` above is
repo-root tooling only, not an application.

## 🏗️ Component Details

### 📱 Mobile App (`mobile/`)
React Native application with:
- TypeScript configuration
- Navigation (Auth + Main tabs)
- State management (Zustand)
- Offline-first architecture
- iOS/Android support

### 🌐 Frontend (`frontend/`)
Next.js web application with:
- Server-side rendering
- TypeScript support
- Responsive design
- PWA capabilities
- Chart visualizations

### ⚙️ Backend (`backend/`)
Rust API server with:
- Axum web framework
- SQLx database integration (PostgreSQL)
- Job scheduling
- Observability/metrics
- Multi-network support

### 📜 Contracts (`contracts/`)
Soroban smart contracts for:
- Asset verification
- Transaction processing
- Governance mechanisms
- Security audits

### 📦 SDK (`sdk/`)
Client SDKs providing:
- API client (TypeScript, Python)
- Type definitions
- React Native compatibility
- Authentication helpers

### 📚 Documentation (`docs/`)
Project documentation organized by:
- Architecture decisions
- Development guides (e.g. [testnet quickstart](testnet-quickstart.md))
- Deployment instructions and runbooks
- Issue tracking
- API references

Note: `docs/` previously contained ~475 vendored third-party markdown/JSON
files (Next.js, undici, Recharts, and other unrelated projects' own docs,
accidentally committed verbatim). Those have been removed; everything
remaining here is project-specific.

## 🚀 Getting Started

1. **Prerequisites**: Node.js, Rust, Docker
2. **Setup**: Run setup scripts in each component
3. **Development**: Use component-specific dev commands
4. **Deployment**: Follow deployment guides in docs/

## 🔗 Quick Links

- [Mobile status & setup](../mobile/README.md#current-status)
- [Testnet quickstart](testnet-quickstart.md)
- [Full documentation index](README.md)
- [Root README](../README.md)

Note: this document previously linked to `backend/README.md`,
`frontend/README.md`, and a root-level `CONTRIBUTING.md` — none of those
files exist in the repo. Removed rather than left dangling; if/when those
are written, re-add the links here.
