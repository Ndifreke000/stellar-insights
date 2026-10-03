import { afterEach, describe, expect, it, vi } from 'vitest';
import { getPaymentPrediction, wilsonInterval } from '@/lib/api/api';

function stubFetch(body: unknown, status = 200) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: status < 400,
    status,
    statusText: 'OK',
    json: async () => body,
    text: async () => JSON.stringify(body),
    headers: new Headers({ 'content-type': 'application/json' }),
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

const corridor = (overrides = {}) => ({
  id: 'USDC:GA->NGN:GB',
  source_asset: 'USDC',
  destination_asset: 'NGN',
  success_rate: 90,
  total_attempts: 200,
  successful_payments: 180,
  health_score: 82,
  ...overrides,
});

describe('wilsonInterval', () => {
  it('brackets the observed rate and narrows with more data', () => {
    const [lowSmall, highSmall] = wilsonInterval(9, 10);
    const [lowLarge, highLarge] = wilsonInterval(900, 1000);
    expect(lowSmall).toBeLessThan(0.9);
    expect(highSmall).toBeGreaterThan(0.9);
    expect(highLarge - lowLarge).toBeLessThan(highSmall - lowSmall);
  });

  it('is uninformative without data', () => {
    expect(wilsonInterval(0, 0)).toEqual([0, 1]);
  });
});

describe('getPaymentPrediction', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('derives the estimate from the pre-payment check', async () => {
    const fetchMock = stubFetch({
      decision: 'caution',
      summary: 'Marginal on: success_rate.',
      corridor: corridor(),
      alternatives: [corridor({ id: 'XLM:native->NGN:GB', source_asset: 'XLM', success_rate: 99, health_score: 95 })],
    });

    const result = await getPaymentPrediction({
      source_asset: 'USDC',
      destination_asset: 'NGN',
      amount: 2500,
      time_of_day: '12:00',
    });

    const [url] = fetchMock.mock.calls[0];
    expect(String(url)).toContain('/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500');
    expect(result.success_probability).toBeCloseTo(0.9);
    expect(result.risk_level).toBe('medium');
    expect(result.recommendation).toBe('Marginal on: success_rate.');
    expect(result.confidence_interval[0]).toBeLessThan(0.9);
    expect(result.confidence_interval[1]).toBeGreaterThan(0.9);
    expect(result.alternative_routes).toHaveLength(1);
    expect(result.alternative_routes[0].estimated_success_rate).toBeCloseTo(0.99);
  });

  it('treats a corridor with no data as high risk, not as a guess', async () => {
    stubFetch({ decision: 'unknown', summary: 'No recent payments.', corridor: null, alternatives: [] });

    const result = await getPaymentPrediction({
      source_asset: 'USDC',
      destination_asset: 'KES',
      amount: 100,
      time_of_day: '12:00',
    });

    expect(result.risk_level).toBe('high');
    expect(result.success_probability).toBe(0);
    expect(result.confidence_interval).toEqual([0, 1]);
  });

  it('fails instead of inventing numbers when the API is down', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('fetch failed')));

    await expect(
      getPaymentPrediction({ source_asset: 'USDC', destination_asset: 'NGN', amount: 1, time_of_day: '12:00' }),
    ).rejects.toThrow();
  });
});
