import { NextResponse } from "next/server";
import * as Sentry from "@sentry/nextjs";

const MAX_REPORT_BYTES = 16 * 1024;

type CspViolation = Record<string, unknown>;

/**
 * Receives CSP violation reports, both the legacy `report-uri` format
 * (`application/csp-report`) and the Reporting API format (`application/reports+json`).
 */
export async function POST(request: Request) {
  const text = await request.text();
  if (text.length > MAX_REPORT_BYTES) {
    return new NextResponse(null, { status: 413 });
  }

  let payload: unknown;
  try {
    payload = JSON.parse(text);
  } catch {
    return new NextResponse(null, { status: 400 });
  }

  const violations: CspViolation[] = Array.isArray(payload)
    ? payload
        .filter((r) => r?.type === "csp-violation")
        .map((r) => r.body as CspViolation)
    : [((payload as { "csp-report"?: CspViolation })["csp-report"] ??
        payload) as CspViolation];

  for (const v of violations) {
    const summary = {
      documentURI: v["document-uri"] ?? v.documentURL,
      violatedDirective: v["violated-directive"] ?? v.effectiveDirective,
      blockedURI: v["blocked-uri"] ?? v.blockedURL,
      sourceFile: v["source-file"] ?? v.sourceFile,
      lineNumber: v["line-number"] ?? v.lineNumber,
      disposition: v.disposition,
      userAgent: request.headers.get("user-agent"),
    };
    console.warn("[csp-violation]", JSON.stringify(summary));

    if (process.env.SENTRY_DSN) {
      Sentry.captureMessage(
        `CSP violation: ${String(summary.violatedDirective)}`,
        { level: "warning", extra: summary },
      );
    }
  }

  return new NextResponse(null, { status: 204 });
}
