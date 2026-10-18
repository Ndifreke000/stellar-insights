export interface PreflightParams {
  sourceAsset: string;
  destinationAsset: string;
  amountUsd: number | string;
  baseUrl?: string;
}

const DEFAULT_BASE_URL = "http://localhost:8080";

function normalizeBaseUrl(baseUrl?: string): string {
  const trimmed = (baseUrl ?? DEFAULT_BASE_URL).trim();
  return trimmed.replace(/\/+$/, "");
}

function normalizeAmount(amountUsd: number | string): string {
  if (typeof amountUsd === "number") {
    if (!Number.isFinite(amountUsd)) {
      throw new Error("amountUsd must be a finite number");
    }
    return String(amountUsd);
  }
  const trimmed = amountUsd.trim();
  if (trimmed === "") {
    throw new Error("amountUsd must not be empty");
  }
  return trimmed;
}

function normalizeAsset(asset: string, field: string): string {
  const trimmed = (asset ?? "").trim();
  if (trimmed === "") {
    throw new Error(`${field} must not be empty`);
  }
  return trimmed;
}

export function buildPreflightQuery(params: PreflightParams): string {
  const sourceAsset = normalizeAsset(params.sourceAsset, "sourceAsset");
  const destinationAsset = normalizeAsset(
    params.destinationAsset,
    "destinationAsset",
  );
  const amountUsd = normalizeAmount(params.amountUsd);

  const search = new URLSearchParams({
    source_asset: sourceAsset,
    destination_asset: destinationAsset,
    amount_usd: amountUsd,
  });
  return search.toString();
}

export function buildPreflightUrl(params: PreflightParams): string {
  const baseUrl = normalizeBaseUrl(params.baseUrl);
  return `${baseUrl}/api/v1/preflight?${buildPreflightQuery(params)}`;
}

export function buildCurlSnippet(params: PreflightParams): string {
  const url = buildPreflightUrl(params);
  return `curl -s "${url}"`;
}

export function buildTypeScriptSnippet(params: PreflightParams): string {
  const url = buildPreflightUrl(params);
  return [
    `const response = await fetch("${url}");`,
    `if (!response.ok) {`,
    `  throw new Error(\`Preflight request failed: \${response.status}\`);`,
    `}`,
    `const preflight = await response.json();`,
    `console.log(preflight);`,
  ].join("\n");
}

export function buildPythonSnippet(params: PreflightParams): string {
  const url = buildPreflightUrl(params);
  return [
    `import requests`,
    ``,
    `response = requests.get("${url}")`,
    `response.raise_for_status()`,
    `preflight = response.json()`,
    `print(preflight)`,
  ].join("\n");
}

export interface PreflightSnippets {
  curl: string;
  typescript: string;
  python: string;
}

export function buildPreflightSnippets(
  params: PreflightParams,
): PreflightSnippets {
  return {
    curl: buildCurlSnippet(params),
    typescript: buildTypeScriptSnippet(params),
    python: buildPythonSnippet(params),
  };
}
