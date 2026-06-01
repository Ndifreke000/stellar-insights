import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import {
  buildTargets,
  evaluateRegression,
  isHealthyRun,
  loadBaseline,
  saveBaseline,
} from "./api.bench.mjs";

test("buildTargets uses provided frontend/backend URLs", () => {
  const targets = buildTargets("http://localhost:3100", "http://localhost:8181");
  assert.equal(targets.length, 4);
  assert.equal(targets[0].url, "http://localhost:3100/api/dashboard");
  assert.equal(targets[1].url, "http://localhost:8181/api/corridors");
  assert.equal(targets[2].url, "http://localhost:8181/api/rpc/ledger/latest");
  assert.equal(targets[3].url, "http://localhost:8181/api/rpc/payments?limit=50");
});

test("evaluateRegression flags >10% drop as regression", () => {
  const regression = evaluateRegression(89, 100);
  assert.equal(regression.status, "REGRESSION");
  assert.equal(regression.deltaPct, -11);

  const healthy = evaluateRegression(95, 100);
  assert.equal(healthy.status, "OK");
  assert.equal(healthy.deltaPct, -5);
});

test("evaluateRegression handles missing baseline", () => {
  const noBaseline = evaluateRegression(100, 0);
  assert.equal(noBaseline.status, "MISSING_BASELINE");
  assert.equal(noBaseline.deltaPct, null);
});

test("isHealthyRun validates request and error conditions", () => {
  assert.equal(
    isHealthyRun({
      name: "ok",
      requestsPerSec: 100,
      latencyAvgMs: 10,
      latencyP99Ms: 20,
      throughputMBps: 1.2,
      errors: 0,
      non2xx: 0,
      timeouts: 0,
    }),
    true,
  );

  assert.equal(
    isHealthyRun({
      name: "bad",
      requestsPerSec: 0,
      latencyAvgMs: 0,
      latencyP99Ms: 0,
      throughputMBps: 0,
      errors: 3,
      non2xx: 0,
      timeouts: 0,
    }),
    false,
  );
});

test("saveBaseline/loadBaseline round-trip", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "stellar-bench-"));
  const baselinePath = path.join(tempDir, "baseline.json");
  const results = { "Dashboard API": 123.45, "Backend Corridors": 88.12 };

  saveBaseline(results, baselinePath);
  const loaded = loadBaseline(baselinePath);

  assert.deepEqual(loaded, results);
});
