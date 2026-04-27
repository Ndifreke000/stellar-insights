import { NextResponse } from "next/server";
import type { Metric, AppError } from "@/lib/monitoring";

export async function POST(request: Request) {
  const body = await request.json().catch(() => null);

  if (!body || typeof body !== "object") {
    return NextResponse.json({ error: "Invalid payload" }, { status: 400 });
  }

  const { metrics, errors } = body as {
    metrics?: Metric[];
    errors?: AppError[];
  };

  const backendUrl = process.env.NEXT_PUBLIC_API_URL;
  if (backendUrl) {
    try {
      await fetch(`${backendUrl}/api/metrics/frontend`, {
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
