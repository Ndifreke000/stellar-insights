"use client";

/**
 * Issue #2131 — API Documentation Portal: Code Examples sub-page
 *
 * Copy-paste code snippets for TypeScript/JavaScript, Python, and cURL
 * covering the most common API use cases.
 */

import React, { useState } from "react";
import Link from "next/link";
import {
  ArrowLeft,
  FlaskConical,
  Copy,
  Check,
  Code2,
  BookOpen,
} from "lucide-react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type Language = "typescript" | "python" | "curl";

interface CodeExample {
  id: string;
  title: string;
  description: string;
  category: string;
  code: Record<Language, string>;
}

// ---------------------------------------------------------------------------
// Code examples catalogue
// ---------------------------------------------------------------------------

const CODE_EXAMPLES: CodeExample[] = [
  // ── Installation / setup ────────────────────────────────────────────────
  {
    id: "setup",
    title: "SDK Setup & Authentication",
    description: "Install the PayRaider SDK and authenticate with your API token.",
    category: "Getting Started",
    code: {
      typescript: `// Install: npm install @payraider/sdk
import { PayRaiderClient } from "@payraider/sdk";

const client = new PayRaiderClient({
  apiKey: process.env.PAYRAIDER_API_KEY!,
  // Optional: switch to testnet
  // baseUrl: "https://testnet-api.payraider.io",
});

// Verify connectivity
const health = await client.network.health();
console.log("Network status:", health.status); // "healthy"`,
      python: `# Install: pip install payraider
from payraider import PayRaiderClient
import os

client = PayRaiderClient(api_key=os.environ["PAYRAIDER_API_KEY"])

# Verify connectivity
health = client.network.health()
print("Network status:", health["status"])  # "healthy"`,
      curl: `# Set your API key
export PAYRAIDER_API_KEY="your_token_here"
export BASE_URL="https://api.payraider.io"

# Test connectivity
curl -s -H "Authorization: Bearer $PAYRAIDER_API_KEY" \\
  "$BASE_URL/api/v1/network/health" | jq .`,
    },
  },
  // ── List corridors ──────────────────────────────────────────────────────
  {
    id: "list-corridors",
    title: "List Payment Corridors",
    description: "Retrieve corridors sorted by reliability with pagination.",
    category: "Analytics",
    code: {
      typescript: `const { corridors, total } = await client.analytics.listCorridors({
  limit: 10,
  sort: "reliability",
});

for (const corridor of corridors) {
  console.log(
    \`\${corridor.id}: \${(corridor.reliability * 100).toFixed(1)}% reliable, \` +
    \`\${corridor.avg_latency_ms}ms avg latency\`
  );
}

console.log(\`Total corridors: \${total}\`);`,
      python: `result = client.analytics.list_corridors(limit=10, sort="reliability")

for corridor in result["corridors"]:
    print(
        f"{corridor['id']}: {corridor['reliability'] * 100:.1f}% reliable, "
        f"{corridor['avg_latency_ms']}ms avg latency"
    )

print(f"Total corridors: {result['total']}")`,
      curl: `curl -s -H "Authorization: Bearer $PAYRAIDER_API_KEY" \\
  "$BASE_URL/api/v1/analytics/corridors?limit=10&sort=reliability" \\
  | jq '.corridors[] | {id, reliability, avg_latency_ms}'`,
    },
  },
  // ── Get corridor details ────────────────────────────────────────────────
  {
    id: "corridor-detail",
    title: "Get Corridor Details",
    description: "Fetch in-depth analytics for a specific payment corridor.",
    category: "Analytics",
    code: {
      typescript: `const corridor = await client.analytics.getCorridor("USD-PHP");

console.log("Reliability:", corridor.reliability);
console.log("Median fee:", corridor.median_fee_pct + "%");
console.log("7-day volume:", corridor.volume_7d);

// Plot historical success rate
corridor.success_history.forEach((rate, i) => {
  console.log(\`Week \${i + 1}: \${(rate * 100).toFixed(1)}%\`);
});`,
      python: `corridor = client.analytics.get_corridor("USD-PHP")

print("Reliability:", corridor["reliability"])
print("Median fee:", str(corridor["median_fee_pct"]) + "%")
print("7-day volume:", corridor["volume_7d"])

# Print historical success rate
for i, rate in enumerate(corridor["success_history"]):
    print(f"Week {i + 1}: {rate * 100:.1f}%")`,
      curl: `curl -s -H "Authorization: Bearer $PAYRAIDER_API_KEY" \\
  "$BASE_URL/api/v1/analytics/corridors/USD-PHP" | jq .`,
    },
  },
  // ── Optimal payment route ───────────────────────────────────────────────
  {
    id: "route-payment",
    title: "Compute Optimal Payment Route",
    description: "Find the cheapest, fastest route for a cross-border payment.",
    category: "Payments",
    code: {
      typescript: `const route = await client.payments.computeRoute({
  source_asset: "USD",
  dest_asset: "PHP",
  amount: 500,
});

console.log("Route:", route.route.join(" → "));
console.log("Estimated fee:", route.estimated_fee_pct + "%");
console.log("Success probability:", (route.success_probability * 100).toFixed(1) + "%");
console.log("Estimated time:", route.estimated_time_ms + "ms");`,
      python: `route = client.payments.compute_route(
    source_asset="USD",
    dest_asset="PHP",
    amount=500,
)

print("Route:", " → ".join(route["route"]))
print(f"Estimated fee: {route['estimated_fee_pct']}%")
print(f"Success probability: {route['success_probability'] * 100:.1f}%")
print(f"Estimated time: {route['estimated_time_ms']}ms")`,
      curl: `curl -s -X POST \\
  -H "Authorization: Bearer $PAYRAIDER_API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"source_asset":"USD","dest_asset":"PHP","amount":500}' \\
  "$BASE_URL/api/v1/payments/route" | jq .`,
    },
  },
  // ── Snapshot verification ───────────────────────────────────────────────
  {
    id: "snapshot-verify",
    title: "Verify Analytics Snapshot",
    description:
      "Cross-check the on-chain snapshot hash for a given epoch against the backend data. Integrates with the Snapshot Verification Rewards contract (Issue #2136).",
    category: "Analytics",
    code: {
      typescript: `import { createHash } from "crypto";

// 1. Fetch the on-chain snapshot record
const snapshot = await client.analytics.getSnapshot(142);

// 2. Fetch the raw analytics payload from the backend
const payload = await client.analytics.getRawPayload(142);

// 3. Compute SHA-256 of the payload
const computed = createHash("sha256")
  .update(JSON.stringify(payload))
  .digest("hex");

// 4. Compare
const verified = computed === snapshot.hash;
console.log("Snapshot verified:", verified);

// 5. If verified, submit to the rewards contract (optional)
if (verified) {
  await client.contracts.snapshotVerification.verify({
    epoch: 142,
    hash: Buffer.from(computed, "hex"),
  });
  console.log("Reward claim submitted!");
}`,
      python: `import hashlib
import json

# 1. Fetch the on-chain snapshot record
snapshot = client.analytics.get_snapshot(142)

# 2. Fetch the raw analytics payload
payload = client.analytics.get_raw_payload(142)

# 3. Compute SHA-256
computed = hashlib.sha256(
    json.dumps(payload, separators=(",", ":")).encode()
).hexdigest()

# 4. Compare
verified = computed == snapshot["hash"]
print("Snapshot verified:", verified)

# 5. Submit reward claim if verified
if verified:
    client.contracts.snapshot_verification.verify(
        epoch=142, hash=bytes.fromhex(computed)
    )
    print("Reward claim submitted!")`,
      curl: `# 1. Fetch on-chain snapshot for epoch 142
curl -s "$BASE_URL/api/v1/analytics/snapshots/142" | jq .

# 2. Fetch the raw payload (example — endpoint varies)
# curl -s "$BASE_URL/api/v1/analytics/payloads/142" | jq .

# The hash comparison and contract interaction are
# best done via the TypeScript or Python SDK.`,
    },
  },
  // ── Create alert ────────────────────────────────────────────────────────
  {
    id: "create-alert",
    title: "Create an Alert Rule",
    description: "Get notified when a corridor drops below a reliability threshold.",
    category: "Alerts",
    code: {
      typescript: `const alert = await client.alerts.create({
  name: "USD-PHP reliability drop",
  condition: "corridor.id === 'USD-PHP' && reliability < 0.90",
  channels: ["email", "webhook"],
  webhook_url: "https://your-server.example.com/webhooks/payraider",
});

console.log("Alert created with id:", alert.id);`,
      python: `alert = client.alerts.create(
    name="USD-PHP reliability drop",
    condition="corridor.id === 'USD-PHP' && reliability < 0.90",
    channels=["email", "webhook"],
    webhook_url="https://your-server.example.com/webhooks/payraider",
)

print("Alert created with id:", alert["id"])`,
      curl: `curl -s -X POST \\
  -H "Authorization: Bearer $PAYRAIDER_API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "name": "USD-PHP reliability drop",
    "condition": "reliability < 0.90",
    "channels": ["email"]
  }' \\
  "$BASE_URL/api/v1/alerts" | jq .`,
    },
  },
  // ── WebSocket streaming ─────────────────────────────────────────────────
  {
    id: "websocket",
    title: "Real-time Corridor Streaming",
    description: "Subscribe to live corridor updates over WebSocket.",
    category: "Real-time",
    code: {
      typescript: `const ws = client.stream.corridors({
  corridors: ["USD-PHP", "EUR-MXN"],
  onUpdate: (update) => {
    console.log(\`[\${update.id}] reliability=\${update.reliability}\`);
  },
  onError: (err) => console.error("Stream error:", err),
});

// Later — clean up
ws.close();`,
      python: `import asyncio

async def stream_corridors():
    async with client.stream.corridors(
        corridors=["USD-PHP", "EUR-MXN"]
    ) as stream:
        async for update in stream:
            print(
                f"[{update['id']}] reliability={update['reliability']}"
            )

asyncio.run(stream_corridors())`,
      curl: `# WebSocket connections via cURL (wscat recommended instead)
# Install: npm i -g wscat
wscat -H "Authorization: Bearer $PAYRAIDER_API_KEY" \\
  -c "wss://api.payraider.io/ws/corridors?ids=USD-PHP,EUR-MXN"`,
    },
  },
];

const CATEGORIES = ["All", ...Array.from(new Set(CODE_EXAMPLES.map((e) => e.category)))];

// ---------------------------------------------------------------------------
// Syntax highlighting helpers (lightweight, no external dep)
// ---------------------------------------------------------------------------

const LANGUAGE_LABELS: Record<Language, string> = {
  typescript: "TypeScript",
  python: "Python",
  curl: "cURL",
};

// ---------------------------------------------------------------------------
// Copy button
// ---------------------------------------------------------------------------

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard not available in all envs
    }
  };

  return (
    <button
      type="button"
      aria-label={copied ? "Copied" : "Copy code"}
      onClick={handleCopy}
      className="flex items-center gap-1.5 text-xs text-slate-400 hover:text-slate-200 bg-slate-700/60 hover:bg-slate-600 px-2.5 py-1 rounded transition-colors"
    >
      {copied ? (
        <>
          <Check className="w-3.5 h-3.5 text-emerald-400" />
          Copied
        </>
      ) : (
        <>
          <Copy className="w-3.5 h-3.5" />
          Copy
        </>
      )}
    </button>
  );
}

// ---------------------------------------------------------------------------
// Individual example card
// ---------------------------------------------------------------------------

function ExampleCard({ example }: { example: CodeExample }) {
  const [lang, setLang] = useState<Language>("typescript");

  return (
    <article
      className="rounded-xl border border-slate-700 bg-slate-800/50 overflow-hidden"
      aria-labelledby={`example-${example.id}-title`}
    >
      {/* Card header */}
      <header className="px-5 pt-4 pb-3 border-b border-slate-700">
        <h3 id={`example-${example.id}-title`} className="text-base font-semibold text-slate-100">
          {example.title}
        </h3>
        <p className="text-sm text-slate-400 mt-0.5">{example.description}</p>
      </header>

      {/* Language tabs */}
      <div className="flex items-center gap-1 px-4 pt-3" role="tablist" aria-label={`Language selection for ${example.title}`}>
        {(Object.keys(LANGUAGE_LABELS) as Language[]).map((l) => (
          <button
            key={l}
            type="button"
            role="tab"
            aria-selected={lang === l}
            aria-controls={`code-${example.id}-${l}`}
            onClick={() => setLang(l)}
            className={`text-xs px-3 py-1.5 rounded-t-md border-b-2 transition-colors ${
              lang === l
                ? "border-indigo-500 text-indigo-300 bg-slate-900/60"
                : "border-transparent text-slate-500 hover:text-slate-300"
            }`}
          >
            {LANGUAGE_LABELS[l]}
          </button>
        ))}
        <div className="ml-auto">
          <CopyButton text={example.code[lang]} />
        </div>
      </div>

      {/* Code block */}
      <div
        id={`code-${example.id}-${lang}`}
        role="tabpanel"
        aria-label={`${LANGUAGE_LABELS[lang]} code for ${example.title}`}
      >
        <pre className="px-5 py-4 bg-slate-900 text-xs font-mono text-slate-300 overflow-x-auto leading-relaxed max-h-80">
          <code>{example.code[lang]}</code>
        </pre>
      </div>
    </article>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function CodeExamples() {
  const [activeCategory, setActiveCategory] = useState("All");

  const filtered = CODE_EXAMPLES.filter(
    (e) => activeCategory === "All" || e.category === activeCategory
  );

  return (
    <div className="min-h-screen bg-slate-900 text-slate-100">
      {/* Header */}
      <header className="border-b border-slate-700 bg-slate-900/80 backdrop-blur-sm sticky top-0 z-10">
        <div className="max-w-5xl mx-auto px-4 py-3 flex items-center gap-4 flex-wrap">
          <Link
            href="/api-docs"
            className="flex items-center gap-1.5 text-sm text-slate-400 hover:text-slate-200 transition-colors"
          >
            <ArrowLeft className="w-4 h-4" />
            Back to Docs
          </Link>
          <div className="flex items-center gap-2">
            <Code2 className="w-5 h-5 text-emerald-400" aria-hidden="true" />
            <h1 className="text-base font-bold">Code Examples</h1>
          </div>
          <Link
            href="/api-docs/playground"
            className="ml-auto flex items-center gap-1.5 text-sm bg-indigo-600 hover:bg-indigo-500 text-white px-3 py-1.5 rounded-lg transition-colors"
          >
            <FlaskConical className="w-4 h-4" />
            Playground
          </Link>
        </div>
      </header>

      <main className="max-w-5xl mx-auto px-4 py-8 space-y-8">
        {/* Intro */}
        <section aria-label="Introduction">
          <h2 className="text-2xl font-bold mb-2">Copy-Paste Examples</h2>
          <p className="text-slate-300 max-w-2xl">
            Ready-to-run snippets for TypeScript, Python, and cURL. Switch languages with the tab selectors on each card.
            Replace <code className="font-mono text-indigo-300">PAYRAIDER_API_KEY</code> with your token from the{" "}
            <Link href="/settings" className="text-indigo-400 hover:text-indigo-300 underline">
              settings page
            </Link>
            .
          </p>
        </section>

        {/* SDK install banner */}
        <div className="rounded-xl border border-indigo-700/40 bg-indigo-900/20 p-4">
          <div className="flex items-start gap-3">
            <BookOpen className="w-5 h-5 text-indigo-400 mt-0.5 shrink-0" aria-hidden="true" />
            <div>
              <p className="text-sm font-medium text-slate-200 mb-1">Install the SDK</p>
              <div className="flex flex-wrap gap-3 text-xs font-mono text-indigo-300">
                <code>npm install @payraider/sdk</code>
                <span className="text-slate-600">|</span>
                <code>pip install payraider</code>
              </div>
            </div>
          </div>
        </div>

        {/* Category filter */}
        <div
          className="flex flex-wrap gap-2"
          role="tablist"
          aria-label="Filter examples by category"
        >
          {CATEGORIES.map((cat) => (
            <button
              key={cat}
              type="button"
              role="tab"
              aria-selected={activeCategory === cat}
              onClick={() => setActiveCategory(cat)}
              className={`text-sm px-3 py-1.5 rounded-lg border transition-colors ${
                activeCategory === cat
                  ? "bg-indigo-600 border-indigo-600 text-white"
                  : "bg-slate-800 border-slate-700 text-slate-400 hover:border-slate-500 hover:text-slate-200"
              }`}
            >
              {cat}
            </button>
          ))}
        </div>

        {/* Examples list */}
        <div className="space-y-5" aria-label="Code examples">
          {filtered.map((example) => (
            <ExampleCard key={example.id} example={example} />
          ))}
        </div>
      </main>
    </div>
  );
}
