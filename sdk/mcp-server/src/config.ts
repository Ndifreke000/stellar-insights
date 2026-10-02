import { NETWORKS } from "@payraider/sdk";

export type Transport = "stdio" | "http";

export interface ServerConfig {
  /** PayRaider API key. Optional: without one the free anonymous tier applies. */
  apiKey?: string;
  /** Base URL of the PayRaider backend. */
  baseUrl: string;
  network: "mainnet" | "testnet";
  transport: Transport;
  /** HTTP transport only. */
  host: string;
  port: number;
  /** HTTP transport only: bearer token callers must present, if set. */
  authToken?: string;
  /** HTTP transport only: origins allowed to call from a browser. */
  allowedOrigins: string[];
  allowWrites: boolean;
}

export class ConfigError extends Error {}

function nonEmpty(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

function parsePort(raw: string | undefined): number {
  if (raw === undefined) return 3333;
  const port = Number(raw);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new ConfigError(`PORT must be an integer between 1 and 65535, got "${raw}"`);
  }
  return port;
}

function parseBaseUrl(raw: string): string {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    throw new ConfigError(`PAYRAIDER_BASE_URL is not a valid URL: "${raw}"`);
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new ConfigError(`PAYRAIDER_BASE_URL must be http or https, got "${url.protocol}"`);
  }
  return raw.replace(/\/+$/, "");
}

/**
 * Resolve configuration from the environment and command-line flags.
 * `--http` and `--stdio` override `PAYRAIDER_MCP_TRANSPORT`.
 */
export function loadConfig(
  env: Record<string, string | undefined> = process.env,
  argv: string[] = process.argv.slice(2),
): ServerConfig {
  const network = env.PAYRAIDER_NETWORK === "mainnet" ? "mainnet" : "testnet";

  let transport: Transport = env.PAYRAIDER_MCP_TRANSPORT === "http" ? "http" : "stdio";
  if (argv.includes("--http")) transport = "http";
  if (argv.includes("--stdio")) transport = "stdio";

  const baseUrl = parseBaseUrl(nonEmpty(env.PAYRAIDER_BASE_URL) ?? NETWORKS[network].apiBaseUrl);

  return {
    apiKey: nonEmpty(env.PAYRAIDER_API_KEY),
    baseUrl,
    network,
    transport,
    host: nonEmpty(env.HOST) ?? "127.0.0.1",
    port: parsePort(nonEmpty(env.PORT)),
    authToken: nonEmpty(env.PAYRAIDER_MCP_AUTH_TOKEN),
    allowedOrigins: (env.PAYRAIDER_MCP_ALLOWED_ORIGINS ?? "")
      .split(",")
      .map((origin) => origin.trim())
      .filter(Boolean),
    allowWrites: env.PAYRAIDER_MCP_ALLOW_WRITES === "true",
  };
}
