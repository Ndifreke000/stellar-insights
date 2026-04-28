import createMiddleware from "next-intl/middleware";
import { routing } from "./i18n/routing";
import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';
import { generateCsrfToken, validateCsrfToken } from './lib/csrf';

const intlMiddleware = createMiddleware(routing);

/**
 * Content Security Policy directives.
 *
 * Notes:
 * - `script-src 'unsafe-inline'` is required by Next.js for inline hydration scripts.
 *   A nonce-based approach can replace this once Next.js nonce support is wired up.
 * - `connect-src` includes wss: for WebSocket and *.sentry.io for error reporting.
 * - `upgrade-insecure-requests` is omitted in development to avoid breaking http://localhost.
 */
function buildCsp(isProd: boolean): string {
  const directives: string[] = [
    "default-src 'self'",
    "script-src 'self' 'unsafe-inline'",
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data: blob: https://*.stellar.org",
    "font-src 'self'",
    "connect-src 'self' wss: https: https://*.sentry.io",
    "frame-src 'none'",
    "frame-ancestors 'none'",
    "object-src 'none'",
    "base-uri 'self'",
    "form-action 'self'",
  ];

  if (isProd) {
    directives.push("upgrade-insecure-requests");
  }

  return directives.join("; ");
}

function applySecurityHeaders(response: NextResponse, isProd: boolean): void {
  response.headers.set("Content-Security-Policy", buildCsp(isProd));
  response.headers.set("X-Frame-Options", "DENY");
  response.headers.set("X-Content-Type-Options", "nosniff");
  response.headers.set("Referrer-Policy", "strict-origin-when-cross-origin");
  response.headers.set(
    "Permissions-Policy",
    "camera=(), microphone=(), geolocation=()"
  );
}

export default function middleware(request: NextRequest) {
  const isProd = process.env.NODE_ENV === "production";

  if (request.nextUrl.pathname.startsWith("/api/")) {
    return handleApiRequest(request, isProd);
  }

  const response = intlMiddleware(request);
  applySecurityHeaders(response, isProd);
  return response;
}

function handleApiRequest(request: NextRequest, isProd: boolean) {
  const response = NextResponse.next();
  applySecurityHeaders(response, isProd);

  if (request.method === "GET") {
    const csrfToken = generateCsrfToken();
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
    const headerToken = request.headers.get("X-CSRF-Token");

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
