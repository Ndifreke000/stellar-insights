/**
 * Bundle Budget Check — verifies that the built JS bundles stay within budget.
 *
 * Run after `next build` to enforce the < 200KB initial load target.
 * In CI (process.env.CI === 'true'), budget violations cause a non-zero exit.
 *
 * Usage: node scripts/bundle-budget-check.mjs
 */
import { readdirSync, statSync, readFileSync } from 'fs';
import { join } from 'path';

const BUILD_DIR = '.next/static';
const BUDGETS = {
  // Per-asset budget (raw, pre-gzip): 500 KB
  perAsset: 500 * 1024,
  // Total initial load budget (raw, pre-gzip): 600 KB (~200 KB gzipped)
  totalInitial: 600 * 1024,
  // Per-chunk budget for named chunks
  namedChunks: {
    vendor: 300 * 1024,
    charts: 200 * 1024,
    animation: 100 * 1024,
  },
};

const isCI = process.env.CI === 'true';
const violations = [];
const warnings = [];

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)}MB`;
}

function walkDir(dir) {
  const results = [];
  for (const entry of readdirSync(dir)) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);
    if (stat.isDirectory()) {
      results.push(...walkDir(fullPath));
    } else {
      results.push({ path: fullPath, size: stat.size });
    }
  }
  return results;
}

function getGzippedSize(filePath) {
  // Estimate: gzip typically achieves ~60-70% compression on JS
  // Actual gzip measurement would require zlib, but this gives a rough estimate
  try {
    const content = readFileSync(filePath, 'utf-8');
    return Math.round(content.length * 0.35); // Conservative estimate
  } catch {
    return null;
  }
}

try {
  const staticDir = join(process.cwd(), BUILD_DIR);
  const files = walkDir(staticDir);

  // Categorize files
  const chunks = files.filter((f) => f.path.includes('/chunks/'));
  const pages = files.filter((f) => f.path.includes('/pages/'));
  const cssFiles = files.filter((f) => f.path.endsWith('.css'));
  const jsFiles = files.filter((f) => f.path.endsWith('.js'));

  // Check per-asset budget
  for (const file of jsFiles) {
    if (file.size > BUDGETS.perAsset) {
      violations.push(
        `ASSET OVER BUDGET: ${file.path.replace(process.cwd(), '')} is ${formatBytes(file.size)} (limit: ${formatBytes(BUDGETS.perAsset)})`
      );
    }
  }

  // Check named chunks
  for (const [chunkName, budget] of Object.entries(BUDGETS.namedChunks)) {
    const chunkFile = chunks.find((c) => c.path.includes(`${chunkName}.`));
    if (chunkFile && chunkFile.size > budget) {
      violations.push(
        `NAMED CHUNK OVER BUDGET: ${chunkName} is ${formatBytes(chunkFile.size)} (limit: ${formatBytes(budget)})`
      );
    }
  }

  // Calculate total initial load (vendor + common + page-level JS)
  const initialChunks = chunks.filter(
    (c) => c.path.includes('vendor.') || c.path.includes('common.') || c.path.includes('framework.')
  );
  const totalInitial = initialChunks.reduce((sum, f) => sum + f.size, 0);

  if (totalInitial > BUDGETS.totalInitial) {
    violations.push(
      `TOTAL INITIAL LOAD OVER BUDGET: ${formatBytes(totalInitial)} (limit: ${formatBytes(BUDGETS.totalInitial)})`
    );
  }

  // Report
  console.log('\n📦 Bundle Budget Report');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log(`Total JS files: ${jsFiles.length}`);
  console.log(`Total CSS files: ${cssFiles.length}`);
  console.log(`Initial chunks total: ${formatBytes(totalInitial)} (budget: ${formatBytes(BUDGETS.totalInitial)})`);

  // List named chunks
  for (const [chunkName] of Object.entries(BUDGETS.namedChunks)) {
    const chunkFile = chunks.find((c) => c.path.includes(`${chunkName}.`));
    if (chunkFile) {
      const gzipped = getGzippedSize(chunkFile.path);
      console.log(`  ${chunkName}: ${formatBytes(chunkFile.size)}${gzipped ? ` (~${formatBytes(gzipped)} gzipped)` : ''}`);
    }
  }

  // List largest page bundles
  const sortedPages = pages.sort((a, b) => b.size - a.size).slice(0, 5);
  if (sortedPages.length > 0) {
    console.log('\nLargest page bundles:');
    for (const page of sortedPages) {
      console.log(`  ${page.path.replace(process.cwd(), '')}: ${formatBytes(page.size)}`);
    }
  }

  if (violations.length > 0) {
    console.log('\n❌ BUDGET VIOLATIONS:');
    for (const v of violations) {
      console.log(`  • ${v}`);
    }
    if (isCI) {
      console.log('\n❌ Bundle budget check FAILED in CI mode.');
      process.exit(1);
    } else {
      console.log('\n⚠️  Running locally — violations are warnings only.');
    }
  } else {
    console.log('\n✅ All bundles within budget.');
  }
} catch (err) {
  if (err.code === 'ENOENT') {
    console.error('❌ Build directory not found. Run `next build` first.');
    process.exit(1);
  }
  throw err;
}
