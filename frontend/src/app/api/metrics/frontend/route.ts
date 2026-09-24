import { NextResponse } from "next/server";
import { FRONTEND_METRICS_ENDPOINT, type Metric, type AppError } from "@/lib/monitoring";

export async function POST(request: Request) {
  const body = await request.json().catch(() => null);

  if (!body || typeof body !== "object") {
    return NextResponse.json({ error: "Invalid payload" }, { status: 400 });
  }

  const { metrics, errors } = body as {
    metrics?: Metric[];
    errors?: AppError[];
  };

  if (process.env.NEXT_PUBLIC_API_URL) {
    try {
      await fetch(FRONTEND_METRICS_ENDPOINT, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ metrics, errors }),
      });
    } catch {
      // Non-fatal: backend may not be running in dev
    }
  }

  return NextResponse.json({ ok: true });
}
