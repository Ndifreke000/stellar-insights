import { z } from "zod";
import type {
  PreflightBatchResponse,
  PreflightCheck,
  PreflightDecision,
  PreflightResponse,
} from "@payraider/sdk";

/**
 * Machine-readable reasons behind a pre-payment decision, so an agent can
 * branch on a code instead of parsing the prose summary.
 *
 * - `ALL_CHECKS_PASSED`: decision is `proceed`.
 * - `NO_RECENT_DATA`: decision is `unknown`.
 * - `<CHECK>_MARGINAL` / `<CHECK>_FAILED`: one per check that warned or
 *   failed, e.g. `LIQUIDITY_FAILED`, `SAMPLE_SIZE_MARGINAL`.
 * - In a batch, an item the backend rejected carries its error code instead
 *   (`INVALID_ASSET`, `INVALID_AMOUNT`, `INVALID_THRESHOLD`).
 */
export function reasonCodes(response: Pick<PreflightResponse, "decision" | "checks">): string[] {
  if (response.decision === "unknown") return ["NO_RECENT_DATA"];
  const codes = response.checks
    .filter((c: PreflightCheck) => c.status !== "pass")
    .map((c: PreflightCheck) => `${c.name.toUpperCase()}_${c.status === "fail" ? "FAILED" : "MARGINAL"}`);
  return codes.length > 0 ? codes : ["ALL_CHECKS_PASSED"];
}

const decision = z.enum(["proceed", "caution", "hold", "unknown"]);
const reasons = z.array(z.string()).describe("Reason codes behind the decision");

/** Structured result of `preflight_payment`: the API response plus reason codes. */
export const PREFLIGHT_OUTPUT_SCHEMA = {
  decision,
  reason_codes: reasons,
  summary: z.string(),
  score: z.number().nullable(),
  corridor: z.record(z.string(), z.unknown()).nullable(),
  checks: z.array(
    z.object({
      name: z.string(),
      status: z.enum(["pass", "warn", "fail"]),
      detail: z.string(),
    }),
  ),
  alternatives: z.array(z.record(z.string(), z.unknown())),
  evaluated_at: z.string(),
} satisfies z.ZodRawShape;

export function presentPreflight(response: PreflightResponse): Presented {
  return {
    structured: { ...response, reason_codes: reasonCodes(response) },
    summary: `${response.decision.toUpperCase()}: ${response.summary}`,
  };
}

/** Structured result of `preflight_batch`, one compact entry per payment. */
export const PREFLIGHT_BATCH_OUTPUT_SCHEMA = {
  evaluated_at: z.string(),
  counts: z.object({
    proceed: z.number().int(),
    caution: z.number().int(),
    hold: z.number().int(),
    unknown: z.number().int(),
    error: z.number().int(),
  }),
  results: z.array(
    z.object({
      index: z.number().int(),
      id: z.string().nullable().describe("The caller's id for this payment, echoed back"),
      decision: decision.nullable().describe("null when the item was rejected; see error"),
      reason_codes: reasons,
      summary: z.string(),
      score: z.number().nullable(),
      corridor_id: z.string().nullable(),
      alternative_corridor_ids: z.array(z.string()),
      error: z.object({ code: z.string(), message: z.string() }).nullable(),
    }),
  ),
} satisfies z.ZodRawShape;

export function presentPreflightBatch(
  response: PreflightBatchResponse,
  ids: (string | undefined)[] = [],
): Presented {
  const counts: Record<PreflightDecision | "error", number> = {
    proceed: 0,
    caution: 0,
    hold: 0,
    unknown: 0,
    error: 0,
  };
  const results = response.results.map((item) => {
    const id = ids[item.index] ?? null;
    if (item.error || !item.result) {
      counts.error += 1;
      const error = item.error ?? { code: "NO_RESULT", message: "the backend returned no result for this item" };
      return {
        index: item.index,
        id,
        decision: null,
        reason_codes: [error.code],
        summary: error.message,
        score: null,
        corridor_id: null,
        alternative_corridor_ids: [],
        error,
      };
    }
    const r = item.result;
    counts[r.decision] += 1;
    return {
      index: item.index,
      id,
      decision: r.decision,
      reason_codes: reasonCodes(r),
      summary: r.summary,
      score: r.score,
      corridor_id: r.corridor?.id ?? null,
      alternative_corridor_ids: r.alternatives.map((a) => a.id),
      error: null,
    };
  });

  const parts = (Object.keys(counts) as (keyof typeof counts)[])
    .filter((k) => counts[k] > 0)
    .map((k) => `${counts[k]} ${k}`);
  return {
    structured: { evaluated_at: response.evaluated_at, counts, results },
    summary: `Checked ${results.length} payment${results.length === 1 ? "" : "s"}: ${parts.join(", ") || "none"}.`,
  };
}

/** What a tool hands back to MCP: structured content plus a one-line summary. */
export interface Presented {
  structured: Record<string, unknown>;
  summary?: string;
}

/**
 * Default for tools without their own presenter. MCP structured content must
 * be a JSON object, so arrays and scalars are wrapped as `{ result }`.
 */
export function presentDefault(result: unknown): Presented {
  const isObject = typeof result === "object" && result !== null && !Array.isArray(result);
  return { structured: isObject ? (result as Record<string, unknown>) : { result } };
}
