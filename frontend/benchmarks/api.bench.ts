import autocannon from "autocannon";
import fs from "node:fs";
import path from "node:path";

type BenchmarkTarget = {
  name: string;
  url: string;
};

type BenchmarkRun = {
  name: string;
  requestsPerSec: number;
  latencyAvgMs: number;
  latencyP99Ms: number;
  throughputMBps: number;
  errors: number;
  non2xx: number;
  timeouts: number;
};

const FRONTEND_BASE_URL = process.env.BENCHMARK_URL?.trim()
  || process.env.FRONTEND_BENCHMARK_URL?.trim()
  || "http://localhost:3000";
const BACKEND_BASE_URL = process.env.BACKEND_BENCHMARK_URL?.trim()
  || process.env.BACKEND_URL?.trim()
  || "http://localhost:8080";

const DURATION_SECONDS = Number(process.env.BENCHMARK_DURATION ?? 10);
const CONNECTIONS = Number(process.env.BENCHMARK_CONNECTIONS ?? 10);
const SAVE_BASELINE = process.env.SAVE_BASELINE === "true";
const baselinePath = path.resolve("benchmarks/baseline.json");

const targets: BenchmarkTarget[] = [
  { name: "Dashboard API", url: `${FRONTEND_BASE_URL}/api/dashboard` },
  { name: "Backend Corridors", url: `${BACKEND_BASE_URL}/api/corridors` },
  { name: "Backend Latest Ledger", url: `${BACKEND_BASE_URL}/api/rpc/ledger/latest` },
  { name: "Backend Payments", url: `${BACKEND_BASE_URL}/api/rpc/payments?limit=50` },
];

function runAutocannon(url: string): Promise<autocannon.Result> {
  return new Promise((resolve, reject) => {
    autocannon(
      {
        url,
        duration: DURATION_SECONDS,
        connections: CONNECTIONS,
        pipelining: 1,
      },
      (error, result) => {
        if (error) {
          reject(error);
          return;
        }

        resolve(result);
      },
    );
  });
}

async function benchmarkEndpoint(target: BenchmarkTarget): Promise<BenchmarkRun | null> {
  console.log(`\nBenchmarking: ${target.name}`);
  console.log(`URL: ${target.url}`);

  try {
    const result = await runAutocannon(target.url);

    const run: BenchmarkRun = {
      name: target.name,
      requestsPerSec: result.requests.average,
      latencyAvgMs: result.latency.average,
      latencyP99Ms: result.latency.p99,
      throughputMBps: result.throughput.average / 1e6,
      errors: result.errors,
      non2xx: result.non2xx,
      timeouts: result.timeouts,
    };

    console.table({
      "Requests/sec (avg)": run.requestsPerSec.toFixed(2),
      "Latency avg (ms)": run.latencyAvgMs.toFixed(2),
      "Latency p99 (ms)": run.latencyP99Ms.toFixed(2),
      "Throughput (MB/s)": run.throughputMBps.toFixed(2),
      Errors: run.errors,
      "Non-2xx": run.non2xx,
      Timeouts: run.timeouts,
    });

    return run;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`Benchmark failed for ${target.name}: ${message}`);
    return null;
  }
}

function loadBaseline(): Record<string, number> | null {
  if (!fs.existsSync(baselinePath)) {
    return null;
  }

  const content = fs.readFileSync(baselinePath, "utf-8");
  return JSON.parse(content) as Record<string, number>;
}

function saveBaseline(results: Record<string, number>): void {
  const directory = path.dirname(baselinePath);
  if (!fs.existsSync(directory)) {
    fs.mkdirSync(directory, { recursive: true });
  }

  fs.writeFileSync(baselinePath, JSON.stringify(results, null, 2));
}

function printRegressionCheck(current: Record<string, number>, baseline: Record<string, number>): void {
  console.log("\nRegression check vs baseline:");

  for (const [name, rps] of Object.entries(current)) {
    const base = baseline[name];
    if (!base || base <= 0) {
      console.log(`- ${name}: no baseline value available`);
      continue;
    }

    const delta = ((rps - base) / base) * 100;
    const flag = delta < -10 ? "REGRESSION" : "OK";
    console.log(`- [${flag}] ${name}: ${rps.toFixed(2)} req/s (${delta.toFixed(1)}% vs baseline)`);
  }
}

async function runBenchmarks(): Promise<void> {
  console.log("Starting API performance benchmarks");
  console.log("=".repeat(50));
  console.log(`Frontend base URL: ${FRONTEND_BASE_URL}`);
  console.log(`Backend base URL: ${BACKEND_BASE_URL}`);
  console.log(`Duration: ${DURATION_SECONDS}s, Connections: ${CONNECTIONS}`);

  const results: Record<string, number> = {};

  for (const target of targets) {
    const run = await benchmarkEndpoint(target);
    if (run) {
      results[target.name] = run.requestsPerSec;
    }
  }

  if (Object.keys(results).length === 0) {
    throw new Error("No benchmark runs completed successfully.");
  }

  console.log("\nSummary (req/sec):");
  console.table(results);

  const baseline = loadBaseline();

  if (SAVE_BASELINE || !baseline) {
    saveBaseline(results);
    console.log(`\nBaseline saved to ${baselinePath}`);
    return;
  }

  printRegressionCheck(results, baseline);
}

runBenchmarks().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`Benchmark run failed: ${message}`);
  process.exitCode = 1;
});
