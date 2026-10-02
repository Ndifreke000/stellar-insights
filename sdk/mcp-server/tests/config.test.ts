import { describe, expect, it } from "vitest";
import { ConfigError, loadConfig } from "../src/config.js";

describe("loadConfig", () => {
  it("defaults to stdio, testnet, loopback and no API key", () => {
    const config = loadConfig({}, []);
    expect(config.transport).toBe("stdio");
    expect(config.network).toBe("testnet");
    expect(config.host).toBe("127.0.0.1");
    expect(config.port).toBe(3333);
    expect(config.apiKey).toBeUndefined();
    expect(config.authToken).toBeUndefined();
    expect(config.allowedOrigins).toEqual([]);
    expect(config.allowWrites).toBe(false);
  });

  it("treats a blank API key as absent", () => {
    expect(loadConfig({ PAYRAIDER_API_KEY: "   " }, []).apiKey).toBeUndefined();
    expect(loadConfig({ PAYRAIDER_API_KEY: " key " }, []).apiKey).toBe("key");
  });

  it("selects the transport from the environment, and flags override it", () => {
    expect(loadConfig({ PAYRAIDER_MCP_TRANSPORT: "http" }, []).transport).toBe("http");
    expect(loadConfig({}, ["--http"]).transport).toBe("http");
    expect(loadConfig({ PAYRAIDER_MCP_TRANSPORT: "http" }, ["--stdio"]).transport).toBe("stdio");
  });

  it("uses the network's hosted API unless a base URL is given", () => {
    expect(loadConfig({ PAYRAIDER_NETWORK: "mainnet" }, []).baseUrl).toMatch(/^https:\/\//);
    expect(loadConfig({ PAYRAIDER_BASE_URL: "http://localhost:8080/" }, []).baseUrl).toBe(
      "http://localhost:8080",
    );
  });

  it("rejects a base URL that is not http(s)", () => {
    expect(() => loadConfig({ PAYRAIDER_BASE_URL: "not a url" }, [])).toThrow(ConfigError);
    expect(() => loadConfig({ PAYRAIDER_BASE_URL: "ftp://example.com" }, [])).toThrow(ConfigError);
  });

  it("rejects a port that is not a valid TCP port", () => {
    for (const port of ["abc", "0", "70000", "80.5"]) {
      expect(() => loadConfig({ PORT: port }, [])).toThrow(ConfigError);
    }
    expect(loadConfig({ PORT: "8080" }, []).port).toBe(8080);
  });

  it("parses the allowed origins list", () => {
    const config = loadConfig(
      { PAYRAIDER_MCP_ALLOWED_ORIGINS: "https://a.example, https://b.example ,," },
      [],
    );
    expect(config.allowedOrigins).toEqual(["https://a.example", "https://b.example"]);
  });
});
