import createMiddleware from "next-intl/middleware";
import { routing } from "./i18n/routing";
import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';
import { generateCsrfToken, validateCsrfToken } from './lib/csrf';

const intlMiddleware = createMiddleware(routing);

/** Receives CSP violation reports; see src/app/api/csp-report/route.ts. */
const CSP_REPORT_PATH = "/api/csp-report";

/**
 * Content Security Policy for page responses.
 *
 * Notes:
 * - Scripts are allowed by a per-request nonce plus 'strict-dynamic'. Next.js reads
 *   the nonce from the request's CSP header and applies it to the scripts it renders;
 *   scripts those load (e.g. Redoc on /api-docs) are trusted transitively. 'self' and
 *   the Redoc CDN are fallbacks for browsers without 'strict-dynamic' support.
 * - `style-src 'unsafe-inline'` stays: charting/animation libraries set inline styles.
 * - `connect-src` includes wss: for WebSocket, *.sentry.io for error reporting,
 *   and Stellar RPC/Horizon endpoints for contract calls.
 * - `upgrade-insecure-requests` is omitted in development to avoid breaking http://localhost.
 */
function buildCsp(isProd: boolean, nonce: string): string {
  const directives: string[] = [
    "default-src 'self'",
    // 'unsafe-eval' is only added outside production: Next.js dev mode
    // (Turbopack/Fast Refresh) evals modules to reconstruct call stacks,
    // and without it every page load throws a CSP console error.
    `script-src 'self' 'nonce-${nonce}' 'strict-dynamic' https://cdn.redoc.ly${isProd ? "" : " 'unsafe-eval'"}`,
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data: blob: https://*.stellar.org",
    "font-src 'self'",
    "connect-src 'self' wss: https: https://*.sentry.io https://soroban-testnet.stellar.org https://stellar.api.onfinality.io https://horizon-testnet.stellar.org https://horizon.stellar.org",
    "worker-src 'self' blob:",
    "frame-src 'none'",
    "frame-ancestors 'none'",
    "object-src 'none'",
    "base-uri 'self'",
    "form-action 'self'",
    `report-uri ${CSP_REPORT_PATH}`,
    "report-to csp-endpoint",
  ];

  if (isProd) {
    directives.push("upgrade-insecure-requests");
  }

  return directives.join("; ");
}

/** API route handlers only return data; they never load resources or get framed. */
const API_CSP = "default-src 'none'; frame-ancestors 'none'";

/** Set CSP_REPORT_ONLY=true to roll a policy change out without enforcing it. */
function cspHeaderName(): string {
  return process.env.CSP_REPORT_ONLY === "true"
    ? "Content-Security-Policy-Report-Only"
    : "Content-Security-Policy";
}

function generateNonce(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return btoa(Array.from(bytes, (b) => String.fromCharCode(b)).join(""));
}

function applySecurityHeaders(response: NextResponse, csp: string): void {
  response.headers.set(cspHeaderName(), csp);
  response.headers.set("Reporting-Endpoints", `csp-endpoint="${CSP_REPORT_PATH}"`);
  response.headers.set("X-Frame-Options", "DENY");
  response.headers.set("X-Content-Type-Options", "nosniff");
  response.headers.set("Referrer-Policy", "strict-origin-when-cross-origin");
  response.headers.set(
    "Permissions-Policy",
    "camera=(), microphone=(), geolocation=()"
  );
}

export default async function middleware(request: NextRequest) {
  const isProd = process.env.NODE_ENV === "production";

  if (request.nextUrl.pathname.startsWith("/api/")) {
    return handleApiRequest(request, isProd);
  }

  const nonce = generateNonce();
  const csp = buildCsp(isProd, nonce);
  // Next.js picks the nonce up from the request's CSP header during rendering;
  // next-intl forwards the incoming request headers.
  request.headers.set("x-nonce", nonce);
  request.headers.set(cspHeaderName(), csp);

  const response = intlMiddleware(request);
  applySecurityHeaders(response, csp);
  return response;
}

async function handleApiRequest(request: NextRequest, isProd: boolean) {
  const response = NextResponse.next();
  applySecurityHeaders(response, API_CSP);

  // Browsers send CSP violation reports without a CSRF token.
  if (request.nextUrl.pathname === CSP_REPORT_PATH) {
    return response;
  }

  if (request.method === "GET") {
    const csrfToken = await generateCsrfToken();
    response.cookies.set("csrf-token", csrfToken, {
      httpOnly: true,
      secure: isProd,
      sameSite: "strict",
      path: "/",
      maxAge: 60 * 60 * 24,
    });
    response.headers.set("X-CSRF-Token", csrfToken);
    return response;
  }

  if (["POST", "PUT", "DELETE", "PATCH"].includes(request.method)) {
    const cookieToken = request.cookies.get("csrf-token")?.value;
    const headerToken = request.headers.get("X-CSRF-Token") ?? undefined;

    if (!validateCsrfToken(cookieToken, headerToken)) {
      return NextResponse.json(
        {
          error: "Invalid CSRF token",
          message:
            "CSRF token validation failed. Please refresh the page and try again.",
        },
        { status: 403 }
      );
    }
  }

  return response;
}

export const config = {
  matcher: [
    "/((?!_next|_vercel|.*\\..*).*)", // All routes except static files
  ],
};
