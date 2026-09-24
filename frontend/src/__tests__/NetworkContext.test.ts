import { describe, it, expect } from 'vitest';
import {
  stellarExpertAccountUrl,
  stellarExpertContractUrl,
} from '@/contexts/NetworkContext';
import type { NetworkInfo } from '@/lib/api/types';

const MAINNET: NetworkInfo = {
  network: 'mainnet',
  display_name: 'Mainnet',
  rpc_url: 'https://stellar.api.onfinality.io/public',
  horizon_url: 'https://horizon.stellar.org',
  network_passphrase: 'Public Global Stellar Network ; September 2015',
  color: '#2563EB',
  is_mainnet: true,
  is_testnet: false,
};

const TESTNET: NetworkInfo = {
  network: 'testnet',
  display_name: 'Testnet',
  rpc_url: 'https://soroban-testnet.stellar.org',
  horizon_url: 'https://horizon-testnet.stellar.org',
  network_passphrase: 'Test SDF Network ; September 2015',
  color: '#4ECDC4',
  is_mainnet: false,
  is_testnet: true,
};

describe('stellarExpert URLs', () => {
  it('uses public explorer segment on mainnet', () => {
    expect(stellarExpertAccountUrl(MAINNET, 'GABC')).toBe(
      'https://stellar.expert/explorer/public/account/GABC',
    );
    expect(stellarExpertContractUrl(MAINNET, 'CABC')).toBe(
      'https://stellar.expert/explorer/public/contract/CABC',
    );
  });

  it('uses testnet explorer segment on testnet', () => {
    expect(stellarExpertAccountUrl(TESTNET, 'GABC')).toBe(
      'https://stellar.expert/explorer/testnet/account/GABC',
    );
    expect(stellarExpertContractUrl(TESTNET, 'CABC')).toBe(
      'https://stellar.expert/explorer/testnet/contract/CABC',
    );
  });

  it('defaults to public when network is unknown', () => {
    expect(stellarExpertAccountUrl(null, 'GABC')).toContain('/public/');
  });
});
