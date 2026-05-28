import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import {
  AnalyticsAPIModule,
  NPMPublishingSetup,
  ReactNativeCompatibility,
  SDKError,
  SDKUnitTests,
  TypeScriptTypes,
} from "../src/index.js";

describe("SDK extensions", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  describe("SDKUnitTests", () => {
    it("executes successfully with valid parameters", async () => {
      const sdk = new SDKUnitTests({ apiKey: "key" });
      const result = await sdk.execute({ suiteName: "core-suite", timeoutMs: 10000 });
      expect(result.success).toBe(true);
      expect(result.data.suiteName).toBe("core-suite");
      expect(result.data.testsRun).toBe(0);
      expect(typeof result.data.executedAt).toBe("string");
    });

    it("throws SDKError when suiteName is missing", async () => {
      const sdk = new SDKUnitTests();
      await expect(sdk.execute({ suiteName: "" } as any)).rejects.toBeInstanceOf(SDKError);
    });
  });

  describe("ReactNativeCompatibility", () => {
    const originalNavigator = globalThis.navigator;

    afterEach(() => {
      Object.defineProperty(globalThis, "navigator", {
        value: originalNavigator,
        configurable: true,
      });
    });

    it("returns browser environment when navigator is not React Native", async () => {
      Object.defineProperty(globalThis, "navigator", {
        value: { product: "Gecko" },
        configurable: true,
      });

      const compatibility = new ReactNativeCompatibility();
      const result = await compatibility.execute({ clientName: "web-client" });
      expect(result.environment).toBe("browser");
      expect(result.supported).toBe(true);
    });

    it("detects React Native environment successfully", async () => {
      Object.defineProperty(globalThis, "navigator", {
        value: { product: "ReactNative" },
        configurable: true,
      });

      const compatibility = new ReactNativeCompatibility();
      const result = await compatibility.execute({ clientName: "mobile-client", debug: true });
      expect(result.environment).toBe("react-native");
      expect(result.data.debug).toBe(true);
    });
  });

  describe("NPMPublishingSetup", () => {
    it("configures publishing with defaults", async () => {
      const npmSetup = new NPMPublishingSetup();
      const result = await npmSetup.execute({ packageName: "@stellar-insights/sdk", version: "1.0.0" });
      expect(result.success).toBe(true);
      expect(result.data.registryUrl).toBe("https://registry.npmjs.org");
      expect(result.data.packageName).toBe("@stellar-insights/sdk");
    });

    it("throws SDKError for invalid packageName", async () => {
      const npmSetup = new NPMPublishingSetup();
      await expect(npmSetup.execute({ packageName: "invalid name", version: "1.0.0" })).rejects.toBeInstanceOf(SDKError);
    });
  });

  describe("AnalyticsAPIModule", () => {
    it("records analytics events with typed response", async () => {
      const analytics = new AnalyticsAPIModule({ apiKey: "key" });
      const result = await analytics.execute({ event: "payment.created", payload: { amount: 150 } });
      expect(result.success).toBe(true);
      expect(result.data.recordedEvent).toBe("payment.created");
      expect(result.data.properties.amount).toBe(150);
    });

    it("throws SDKError if event is invalid", async () => {
      const analytics = new AnalyticsAPIModule();
      await expect(analytics.execute({ event: "", payload: {} } as any)).rejects.toBeInstanceOf(SDKError);
    });
  });

  describe("TypeScriptTypes", () => {
    it("resolves successfully with valid typeName", async () => {
      const ts = new TypeScriptTypes({ apiKey: "key" });
      const result = await ts.execute({ typeName: "Anchor", verbose: true });
      expect(result.success).toBe(true);
      expect(result.data.typeName).toBe("Anchor");
      expect(result.data.verbose).toBe(true);
      expect(typeof result.data.resolvedAt).toBe("string");
    });

    it("attaches optional metadata to result", async () => {
      const ts = new TypeScriptTypes();
      const result = await ts.execute({ typeName: "Corridor", metadata: { version: 2 } });
      expect(result.data.metadata).toEqual({ version: 2 });
    });

    it("throws SDKError when typeName is empty", async () => {
      const ts = new TypeScriptTypes();
      await expect(ts.execute({ typeName: "" } as any)).rejects.toBeInstanceOf(SDKError);
    });

    it("throws SDKError when typeName is missing", async () => {
      const ts = new TypeScriptTypes();
      await expect(ts.execute({} as any)).rejects.toBeInstanceOf(SDKError);
    });
  });
});
