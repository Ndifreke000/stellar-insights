/**
 * @jest-environment node
 */

import { GET } from '../src/app/api/dashboard/route';
import { 
  validateCorridorHealth, 
  validateLiquidityDataPoint, 
  validateAssetData, 
  validateSettlementSpeedDataPoint 
} from '../src/types/dashboard';

describe('/api/dashboard', () => {
  it('should return valid dashboard data with proper types', async () => {
    const response = await GET();
    const data = await response.json();

    // Check response structure
    expect(data).toHaveProperty('kpi');
    expect(data).toHaveProperty('corridors');
    expect(data).toHaveProperty('liquidity');
    expect(data).toHaveProperty('assets');
    expect(data).toHaveProperty('settlement');

    // Validate arrays are properly typed
    expect(Array.isArray(data.corridors)).toBe(true);
    expect(Array.isArray(data.liquidity)).toBe(true);
    expect(Array.isArray(data.assets)).toBe(true);
    expect(Array.isArray(data.settlement)).toBe(true);

    // Validate corridor data
    data.corridors.forEach((corridor: any) => {
      expect(validateCorridorHealth(corridor)).toBe(true);
    });

    // Validate liquidity data
    data.liquidity.forEach((point: any) => {
      expect(validateLiquidityDataPoint(point)).toBe(true);
    });

    // Validate asset data
    data.assets.forEach((asset: any) => {
      expect(validateAssetData(asset)).toBe(true);
    });

    // Validate settlement data
    data.settlement.forEach((point: any) => {
      expect(validateSettlementSpeedDataPoint(point)).toBe(true);
    });
  });

  it('should validate individual data types correctly', () => {
    // Test corridor validation
    expect(validateCorridorHealth({
      id: '1',
      name: 'Test',
      status: 'optimal',
      uptime: 99.9,
      volume24h: 1000000
    })).toBe(true);

    expect(validateCorridorHealth({
      id: '1',
      name: 'Test',
      status: 'invalid-status', // Invalid status
      uptime: 99.9,
      volume24h: 1000000
    })).toBe(false);

    // Test asset validation
    expect(validateAssetData({
      symbol: 'USDC',
      name: 'USD Coin',
      volume24h: 1000000,
      price: 1.0,
      change24h: 0.1
    })).toBe(true);

    expect(validateAssetData({
      symbol: 'USDC',
      name: 'USD Coin',
      volume24h: 'invalid', // Invalid type
      price: 1.0,
      change24h: 0.1
    })).toBe(false);
  });
});