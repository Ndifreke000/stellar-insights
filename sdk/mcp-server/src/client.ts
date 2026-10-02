import { PayRaider } from "@payraider/sdk";
import type { ServerConfig } from "./config.js";
import type { PayRaiderClient } from "./sdk-types.js";

/**
 * One client per server process, matching how every other SDK consumer
 * authenticates. The API key is optional: without it requests go out
 * unauthenticated and the backend's anonymous rate-limit tier applies, which
 * is enough for the pre-payment check and the other public read endpoints.
 */
export function createClient(config: Pick<ServerConfig, "apiKey" | "baseUrl">): PayRaiderClient {
  return new PayRaider({ apiKey: config.apiKey, baseUrl: config.baseUrl });
}
