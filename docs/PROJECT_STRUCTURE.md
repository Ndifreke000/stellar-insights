# Stellar Insights - Project Structure

This document outlines the organized structure of the Stellar Insights monorepo.

## 📁 Root Structure

```
stellar-insights/
├── 📱 mobile/           # React Native mobile app
├── 🌐 frontend/         # Next.js web application  
├── ⚙️  backend/          # Rust API server
├── 📜 contracts/        # Soroban smart contracts
├── 📦 sdk/              # TypeScript SDK
├── 📚 docs/             # All documentation
├── 🔧 scripts/          # Build and deployment scripts
├── ☁️  k8s/              # Kubernetes configs
├── 🐳 terraform/        # Infrastructure as code
├── 📊 elk/              # ELK stack configs
└── 📋 README.md         # Main project documentation
```

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
- SQLx database integration
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
TypeScript SDK providing:
- API client
- Type definitions
- React Native compatibility
- Authentication helpers

### 📚 Documentation (`docs/`)
All project documentation organized by:
- Architecture decisions
- Development guides
- Deployment instructions
- Issue tracking
- API references

## 🚀 Getting Started

1. **Prerequisites**: Node.js, Rust, Docker
2. **Setup**: Run setup scripts in each component
3. **Development**: Use component-specific dev commands
4. **Deployment**: Follow deployment guides in docs/

## 🔗 Quick Links

- [Mobile Setup](mobile/README.md)
- [Backend Setup](backend/README.md)  
- [Frontend Setup](frontend/README.md)
- [Full Documentation](docs/README.md)
- [Contributing Guide](CONTRIBUTING.md)