# Testnet Quickstart Guide

This guide describes how to connect the entire Stellar Insights stack (backend, frontend, mobile, contracts, and SDK examples) to the Stellar Testnet.

## Fund a testnet account with Friendbot

Before deploying contracts or making transactions, you need a funded Stellar account on the testnet.

1. **Generate a new keypair**:
   Using the Stellar CLI:
   ```bash
   stellar keys generate --network testnet my-identity
   ```
   Or using standard tools to obtain a Public Key (starting with `G`) and a Secret Key (starting with `S`).

2. **Fund the account with Friendbot**:
   You can fund your new account via the Stellar CLI:
   ```bash
   stellar keys fund my-identity --network testnet
   ```
   Alternatively, trigger Friendbot using `curl`:
   ```bash
   curl "https://friendbot.stellar.org/?addr=YOUR_STELLAR_PUBLIC_KEY"
   ```

---

## Configure backend .env for testnet

To configure the backend to read from the testnet:

1. Copy `backend/.env.example` to `backend/.env`.
2. Update the network environment variables:
   ```ini
   # Set network to testnet
   STELLAR_NETWORK=testnet
   STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
   
   # SEP-10 server key (must start with G and be 56 characters)
   SEP10_SERVER_PUBLIC_KEY=YOUR_TESTNET_PUBLIC_KEY
   SEP10_HOME_DOMAIN=localhost:8080
   ```
3. Verify that the testnet RPC/Horizon endpoints are set correctly:
   ```ini
   STELLAR_RPC_URL_TESTNET=https://soroban-testnet.stellar.org
   STELLAR_HORIZON_URL_TESTNET=https://horizon-testnet.stellar.org
   ```

---

## Deploy contracts to testnet

To deploy the Soroban contracts to the testnet:

1. **Build the contracts**:
   Navigate to the `contracts/` directory and compile the WASM targets:
   ```bash
   cd contracts
   cargo build --target wasm32v1-none --release
   ```

2. **Deploy via Stellar CLI**:
   Deploy the `stellar_insights` analytics contract to the testnet:
   ```bash
   stellar contract deploy \
     --wasm target/wasm32v1-none/release/stellar_insights.wasm \
     --source my-identity \
     --network testnet
   ```
   Note the returned Contract ID (64-character hex string) for use in backend configuration.

---

## Run frontend against testnet API

To point the Next.js frontend to the testnet backend API:

1. Create a `frontend/.env.local` file.
2. Set the backend API URL:
   ```ini
   NEXT_PUBLIC_API_URL=http://localhost:8080/api/v1
   NEXT_PUBLIC_WS_URL=ws://localhost:8080/ws
   ```
3. Run the development server:
   ```bash
   cd frontend
   npm run dev
   ```

---

## Run mobile app against testnet

To run the React Native mobile app pointing to the testnet:

1. Copy `mobile/.env.example` to `mobile/.env`.
2. Configure the backend API URL and the Stellar network settings:
   ```ini
   # API Configuration pointing to local backend (or deployed staging backend)
   # For Android emulator, use 10.0.2.2 instead of localhost
   API_BASE_URL=http://localhost:8080/api/v1
   
   # Stellar Network Configuration
   STELLAR_NETWORK=testnet
   STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org
   ```
3. Run the mobile application:
   ```bash
   cd mobile
   npm run android  # or npm run ios
   ```

---

## Run SDK examples against testnet

To run SDK examples pointing to the testnet:

1. Verify that your SDK configuration imports the testnet passphrase and endpoint.
2. Example script usage:
   ```javascript
   import { StellarSdk } from '@stellar-insights/sdk';
   
   const sdk = new StellarSdk({
     network: 'testnet',
     rpcUrl: 'https://soroban-testnet.stellar.org',
     horizonUrl: 'https://horizon-testnet.stellar.org'
   });
   
   // Run your integration script
   await sdk.fetchCorridors();
   ```
