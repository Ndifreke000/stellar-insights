import React from "react";
import { redirect } from "next/navigation";
import { headers } from "next/headers";
import { Activity } from "lucide-react";
import { JobMonitoringDashboard } from "@/components/JobMonitoringDashboard";

/**
 * Internal monitoring dashboard — #1828
 *
 * Access control: This is a server component. Before rendering anything it
 * checks that the request carries a valid internal access token.  The token
 * is compared against the INTERNAL_SECRET environment variable (server-side
 * only — never a NEXT_PUBLIC_ var so it is never shipped to the browser).
 *
 * Two ways to pass the token:
 *   1. HTTP header:  X-Internal-Token: <secret>
 *   2. Cookie:       internal_token=<secret>
 *
 * If neither is present or the value doesn't match, the visitor is redirected
 * to "/" instead of receiving a 403 — this avoids leaking that the route
 * exists at all.
 *
 * In development (NODE_ENV !== "production") the gate is skipped when
 * INTERNAL_SECRET is not set, so local devs can still iterate without
 * configuring the secret.
 */
function isAuthorised(headersList: Headers): boolean {
  const secret = process.env.INTERNAL_SECRET;

  // Dev shortcut: if no secret is configured, allow access in development only
  if (!secret) {
    return process.env.NODE_ENV !== "production";
  }

  // Check X-Internal-Token header (useful for curl / scripted access)
  const headerToken = headersList.get("x-internal-token");
  if (headerToken === secret) return true;

  // Check cookie (useful for browser access after initial auth)
  const cookieHeader = headersList.get("cookie") ?? "";
  const match = cookieHeader
    .split(";")
    .map((c) => c.trim())
    .find((c) => c.startsWith("internal_token="));
  if (match) {
    const cookieToken = match.slice("internal_token=".length);
    if (cookieToken === secret) return true;
  }

  return false;
}

export const metadata = {
  title: "Internal Monitoring — PayRaider",
  // Prevent search engines from indexing this internal page
  robots: "noindex, nofollow",
};

export default async function MonitoringDashboard() {
  const headersList = await headers();

  if (!isAuthorised(headersList)) {
    redirect("/");
  }

  return (
    <div className="p-4 md:p-8 max-w-7xl mx-auto">
      <header className="mb-8">
        <h1 className="text-3xl font-bold flex items-center gap-2">
          <Activity className="text-blue-500" aria-hidden="true" />
          Internal Monitoring
        </h1>
        <p className="text-muted-foreground mt-1">
          Real-time job execution status and system health. This page is
          internal-only and not publicly accessible.
        </p>
      </header>

      {/* JobMonitoringDashboard fetches /api/v1/jobs/status from the real backend */}
      <JobMonitoringDashboard />
    </div>
  );
}
