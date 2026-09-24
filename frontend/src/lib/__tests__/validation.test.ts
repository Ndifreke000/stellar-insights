import { describe, it, expect } from "vitest";
import { z } from "zod";
import {
  sep24FlowSchema,
  sep31PaymentFlowSchema,
  costCalculatorSchema,
  formatZodError,
  VALIDATION_MESSAGES
} from "../schemas";

describe("Validation Schemas", () => {
  describe("SEP-24 Flow Schema", () => {
    it("should validate valid SEP-24 form data", () => {
      const validData = {
        transferServer: "https://api.anchor.example/sep24",
        assetCode: "USDC",
        amount: "100.50",
        account: "GABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890ABCDEFGHI",
        jwt: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test",
      };

      const result = sep24FlowSchema.safeParse(validData);
      expect(result.success).toBe(true);
    });

    it("should validate SEP-24 form with optional fields empty", () => {
      const validData = {
        transferServer: "https://api.anchor.example/sep24",
        assetCode: "USDC",
        amount: "",
        account: "",
        jwt: "",
      };

      const result = sep24FlowSchema.safeParse(validData);
      expect(result.success).toBe(true);
    });

    it("should reject invalid transfer server URL", () => {
      const invalidData = {
        transferServer: "http://not-secure.com",
        assetCode: "USDC",
        amount: "100",
        account: "",
        jwt: "",
      };

      const result = sep24FlowSchema.safeParse(invalidData);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues[0].message).toBe(VALIDATION_MESSAGES.HTTPS_REQUIRED);
      }
    });

    it("should reject invalid amount", () => {
      const invalidData = {
        transferServer: "https://api.anchor.example/sep24",
        assetCode: "USDC",
        amount: "-50",
        account: "",
        jwt: "",
      };

      const result = sep24FlowSchema.safeParse(invalidData);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues[0].message).toBe(VALIDATION_MESSAGES.INVALID_AMOUNT);
      }
    });

    it("should reject invalid Stellar account", () => {
      const invalidData = {
        transferServer: "https://api.anchor.example/sep24",
        assetCode: "USDC",
        amount: "100",
        account: "invalid-account",
        jwt: "",
      };

      const result = sep24FlowSchema.safeParse(invalidData);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues[0].message).toBe(VALIDATION_MESSAGES.INVALID_STELLAR_ACCOUNT);
      }
    });
  });

  describe("SEP-31 Payment Flow Schema", () => {
    it("should validate valid SEP-31 form data", () => {
      const validData = {
        transferServer: "https://api.anchor.example/sep31",
        amount: "1000",
        receiverId: "receiver@example.com",
        sourceAsset: "USDC",
        destAsset: "USD",
        jwt: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
      };

      const result = sep31PaymentFlowSchema.safeParse(validData);
      expect(result.success).toBe(true);
    });

    it("should require receiver ID", () => {
      const invalidData = {
        transferServer: "https://api.anchor.example/sep31",
        amount: "1000",
        receiverId: "",
        sourceAsset: "",
        destAsset: "",
        jwt: "",
      };

      const result = sep31PaymentFlowSchema.safeParse(invalidData);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues[0].message).toBe(VALIDATION_MESSAGES.RECEIVER_ID_LENGTH);
      }
    });

    it("should reject receiver ID that's too short", () => {
      const invalidData = {
        transferServer: "https://api.anchor.example/sep31",
        amount: "1000",
        receiverId: "ab",
        sourceAsset: "",
        destAsset: "",
        jwt: "",
      };

      const result = sep31PaymentFlowSchema.safeParse(invalidData);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues[0].message).toBe(VALIDATION_MESSAGES.RECEIVER_ID_LENGTH);
      }
    });
  });

  describe("Cost Calculator Schema", () => {
    it("should validate valid cost calculator data", () => {
      const validData = {
        sourceCurrency: "USDC",
        destinationCurrency: "NGN",
        sourceAmount: "1000",
        destinationAmount: "",
        routes: ["stellar_dex", "anchor_direct"],
      };

      const result = costCalculatorSchema.safeParse(validData);
      expect(result.success).toBe(true);
    });

    it("should reject same source and destination currency", () => {
      const invalidData = {
        sourceCurrency: "USDC",
        destinationCurrency: "USDC",
        sourceAmount: "1000",
        destinationAmount: "",
        routes: ["stellar_dex"],
      };

      const result = costCalculatorSchema.safeParse(invalidData);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues[0].message).toBe(VALIDATION_MESSAGES.CURRENCY_DIFFERENT);
      }
    });

    it("should reject empty routes array", () => {
      const invalidData = {
        sourceCurrency: "USDC",
        destinationCurrency: "NGN",
        sourceAmount: "1000",
        destinationAmount: "",
        routes: [],
      };

      const result = costCalculatorSchema.safeParse(invalidData);
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error.issues[0].message).toBe(VALIDATION_MESSAGES.SELECT_ROUTE);
      }
    });
  });
});

describe("Error Formatting", () => {
  it("should format Zod errors correctly", () => {
    const schema = z.object({
      name: z.string().min(1, "Name is required"),
      age: z.number().min(18, "Must be 18 or older"),
    });

    const result = schema.safeParse({
      name: "",
      age: 16,
    });

    expect(result.success).toBe(false);
    if (!result.success) {
      const formattedErrors = formatZodError(result.error);

      expect(formattedErrors).toEqual({
        name: "Name is required",
        age: "Must be 18 or older",
      });
    }
  });

  it("should handle nested field paths", () => {
    const schema = z.object({
      user: z.object({
        email: z.string().email("Invalid email"),
        profile: z.object({
          name: z.string().min(1, "Name required"),
        }),
      }),
    });

    const result = schema.safeParse({
      user: {
        email: "invalid-email",
        profile: {
          name: "",
        },
      },
    });

    expect(result.success).toBe(false);
    if (!result.success) {
      const formattedErrors = formatZodError(result.error);

      expect(formattedErrors).toEqual({
        "user.email": "Invalid email",
        "user.profile.name": "Name required",
      });
    }
  });
});

describe("Validation Messages", () => {
  it("should contain all required validation messages", () => {
    expect(VALIDATION_MESSAGES.REQUIRED).toBeDefined();
    expect(VALIDATION_MESSAGES.INVALID_URL).toBeDefined();
    expect(VALIDATION_MESSAGES.HTTPS_REQUIRED).toBeDefined();
    expect(VALIDATION_MESSAGES.INVALID_AMOUNT).toBeDefined();
    expect(VALIDATION_MESSAGES.INVALID_STELLAR_ACCOUNT).toBeDefined();
    expect(VALIDATION_MESSAGES.RECEIVER_ID_LENGTH).toBeDefined();
    expect(VALIDATION_MESSAGES.INVALID_ASSET).toBeDefined();
    expect(VALIDATION_MESSAGES.JWT_TOO_SHORT).toBeDefined();
    expect(VALIDATION_MESSAGES.CURRENCY_DIFFERENT).toBeDefined();
    expect(VALIDATION_MESSAGES.SELECT_ROUTE).toBeDefined();
  });

  it("should have consistent message format", () => {
    Object.values(VALIDATION_MESSAGES).forEach(message => {
      expect(typeof message).toBe("string");
      expect(message.length).toBeGreaterThan(0);
      expect(message[0]).toBe(message[0].toUpperCase() || message[0]);
    });
  });
});
