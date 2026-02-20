/**
 * Trustline API Client
 * Handles all trustline data fetching from the backend
 */

export interface TrustlineAsset {
  asset_code: string;
  asset_issuer: string;
  asset_type: string;
  num_accounts: number;
  amount: number;
  num_accounts_authorized: number;
  num_accounts_authorized_to_maintain: number;
  flags_auth_required: number;
  flags_auth_revocable: number;
  flags_auth_immutable: number;
  flags_auth_clawback: number;
  paging_token: string | null;
  last_synced_at: string;
  created_at: string;
  updated_at: string;
}

export interface TrustlineSnapshot {
  id: number;
  asset_code: string;
  asset_issuer: string;
  num_accounts: number;
  amount: number;
  snapshot_at: string;
}

export interface TrustlineStats {
  total_assets: number;
  total_trustlines: number;
  avg_trustlines: number;
  top_asset_code: string;
  top_asset_trustlines: number;
  assets_growing: number;
  assets_declining: number;
}

export interface TrustlineChange {
  asset_code: string;
  asset_issuer: string;
  current_accounts: number;
  previous_accounts: number;
  change_amount: number;
  change_pct: number;
  snapshot_at: string;
}

export interface AssetDetailResponse {
  asset: TrustlineAsset;
  snapshots: TrustlineSnapshot[];
}

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

async function safeFetch<T>(url: string, fallback: T): Promise<T> {
  try {
    const response = await fetch(url, {
      method: "GET",
      headers: { "Content-Type": "application/json" },
    });
    if (!response.ok) throw new Error(`API error: ${response.status}`);
    return response.json();
  } catch (error) {
    const isNetworkError =
      error instanceof TypeError &&
      (error.message.includes("Failed to fetch") ||
        error.message.includes("Network request failed"));
    if (!isNetworkError) {
      console.error(`Failed to fetch ${url}:`, error);
    }
    return fallback;
  }
}

export async function fetchTrustlineAssets(): Promise<TrustlineAsset[]> {
  return safeFetch(`${API_BASE}/api/trustlines/`, getMockAssets());
}

export async function fetchTrustlineStats(): Promise<TrustlineStats> {
  return safeFetch(`${API_BASE}/api/trustlines/stats`, getMockStats());
}

export async function fetchTrustlineRankings(
  sortBy: string = "trustlines",
  limit: number = 20,
): Promise<TrustlineAsset[]> {
  return safeFetch(
    `${API_BASE}/api/trustlines/rankings?sort_by=${sortBy}&limit=${limit}`,
    getMockAssets(),
  );
}

export async function fetchTrustlineChanges(
  limit: number = 20,
): Promise<TrustlineChange[]> {
  return safeFetch(
    `${API_BASE}/api/trustlines/changes?limit=${limit}`,
    getMockChanges(),
  );
}

export async function fetchAssetDetail(
  assetKey: string,
): Promise<AssetDetailResponse> {
  return safeFetch(
    `${API_BASE}/api/trustlines/${assetKey}`,
    getMockAssetDetail(assetKey),
  );
}

export async function fetchAssetSnapshots(
  assetKey: string,
  limit: number = 100,
): Promise<TrustlineSnapshot[]> {
  return safeFetch(
    `${API_BASE}/api/trustlines/${assetKey}/snapshots?limit=${limit}`,
    getMockSnapshots(assetKey),
  );
}

// =============================================================================
// Mock Data
// =============================================================================

function getMockAssets(): TrustlineAsset[] {
  const now = new Date().toISOString();
  return [
    {
      asset_code: "USDC",
      asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      asset_type: "credit_alphanum4",
      num_accounts: 285400,
      amount: 1250000000.0,
      num_accounts_authorized: 285400,
      num_accounts_authorized_to_maintain: 0,
      flags_auth_required: 1,
      flags_auth_revocable: 1,
      flags_auth_immutable: 0,
      flags_auth_clawback: 1,
      paging_token: "USDC_0",
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      asset_code: "yUSDC",
      asset_issuer: "GDGTVWSM4MGS2T7Z7GVZE5SAEVLSWM5SGY5Q2EMUQWRMEV2RNYY3YFG6",
      asset_type: "credit_alphanum4",
      num_accounts: 142300,
      amount: 890000000.0,
      num_accounts_authorized: 142300,
      num_accounts_authorized_to_maintain: 0,
      flags_auth_required: 0,
      flags_auth_revocable: 1,
      flags_auth_immutable: 0,
      flags_auth_clawback: 0,
      paging_token: "yUSDC_1",
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      asset_code: "AQUA",
      asset_issuer: "GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
      asset_type: "credit_alphanum4",
      num_accounts: 98700,
      amount: 10000000000.0,
      num_accounts_authorized: 98700,
      num_accounts_authorized_to_maintain: 0,
      flags_auth_required: 1,
      flags_auth_revocable: 0,
      flags_auth_immutable: 0,
      flags_auth_clawback: 0,
      paging_token: "AQUA_2",
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      asset_code: "SHX",
      asset_issuer: "GDSTRSHXHGJ7ZIVRBXEYE5Q74XUVCUSEZ6IRLT2WN6OS2IF5KSGAU2VH",
      asset_type: "credit_alphanum4",
      num_accounts: 67200,
      amount: 750000000.0,
      num_accounts_authorized: 67200,
      num_accounts_authorized_to_maintain: 0,
      flags_auth_required: 0,
      flags_auth_revocable: 0,
      flags_auth_immutable: 0,
      flags_auth_clawback: 0,
      paging_token: "SHX_3",
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      asset_code: "EURC",
      asset_issuer: "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y36DAVIZA67CE7BKBHP4V2OA",
      asset_type: "credit_alphanum4",
      num_accounts: 54100,
      amount: 320000000.0,
      num_accounts_authorized: 54100,
      num_accounts_authorized_to_maintain: 0,
      flags_auth_required: 1,
      flags_auth_revocable: 1,
      flags_auth_immutable: 0,
      flags_auth_clawback: 1,
      paging_token: "EURC_4",
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      asset_code: "RMT",
      asset_issuer: "GDEGOXPCHXWFYY234D2YNNFBY5HAJL5XZURXQBE4KEHBHPCMOJK6PMLH",
      asset_type: "credit_alphanum4",
      num_accounts: 41800,
      amount: 500000000.0,
      num_accounts_authorized: 41800,
      num_accounts_authorized_to_maintain: 0,
      flags_auth_required: 0,
      flags_auth_revocable: 1,
      flags_auth_immutable: 0,
      flags_auth_clawback: 0,
      paging_token: "RMT_5",
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      asset_code: "MOBI",
      asset_issuer: "GA6HCMBLTZS5VYYBCATRBRZ3BZJMAFUDKYYF6AH6MVCMGWMRDNSWJPIH",
      asset_type: "credit_alphanum4",
      num_accounts: 28900,
      amount: 890000000.0,
      num_accounts_authorized: 28900,
      num_accounts_authorized_to_maintain: 0,
      flags_auth_required: 1,
      flags_auth_revocable: 0,
      flags_auth_immutable: 0,
      flags_auth_clawback: 0,
      paging_token: "MOBI_6",
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      asset_code: "SLT",
      asset_issuer: "GCKA6K6MRTDWFO2BBCZJYAQ4QR4DMSSIJLHL5BOG2UNXLCTKOBDMYEAM",
      asset_type: "credit_alphanum4",
      num_accounts: 15600,
      amount: 150000000.0,
      num_accounts_authorized: 15600,
      num_accounts_authorized_to_maintain: 0,
      flags_auth_required: 0,
      flags_auth_revocable: 0,
      flags_auth_immutable: 0,
      flags_auth_clawback: 0,
      paging_token: "SLT_7",
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
  ];
}

function getMockStats(): TrustlineStats {
  const assets = getMockAssets();
  const totalTrustlines = assets.reduce((s, a) => s + a.num_accounts, 0);
  return {
    total_assets: assets.length,
    total_trustlines: totalTrustlines,
    avg_trustlines: Math.round(totalTrustlines / assets.length),
    top_asset_code: "USDC",
    top_asset_trustlines: 285400,
    assets_growing: 6,
    assets_declining: 2,
  };
}

function getMockChanges(): TrustlineChange[] {
  const now = new Date().toISOString();
  return [
    {
      asset_code: "USDC",
      asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      current_accounts: 285400,
      previous_accounts: 282100,
      change_amount: 3300,
      change_pct: 1.17,
      snapshot_at: now,
    },
    {
      asset_code: "AQUA",
      asset_issuer: "GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
      current_accounts: 98700,
      previous_accounts: 96200,
      change_amount: 2500,
      change_pct: 2.6,
      snapshot_at: now,
    },
    {
      asset_code: "yUSDC",
      asset_issuer: "GDGTVWSM4MGS2T7Z7GVZE5SAEVLSWM5SGY5Q2EMUQWRMEV2RNYY3YFG6",
      current_accounts: 142300,
      previous_accounts: 140800,
      change_amount: 1500,
      change_pct: 1.07,
      snapshot_at: now,
    },
    {
      asset_code: "SHX",
      asset_issuer: "GDSTRSHXHGJ7ZIVRBXEYE5Q74XUVCUSEZ6IRLT2WN6OS2IF5KSGAU2VH",
      current_accounts: 67200,
      previous_accounts: 68400,
      change_amount: -1200,
      change_pct: -1.75,
      snapshot_at: now,
    },
    {
      asset_code: "RMT",
      asset_issuer: "GDEGOXPCHXWFYY234D2YNNFBY5HAJL5XZURXQBE4KEHBHPCMOJK6PMLH",
      current_accounts: 41800,
      previous_accounts: 42500,
      change_amount: -700,
      change_pct: -1.65,
      snapshot_at: now,
    },
    {
      asset_code: "EURC",
      asset_issuer: "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y36DAVIZA67CE7BKBHP4V2OA",
      current_accounts: 54100,
      previous_accounts: 53200,
      change_amount: 900,
      change_pct: 1.69,
      snapshot_at: now,
    },
  ];
}

function getMockSnapshots(assetKey: string): TrustlineSnapshot[] {
  const code = assetKey.split(":")[0] || "USDC";
  const issuer = assetKey.split(":")[1] || "";
  const now = new Date();
  const baseAccounts: Record<string, number> = {
    USDC: 285400,
    yUSDC: 142300,
    AQUA: 98700,
    SHX: 67200,
    EURC: 54100,
    RMT: 41800,
    MOBI: 28900,
    SLT: 15600,
  };
  const base = baseAccounts[code] || 50000;

  return Array.from({ length: 30 }, (_, i) => {
    const date = new Date(now);
    date.setDate(date.getDate() - (29 - i));
    // Simulate growth trend with some noise
    const growthFactor = 1 + (i / 30) * 0.05;
    const noise = 1 + (Math.random() - 0.5) * 0.02;
    const accounts = Math.round(base * growthFactor * noise * 0.97);
    return {
      id: i + 1,
      asset_code: code,
      asset_issuer: issuer,
      num_accounts: accounts,
      amount: accounts * 4500 + Math.random() * 100000,
      snapshot_at: date.toISOString(),
    };
  });
}

function getMockAssetDetail(assetKey: string): AssetDetailResponse {
  const assets = getMockAssets();
  const code = assetKey.split(":")[0];
  const asset = assets.find((a) => a.asset_code === code) || assets[0];
  return {
    asset,
    snapshots: getMockSnapshots(assetKey),
  };
}
