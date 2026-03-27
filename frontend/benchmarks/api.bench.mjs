import autocannon from "autocannon";
import fs from "node:fs";
import path from "node:path";

export const DEFAULT_BASELINE_PATH = path.resolve("benchmarks/baseline.json");

/**
 * @typedef {{name: string, url: string}} BenchmarkTarget
 * @typedef {{name: string, requestsPerSec: number, latencyAvgMs: number, latencyP99Ms: number, throughputMBps: number, errors: number, non2xx: number, timeouts: number}} BenchmarkRun
 */

/**
 * @param {string | undefined} frontendBaseUrl
 * @param {string | undefined} backendBaseUrl
 * @returns {BenchmarkTarget[]}
 */
export function buildTargets(frontendBaseUrl, backendBaseUrl) {
  const frontend = frontendBaseUrl?.trim() || "http://localhost:3000";
  const backend = backendBaseUrl?.trim() || "http://localhost:8080";

  return [
    { name: "Dashboard API", url: `${frontend}/api/dashboard` },
    { name: "Backend Corridors", url: `${backend}/api/corridors` },
    { name: "Backend Latest Ledger", url: `${backend}/api/rpc/ledger/latest` },
    { name: "Backend Payments", url: `${backend}/api/rpc/payments?limit=50` },
  ];
}

/**
 * @param {string} url
 * @param {number} durationSeconds
 * @param {number} connections
 * @returns {Promise<autocannon.Result>}
 */
export function runAutocannon(url, durationSeconds, connections) {
  return new Promise((resolve, reject) => {
    autocannon(
      {
        url,
        duration: durationSeconds,
        connections,
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

/**
 * @param {BenchmarkTarget} target
 * @param {number} durationSeconds
 * @param {number} connections
 * @returns {Promise<BenchmarkRun | null>}
 */
export async function benchmarkEndpoint(target, durationSeconds, connections) {
  console.log(`\nBenchmarking: ${target.name}`);
  console.log(`URL: ${target.url}`);

  try {
    const result = await runAutocannon(target.url, durationSeconds, connections);

    const run = {
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

    if (!isHealthyRun(run)) {
      console.error(
        `Endpoint ${target.name} produced unhealthy results (req/s=${run.requestsPerSec.toFixed(2)}, errors=${run.errors}, non2xx=${run.non2xx}, timeouts=${run.timeouts}).`,
      );
      return null;
    }

    return run;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`Benchmark failed for ${target.name}: ${message}`);
    return null;
  }
}

/**
 * @param {string} baselinePath
 * @returns {Record<string, number> | null}
 */
export function loadBaseline(baselinePath = DEFAULT_BASELINE_PATH) {
  if (!fs.existsSync(baselinePath)) {
    return null;
  }

  const content = fs.readFileSync(baselinePath, "utf-8");
  return JSON.parse(content);
}

/**
 * @param {Record<string, number>} results
 * @param {string} baselinePath
 */
export function saveBaseline(results, baselinePath = DEFAULT_BASELINE_PATH) {
  const directory = path.dirname(baselinePath);
  if (!fs.existsSync(directory)) {
    fs.mkdirSync(directory, { recursive: true });
  }

  fs.writeFileSync(baselinePath, JSON.stringify(results, null, 2));
}

/**
 * @param {number} currentRps
 * @param {number} baselineRps
 * @returns {{deltaPct: number | null, status: "OK" | "REGRESSION" | "MISSING_BASELINE"}}
 */
export function evaluateRegression(currentRps, baselineRps) {
  if (!baselineRps || baselineRps <= 0) {
    return { deltaPct: null, status: "MISSING_BASELINE" };
  }

  const deltaPct = ((currentRps - baselineRps) / baselineRps) * 100;
  return { deltaPct, status: deltaPct < -10 ? "REGRESSION" : "OK" };
}

/**
 * @param {BenchmarkRun} run
 * @returns {boolean}
 */
export function isHealthyRun(run) {
  return run.requestsPerSec > 0 && run.errors === 0 && run.non2xx === 0 && run.timeouts === 0;
}

/**
 * @param {Record<string, number>} current
 * @param {Record<string, number>} baseline
 */
export function printRegressionCheck(current, baseline) {
  console.log("\nRegression check vs baseline:");

  for (const [name, rps] of Object.entries(current)) {
    const evaluation = evaluateRegression(rps, baseline[name]);
    if (evaluation.status === "MISSING_BASELINE") {
      console.log(`- ${name}: no baseline value available`);
      continue;
    }

    console.log(
      `- [${evaluation.status}] ${name}: ${rps.toFixed(2)} req/s (${evaluation.deltaPct.toFixed(1)}% vs baseline)`,
    );
  }
}

export async function runBenchmarks() {
  const frontendBaseUrl = process.env.BENCHMARK_URL?.trim() || process.env.FRONTEND_BENCHMARK_URL?.trim();
  const backendBaseUrl = process.env.BACKEND_BENCHMARK_URL?.trim() || process.env.BACKEND_URL?.trim();
  const durationSeconds = Number(process.env.BENCHMARK_DURATION ?? 10);
  const connections = Number(process.env.BENCHMARK_CONNECTIONS ?? 10);
  const saveBaselineFlag = process.env.SAVE_BASELINE === "true";

  const targets = buildTargets(frontendBaseUrl, backendBaseUrl);
  const frontend = frontendBaseUrl || "http://localhost:3000";
  const backend = backendBaseUrl || "http://localhost:8080";

  console.log("Starting API performance benchmarks");
  console.log("=".repeat(50));
  console.log(`Frontend base URL: ${frontend}`);
  console.log(`Backend base URL: ${backend}`);
  console.log(`Duration: ${durationSeconds}s, Connections: ${connections}`);

  /** @type {Record<string, number>} */
  const results = {};
  for (const target of targets) {
    const run = await benchmarkEndpoint(target, durationSeconds, connections);
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
  if (saveBaselineFlag || !baseline) {
    saveBaseline(results);
    console.log(`\nBaseline saved to ${DEFAULT_BASELINE_PATH}`);
    return;
  }

  printRegressionCheck(results, baseline);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  runBenchmarks().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`Benchmark run failed: ${message}`);
    process.exitCode = 1;
  });
}
