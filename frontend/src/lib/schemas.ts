import { z } from "zod";

// Base validation patterns
const urlSchema = z.string().url("Enter a valid URL (e.g., https://api.example.com/sep24)").refine(
  (url) => url.startsWith("https://"),
  { message: "URL must use HTTPS" }
);

const amountSchema = z.string()
  .min(1, "Amount is required")
  .refine(
    (val) => !isNaN(parseFloat(val)) && parseFloat(val) > 0,
    { message: "Amount must be a positive number" }
  )
  .refine(
    (val) => parseFloat(val) <= 1e12,
    { message: "Amount too large" }
  )
  .refine(
    (val) => {
      const parts = val.split(".");
      return parts.length <= 2 && (!parts[1] || parts[1].length <= 7);
    },
    { message: "Max 7 decimal places" }
  );

const stellarAccountSchema = z.string()
  .regex(/^G[A-Z0-9]{55}$/, "Invalid Stellar account (must be 56 chars starting with G)")
  .optional()
  .or(z.literal(""));

const receiverIdSchema = z.string()
  .min(3, "Receiver ID must be 3-256 chars")
  .max(256, "Receiver ID must be 3-256 chars");

const assetCodeSchema = z.string()
  .regex(/^[A-Z0-9]+(:[A-Z0-9]+)?$/, "Invalid asset (e.g., USDC or USDC:GBEZET...)")
  .max(64, "Asset code too long")
  .optional()
  .or(z.literal(""));

const jwtSchema = z.string()
  .min(10, "JWT too short")
  .optional()
  .or(z.literal(""));

// SEP-24 Flow Schema
export const sep24FlowSchema = z.object({
  transferServer: urlSchema,
  assetCode: z.string().min(1, "Asset is required"),
  amount: z.string().optional().or(z.literal("")).refine(
    (val) => {
      if (!val || val === "") return true; // Optional
      const num = parseFloat(val);
      return !isNaN(num) && num > 0 && num <= 1e12;
    },
    { message: "Amount must be a positive number" }
  ).refine(
    (val) => {
      if (!val || val === "") return true;
      const parts = val.split(".");
      return parts.length <= 2 && (!parts[1] || parts[1].length <= 7);
    },
    { message: "Max 7 decimal places" }
  ),
  account: stellarAccountSchema,
  jwt: jwtSchema,
});

// SEP-31 Payment Flow Schema
export const sep31PaymentFlowSchema = z.object({
  transferServer: urlSchema,
  amount: amountSchema,
  receiverId: receiverIdSchema,
  sourceAsset: assetCodeSchema,
  destAsset: assetCodeSchema,
  jwt: jwtSchema,
});

// Cost Calculator Schema
export const costCalculatorSchema = z.object({
  sourceCurrency: z.string().min(1, "Source currency is required"),
  destinationCurrency: z.string().min(1, "Destination currency is required"),
  sourceAmount: amountSchema,
  destinationAmount: z.string().optional().or(z.literal("")).refine(
    (val) => {
      if (!val || val === "") return true; // Optional
      const num = parseFloat(val);
      return !isNaN(num) && num > 0 && num <= 1e12;
    },
    { message: "Amount must be a positive number" }
  ),
  routes: z.array(z.string()).min(1, "Select at least one route"),
}).refine(
  (data) => data.sourceCurrency !== data.destinationCurrency,
  {
    message: "Source and destination currencies must differ",
    path: ["destinationCurrency"]
  }
);

// Form types
export type Sep24FlowForm = z.infer<typeof sep24FlowSchema>;
export type Sep31PaymentFlowForm = z.infer<typeof sep31PaymentFlowSchema>;
export type CostCalculatorForm = z.infer<typeof costCalculatorSchema>;

// Validation error formatter
export const formatZodError = (error: z.ZodError): Record<string, string> => {
  const errors: Record<string, string> = {};

  if (error.issues && Array.isArray(error.issues)) {
    error.issues.forEach((err) => {
      const field = err.path.join(".");
      errors[field] = err.message;
    });
  }

  return errors;
};

// Common validation messages for consistency
export const VALIDATION_MESSAGES = {
  REQUIRED: "This field is required",
  INVALID_URL: "Enter a valid URL (e.g., https://api.example.com/sep24)",
  HTTPS_REQUIRED: "URL must use HTTPS",
  INVALID_AMOUNT: "Amount must be a positive number",
  AMOUNT_TOO_LARGE: "Amount too large",
  DECIMAL_PLACES: "Max 7 decimal places",
  INVALID_STELLAR_ACCOUNT: "Invalid Stellar account (must be 56 chars starting with G)",
  RECEIVER_ID_LENGTH: "Receiver ID must be 3-256 chars",
  INVALID_ASSET: "Invalid asset (e.g., USDC or USDC:GBEZET...)",
  JWT_TOO_SHORT: "JWT too short",
  CURRENCY_DIFFERENT: "Source and destination currencies must differ",
  SELECT_ROUTE: "Select at least one route",
} as const;
