import AsyncStorage from '@react-native-async-storage/async-storage';
import {
  clearDatabase,
  getCachedAssets,
  getCachedAnchorsList,
  getCachedCorridorsList,
  initializeDatabase,
  setCachedAnchorsList,
  setCachedAssets,
  setCachedCorridorsList,
} from '@services/database';
import type {
  AnchorsCache,
  AssetsCache,
  CorridorsCache,
} from '@services/database';

describe('initializeDatabase', () => {
  beforeEach(async () => {
    await AsyncStorage.clear();
    jest.clearAllMocks();
  });

  it('runs without throwing', async () => {
    await expect(initializeDatabase()).resolves.toBeUndefined();
  });

  it('stores a DB version key after first run', async () => {
    await initializeDatabase();
    const version = await AsyncStorage.getItem('@db_version');
    expect(version).not.toBeNull();
    expect(JSON.parse(version!)).toBe(1);
  });

  it('is idempotent across multiple calls', async () => {
    await initializeDatabase();
    await initializeDatabase();
    const version = await AsyncStorage.getItem('@db_version');
    expect(JSON.parse(version!)).toBe(1);
  });
});

describe('Assets cache', () => {
  const sampleCache: AssetsCache = {
    assets: [
      { code: 'USDC', issuer: 'GISSUER', verified: true },
    ],
    total: 1,
    cachedAt: '2026-06-30T00:00:00.000Z',
  };

  beforeEach(() => AsyncStorage.clear());

  it('returns null when no assets cache exists', async () => {
    expect(await getCachedAssets()).toBeNull();
  });

  it('persists and retrieves assets cache', async () => {
    await setCachedAssets(sampleCache);
    const retrieved = await getCachedAssets();
    expect(retrieved).toEqual(sampleCache);
  });

  it('overwrites existing cache on subsequent writes', async () => {
    await setCachedAssets(sampleCache);
    const updated: AssetsCache = {
      ...sampleCache,
      total: 2,
      assets: [
        ...sampleCache.assets,
        { code: 'EURC', issuer: 'GISSUER2', verified: false },
      ],
    };
    await setCachedAssets(updated);
    const retrieved = await getCachedAssets();
    expect(retrieved?.total).toBe(2);
  });
});

describe('Corridors list cache', () => {
  const sampleCache: CorridorsCache = {
    corridors: [{ id: 'USDC-PHP' }],
    total: 1,
    cachedAt: '2026-06-30T00:00:00.000Z',
  };

  beforeEach(() => AsyncStorage.clear());

  it('returns null when no corridors cache exists', async () => {
    expect(await getCachedCorridorsList()).toBeNull();
  });

  it('persists and retrieves corridors list', async () => {
    await setCachedCorridorsList(sampleCache);
    const retrieved = await getCachedCorridorsList();
    expect(retrieved).toEqual(sampleCache);
  });
});

describe('Anchors list cache', () => {
  const sampleCache: AnchorsCache = {
    anchors: [{ id: 'anchor-1', name: 'MoneyGram' }],
    total: 1,
    cachedAt: '2026-06-30T00:00:00.000Z',
  };

  beforeEach(() => AsyncStorage.clear());

  it('returns null when no anchors cache exists', async () => {
    expect(await getCachedAnchorsList()).toBeNull();
  });

  it('persists and retrieves anchors list', async () => {
    await setCachedAnchorsList(sampleCache);
    const retrieved = await getCachedAnchorsList();
    expect(retrieved).toEqual(sampleCache);
  });
});

describe('clearDatabase', () => {
  beforeEach(() => AsyncStorage.clear());

  it('removes cached corridors, anchors, and assets', async () => {
    await AsyncStorage.setItem('corridors', JSON.stringify({ corridors: [], total: 0 }));
    await AsyncStorage.setItem('anchors', JSON.stringify({ anchors: [], total: 0 }));
    await AsyncStorage.setItem('assets', JSON.stringify({ assets: [], total: 0 }));

    await clearDatabase();

    expect(await AsyncStorage.getItem('corridors')).toBeNull();
    expect(await AsyncStorage.getItem('anchors')).toBeNull();
    expect(await AsyncStorage.getItem('assets')).toBeNull();
  });

  it('removes detail-level cache keys', async () => {
    await AsyncStorage.setItem('corridor_detail:USDC-PHP', '{}');
    await AsyncStorage.setItem('anchor_detail:anchor-1', '{}');

    await clearDatabase();

    expect(await AsyncStorage.getItem('corridor_detail:USDC-PHP')).toBeNull();
    expect(await AsyncStorage.getItem('anchor_detail:anchor-1')).toBeNull();
  });

  it('does not throw when cache is already empty', async () => {
    await expect(clearDatabase()).resolves.toBeUndefined();
  });
});
