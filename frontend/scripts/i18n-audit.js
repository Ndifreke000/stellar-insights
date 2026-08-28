#!/usr/bin/env node
/**
 * Translation completeness audit (stellar-insights#1872).
 *
 * Enumerates every supported locale from src/i18n/routing.ts, flattens each
 * locale's messages/<locale>.json into dot-separated keys, and diffs every
 * locale against the default locale to report missing keys, extra keys,
 * empty/null values, and a rough string-length delta (a proxy for layout
 * risk from longer/shorter translated strings).
 *
 * Dependency-free: uses only Node's built-in `fs`/`path` so it can run with
 * plain `node scripts/i18n-audit.js` without installing anything.
 */

const fs = require("fs");
const path = require("path");

const FRONTEND_ROOT = path.resolve(__dirname, "..");
const MESSAGES_DIR = path.join(FRONTEND_ROOT, "messages");
const ROUTING_FILE = path.join(FRONTEND_ROOT, "src", "i18n", "routing.ts");

function getSupportedLocales() {
  const source = fs.readFileSync(ROUTING_FILE, "utf8");

  const localesMatch = source.match(/locales:\s*\[([^\]]*)\]/);
  if (!localesMatch) {
    throw new Error(`Could not find "locales" array in ${ROUTING_FILE}`);
  }
  const locales = [...localesMatch[1].matchAll(/["']([\w-]+)["']/g)].map(
    (m) => m[1]
  );

  const defaultMatch = source.match(/defaultLocale:\s*["']([\w-]+)["']/);
  if (!defaultMatch) {
    throw new Error(`Could not find "defaultLocale" in ${ROUTING_FILE}`);
  }

  return { locales, defaultLocale: defaultMatch[1] };
}

function loadMessages(locale) {
  const file = path.join(MESSAGES_DIR, `${locale}.json`);
  if (!fs.existsSync(file)) {
    return { file, missing: true, messages: {} };
  }
  return { file, missing: false, messages: JSON.parse(fs.readFileSync(file, "utf8")) };
}

/** Flatten a nested object into { "a.b.c": value } entries. */
function flatten(obj, prefix = "", out = {}) {
  for (const [key, value] of Object.entries(obj)) {
    const full = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === "object" && !Array.isArray(value)) {
      flatten(value, full, out);
    } else {
      out[full] = value;
    }
  }
  return out;
}

function stringLength(value) {
  return typeof value === "string" ? value.length : null;
}

function main() {
  const { locales, defaultLocale } = getSupportedLocales();

  console.log(`Supported locales (${locales.length}): ${locales.join(", ")}`);
  console.log(`Default locale: ${defaultLocale}\n`);

  const loaded = Object.fromEntries(locales.map((l) => [l, loadMessages(l)]));

  const missingFiles = locales.filter((l) => loaded[l].missing);
  if (missingFiles.length > 0) {
    console.log(`MISSING message files for: ${missingFiles.join(", ")}\n`);
  }

  const flattened = Object.fromEntries(
    locales.map((l) => [l, flatten(loaded[l].messages)])
  );

  const defaultKeys = new Set(Object.keys(flattened[defaultLocale] || {}));
  let hasGaps = false;
  const summary = [];

  for (const locale of locales) {
    if (locale === defaultLocale) continue;
    const keys = new Set(Object.keys(flattened[locale]));

    const missingKeys = [...defaultKeys].filter((k) => !keys.has(k));
    const extraKeys = [...keys].filter((k) => !defaultKeys.has(k));

    const emptyOrNull = Object.entries(flattened[locale]).filter(
      ([, v]) => v === "" || v === null || v === undefined
    );

    const lengthDeltas = [...defaultKeys]
      .filter((k) => keys.has(k))
      .map((k) => {
        const a = stringLength(flattened[defaultLocale][k]);
        const b = stringLength(flattened[locale][k]);
        if (a === null || b === null || a === 0) return null;
        return { key: k, defaultLen: a, localeLen: b, ratio: b / a };
      })
      .filter(Boolean);

    const bigGrowth = lengthDeltas.filter((d) => d.ratio >= 1.8);
    const bigShrink = lengthDeltas.filter((d) => d.ratio <= 0.5);

    if (missingKeys.length || extraKeys.length || emptyOrNull.length) {
      hasGaps = true;
    }

    summary.push({
      locale,
      totalKeys: keys.size,
      missingKeys,
      extraKeys,
      emptyOrNull: emptyOrNull.map(([k]) => k),
      bigGrowth,
      bigShrink,
    });
  }

  for (const s of summary) {
    console.log(`--- ${s.locale} (default: ${defaultLocale}) ---`);
    console.log(`  keys: ${s.totalKeys} / ${defaultKeys.size}`);
    console.log(
      `  missing keys (${s.missingKeys.length}): ${
        s.missingKeys.slice(0, 20).join(", ") || "none"
      }${s.missingKeys.length > 20 ? ", ..." : ""}`
    );
    console.log(
      `  extra keys (${s.extraKeys.length}): ${
        s.extraKeys.slice(0, 20).join(", ") || "none"
      }${s.extraKeys.length > 20 ? ", ..." : ""}`
    );
    console.log(
      `  empty/null values (${s.emptyOrNull.length}): ${
        s.emptyOrNull.slice(0, 20).join(", ") || "none"
      }${s.emptyOrNull.length > 20 ? ", ..." : ""}`
    );
    console.log(
      `  strings >=1.8x longer than ${defaultLocale} (layout risk): ${s.bigGrowth.length}`
    );
    for (const d of s.bigGrowth.slice(0, 10)) {
      console.log(`    ${d.key}: ${d.defaultLen} -> ${d.localeLen} chars`);
    }
    console.log(
      `  strings <=0.5x shorter than ${defaultLocale} (layout risk): ${s.bigShrink.length}`
    );
    for (const d of s.bigShrink.slice(0, 10)) {
      console.log(`    ${d.key}: ${d.defaultLen} -> ${d.localeLen} chars`);
    }
    console.log("");
  }

  if (missingFiles.length > 0 || hasGaps) {
    console.log("RESULT: translation gaps found (see detail above).");
    process.exitCode = 1;
  } else {
    console.log("RESULT: all locales have complete key coverage vs default locale.");
  }
}

main();
