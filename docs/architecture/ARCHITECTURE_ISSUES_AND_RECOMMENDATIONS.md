# Architecture Issues & Recommendations for Mobile + Multi-Network Support

## Executive Summary

Your PayRaider project needs significant architectural changes to support:
1. **Mobile applications** (iOS/Android)
2. **Runtime network switching** (testnet/mainnet)
3. **Multi-platform consistency**

## Critical Architectural Issues

### 🔴 Issue 1: Tight Coupling to Next.js SSR

**Problem:**
- Frontend is built as a Next.js app with server-side rendering
- Many components rely on Next.js-specific APIs (`getServerSideProps`, `getStaticProps`)
- Cannot be easily ported to mobile

**Impact:**
- Cannot reuse frontend code for mobile
- Must rebuild entire UI for mobile from scratch
- Inconsistent user experience across platforms

**Solution:**
```
1. Extract business logic into shared SDK
2. Make components platform-agnostic
3. Use API routes only for backend proxy, not business logic
4. Consider Next.js as just one client of the backend API
```

### 🔴 Issue 2: Hardcoded Network Configuration

**Problem:**
```rust
// backend/.env.example
STELLAR_NETWORK=mainnet  // ❌ Environment variable, not runtime
STELLAR_RPC_URL_MAINNET=https://stellar.api.onfinality.io/public
STELLAR_RPC_URL_TESTNET=https://soroban-testnet.stellar.org
```

**Impact:**
- Users cannot switch networks without restarting the app
- Requires separate deployments for testnet/mainnet
- Testing is cumbersome
- Cannot support multi-network wallets

**Solution:**
```rust
// Add network context to every request
pub struct NetworkContext {
    pub network: StellarNetwork,  // Testnet | Mainnet
    pub rpc_url: String,
    pub horizon_url: String,
}

// Accept via HTTP header
X-Stellar-Network: testnet
```

### 🔴 Issue 3: No Mobile Strategy

**Problem:**
- No mobile app exists
- No mobile-specific API endpoints
- No offline support
- No push notifications
- Web app not optimized for mobile browsers

**Impact:**
- Missing entire mobile user segment
- Poor mobile web experience
- Cannot compete with mobile-first competitors

**Solution:**
```
1. Build React Native app (recommended) or Flutter
2. Create mobile-optimized API endpoints
3. Implement offline-first architecture
4. Add push notification support
```

### 🔴 Issue 4: No Shared SDK

**Problem:**
- API client logic is embedded in Next.js frontend
- Each new client (mobile, CLI, etc.) must reimplement API calls
- No type safety across platforms
- Inconsistent error handling

**Impact:**
- Code duplication
- Inconsistent behavior across clients
- Difficult to maintain
- Higher bug rate

**Solution:**
```typescript
// Create @payraider/sdk package
import { PayRaiderClient } from '@payraider/sdk';

const client = new PayRaiderClient({
  apiUrl: 'https://api.payraider.com',
  network: 'testnet',
  auth: { token: 'xxx' }
});

const corridors = await client.corridors.list();
```

### 🔴 Issue 5: Single Database for All Networks

**Problem:**
```rust
// backend/.env.example
DATABASE_URL=sqlite:./payraider.db  // ❌ Single database
```

**Impact:**
- Testnet and mainnet data mixed together
- Risk of data contamination
- Cannot test safely
- Difficult to separate metrics

**Solution:**
```rust
// Separate databases per network
DATABASE_URL_TESTNET=sqlite:./payraider_testnet.db
DATABASE_URL_MAINNET=sqlite:./payraider_mainnet.db

// Or use schema prefixing
testnet_corridors
mainnet_corridors
```

### 🟡 Issue 6: Authentication Not Mobile-Friendly

**Problem:**
- SEP-10 authentication exists but designed for web
- No biometric authentication support
- No secure token storage for mobile
- Cookie-based sessions don't work well on mobile

**Impact:**
- Poor mobile UX
- Security concerns on mobile
- Cannot use platform features (Face ID, Touch ID)

**Solution:**
```
1. Implement JWT token-based auth for mobile
2. Add biometric authentication
3. Use platform keychain/keystore for token storage
4. Support refresh token rotation
```

### 🟡 Issue 7: No Offline Support

**Problem:**
- All data fetched from API in real-time
- No local caching strategy
- No offline queue for mutations
- Poor experience on slow/unreliable networks

**Impact:**
- App unusable without internet
- Poor mobile experience
- High data usage
- Slow perceived performance

**Solution:**
```
1. Implement offline-first architecture
2. Use SQLite/Realm for local storage
3. Queue mutations when offline
4. Sync when connection restored
5. Show staleness indicators
```

### 🟡 Issue 8: State Management Not Portable

**Problem:**
```typescript
// frontend uses Zustand (web-only)
import { create } from 'zustand';
```

**Impact:**
- Cannot share state management with mobile
- Must reimplement for mobile
- Inconsistent behavior

**Solution:**
```
1. Use platform-agnostic state management (Redux Toolkit, Jotai)
2. Or keep state in SDK layer
3. Use React Query for server state (works on mobile)
```

## Recommended Architecture

### Target Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Client Layer                             │
├──────────────────────┬──────────────────────┬───────────────┤
│   Web (Next.js)      │  Mobile (React Native)│  CLI/Desktop  │
│   - SSR/SSG          │  - Native UI          │               │
│   - Browser APIs     │  - Offline-first      │               │
│   - PWA              │  - Push notifications │               │
└──────────┬───────────┴──────────┬────────────┴───────────────┘
           │                      │
           └──────────┬───────────┘
                      │
           ┌──────────▼──────────────────────┐
           │   @payraider/sdk         │
           │   - TypeScript                  │
           │   - Platform-agnostic           │
           │   - Type-safe API client        │
           │   - Network context management  │
           │   - Retry logic                 │
           │   - Request deduplication       │
           └──────────┬──────────────────────┘
                      │
           ┌──────────▼──────────────────────┐
           │   API Gateway (Optional)        │
           │   - Rate limiting               │
           │   - Request routing             │
           │   - Load balancing              │
           └──────────┬──────────────────────┘
                      │
           ┌──────────▼──────────────────────┐
           │   Backend (Rust)                │
           │   - Network context injection   │
           │   - Business logic              │
           │   - Data aggregation            │
           └──────────┬──────────────────────┘
                      │
        ┌─────────────┴─────────────┐
        │                           │
┌───────▼────────┐         ┌───────▼────────┐
│ Testnet Stack  │         │ Mainnet Stack  │
├────────────────┤         ├────────────────┤
│ - SQLite DB    │         │ - SQLite DB    │
│ - Redis Cache  │         │ - Redis Cache  │
│ - RPC Client   │         │ - RPC Client   │
└────────┬───────┘         └────────┬───────┘
         │                          │
┌────────▼───────┐         ┌────────▼───────┐
│ Stellar Testnet│         │ Stellar Mainnet│
└────────────────┘         └────────────────┘
```

### Key Changes Required

#### 1. Backend Changes

**File: `backend/src/network_context.rs` (NEW)**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StellarNetwork {
    Testnet,
    Mainnet,
}

pub struct NetworkContext {
    pub network: StellarNetwork,
    pub rpc_url: String,
    pub horizon_url: String,
    pub db_pool: SqlitePool,
}

// Middleware to extract network from header
pub async fn network_context_middleware(
    req: Request,
    next: Next,
) -> Result<Response, NetworkError> {
    let network = req
        .headers()
        .get("X-Stellar-Network")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| match s {
            "testnet" => Some(StellarNetwork::Testnet),
            "mainnet" => Some(StellarNetwork::Mainnet),
            _ => None,
        })
        .ok_or(NetworkError::InvalidNetwork)?;
    
    let context = NetworkContext::from_network(network)?;
    req.extensions_mut().insert(context);
    
    Ok(next.run(req).await)
}
```

**File: `backend/src/database.rs` (MODIFY)**
```rust
pub struct DatabaseManager {
    testnet_pool: SqlitePool,
    mainnet_pool: SqlitePool,
}

impl DatabaseManager {
    pub fn get_pool(&self, network: StellarNetwork) -> &SqlitePool {
        match network {
            StellarNetwork::Testnet => &self.testnet_pool,
            StellarNetwork::Mainnet => &self.mainnet_pool,
        }
    }
}
```

#### 2. Create Shared SDK

**File: `sdk/typescript/src/client.ts` (NEW)**
```typescript
export class PayRaiderClient {
  private apiUrl: string;
  private network: 'testnet' | 'mainnet';
  private authToken?: string;

  constructor(config: ClientConfig) {
    this.apiUrl = config.apiUrl;
    this.network = config.network;
    this.authToken = config.authToken;
  }

  private async request<T>(
    endpoint: string,
    options?: RequestOptions
  ): Promise<T> {
    const response = await fetch(`${this.apiUrl}${endpoint}`, {
      ...options,
      headers: {
        'X-Stellar-Network': this.network,
        'Authorization': this.authToken ? `Bearer ${this.authToken}` : '',
        'Content-Type': 'application/json',
        ...options?.headers,
      },
    });

    if (!response.ok) {
      throw new ApiError(response);
    }

    return response.json();
  }

  // API methods
  corridors = {
    list: () => this.request<Corridor[]>('/api/v1/corridors'),
    get: (id: string) => this.request<Corridor>(`/api/v1/corridors/${id}`),
  };

  anchors = {
    list: () => this.request<Anchor[]>('/api/v1/anchors'),
    get: (id: string) => this.request<Anchor>(`/api/v1/anchors/${id}`),
  };

  // Network switching
  switchNetwork(network: 'testnet' | 'mainnet') {
    this.network = network;
    // Clear any cached data
    this.clearCache();
  }
}
```

#### 3. Mobile App Structure

**File: `mobile/src/App.tsx` (NEW)**
```typescript
import React from 'react';
import { NavigationContainer } from '@react-navigation/native';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { PayRaiderClient } from '@payraider/sdk';

const client = new PayRaiderClient({
  apiUrl: 'https://api.payraider.com',
  network: 'testnet',
});

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // Offline-first configuration
      cacheTime: 1000 * 60 * 60 * 24, // 24 hours
      staleTime: 1000 * 60 * 5, // 5 minutes
      retry: 3,
    },
  },
});

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <NavigationContainer>
        {/* Your app navigation */}
      </NavigationContainer>
    </QueryClientProvider>
  );
}
```

## Migration Roadmap

### Phase 1: Backend Refactoring (2-3 weeks)
- [ ] Implement network context middleware
- [ ] Separate databases for testnet/mainnet
- [ ] Add network-aware RPC routing
- [ ] Create mobile-optimized endpoints (pagination, field selection)
- [ ] Add push notification infrastructure

### Phase 2: Shared SDK (2 weeks)
- [ ] Extract API client from frontend
- [ ] Create TypeScript SDK package
- [ ] Add React Native compatibility
- [ ] Publish to npm
- [ ] Update web frontend to use SDK

### Phase 3: Mobile App MVP (4-6 weeks)
- [ ] Set up React Native project
- [ ] Implement authentication (JWT + biometric)
- [ ] Build core screens (dashboard, corridors, anchors)
- [ ] Add offline support (SQLite + sync queue)
- [ ] Implement push notifications
- [ ] Add network switcher UI

### Phase 4: Testing & Launch (2 weeks)
- [ ] End-to-end testing
- [ ] Performance optimization
- [ ] Security audit
- [ ] Beta testing (TestFlight, Google Play Beta)
- [ ] Production launch

## Estimated Effort

- **Backend refactoring**: 80-120 hours
- **Shared SDK**: 40-60 hours
- **Mobile app MVP**: 160-240 hours
- **Testing & refinement**: 40-60 hours

**Total**: 320-480 hours (8-12 weeks with 1 developer)

## Priority Recommendations

### Must Have (P0)
1. ✅ Network context management in backend
2. ✅ Separate databases for testnet/mainnet
3. ✅ Shared SDK creation
4. ✅ Mobile app foundation (React Native)

### Should Have (P1)
5. Offline support
6. Push notifications
7. Biometric authentication
8. Mobile-optimized endpoints

### Nice to Have (P2)
9. GraphQL API
10. Real-time sync via WebSockets
11. Advanced caching strategies
12. Desktop app (Electron)

## Technology Recommendations

### Mobile Framework: React Native ✅
**Why:**
- Share code with web (React)
- Large ecosystem
- Good performance
- Team already knows React

**Alternatives:**
- Flutter (better performance, different language)
- Native (best performance, 2x development time)

### State Management: React Query + Zustand
**Why:**
- React Query handles server state (works on mobile)
- Zustand for local UI state (simple, works on mobile)
- Both are platform-agnostic

### Offline Storage: WatermelonDB
**Why:**
- Built for React Native
- Excellent performance
- Sync support built-in
- Observable queries

### Push Notifications: Firebase Cloud Messaging
**Why:**
- Works on both iOS and Android
- Free tier sufficient for most apps
- Good documentation

## Security Considerations

1. **Token Storage**: Use React Native Keychain for secure token storage
2. **Certificate Pinning**: Implement SSL pinning to prevent MITM attacks
3. **Biometric Auth**: Use react-native-biometrics for Face ID/Touch ID
4. **Network Validation**: Always validate network context on backend
5. **Rate Limiting**: Implement per-device rate limiting

## Next Steps

1. **Review this document** with your team
2. **Create detailed spec** in `.kiro/specs/mobile-and-multi-network-architecture/`
3. **Start with Phase 1** (backend refactoring)
4. **Set up CI/CD** for mobile builds
5. **Create project roadmap** with milestones

---

**Questions? Issues?**
- Create GitHub issues for specific problems
- Reference this document in architectural discussions
- Update this document as architecture evolves
