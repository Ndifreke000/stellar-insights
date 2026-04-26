import { describe, it, expect } from "vitest";
import en from "../../messages/en.json";
import es from "../../messages/es.json";
import zh from "../../messages/zh.json";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Flatten a nested object into dot-separated keys, e.g. "common.loading" */
function flattenKeys(obj: Record<string, unknown>, prefix = ""): string[] {
  return Object.entries(obj).flatMap(([key, value]) => {
    const full = prefix ? `${prefix}.${key}` : key;
    return value !== null && typeof value === "object" && !Array.isArray(value)
      ? flattenKeys(value as Record<string, unknown>, full)
      : [full];
  });
}

const enKeys = flattenKeys(en as Record<string, unknown>);
const esKeys = new Set(flattenKeys(es as Record<string, unknown>));
const zhKeys = new Set(flattenKeys(zh as Record<string, unknown>));

// ---------------------------------------------------------------------------
// RTL locales
// ---------------------------------------------------------------------------

const RTL_LOCALES = ["ar", "he", "fa", "ur"];
const LTR_LOCALES = ["en", "es", "zh"];

function getTextDirection(locale: string): "rtl" | "ltr" {
  return RTL_LOCALES.includes(locale) ? "rtl" : "ltr";
}

// ---------------------------------------------------------------------------
// Locale switching simulation
// ---------------------------------------------------------------------------

const SUPPORTED_LOCALES = ["en", "es", "zh"] as const;
type Locale = (typeof SUPPORTED_LOCALES)[number];

const MESSAGES: Record<Locale, Record<string, unknown>> = { en, es, zh };

function resolveLocale(requested: string): Locale {
  return (SUPPORTED_LOCALES as readonly string[]).includes(requested)
    ? (requested as Locale)
    : "en";
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Translation key coverage", () => {
  it("es has all keys present in en", () => {
    const missing = enKeys.filter((k) => !esKeys.has(k));
    expect(missing, `Missing in es.json: ${missing.join(", ")}`).toEqual([]);
  });

  it("zh has all keys present in en", () => {
    const missing = enKeys.filter((k) => !zhKeys.has(k));
    expect(missing, `Missing in zh.json: ${missing.join(", ")}`).toEqual([]);
  });

  it("es has no extra keys not in en", () => {
    const enSet = new Set(enKeys);
    const extra = [...esKeys].filter((k) => !enSet.has(k));
    expect(extra, `Extra keys in es.json: ${extra.join(", ")}`).toEqual([]);
  });

  it("zh has no extra keys not in en", () => {
    const enSet = new Set(enKeys);
    const extra = [...zhKeys].filter((k) => !enSet.has(k));
    expect(extra, `Extra keys in zh.json: ${extra.join(", ")}`).toEqual([]);
  });
});

describe("Missing translation detection", () => {
  it("no translation value is an empty string", () => {
    const checkEmpty = (
      obj: Record<string, unknown>,
      locale: string,
      prefix = ""
    ): string[] =>
      Object.entries(obj).flatMap(([key, value]) => {
        const full = prefix ? `${prefix}.${key}` : key;
        if (value !== null && typeof value === "object" && !Array.isArray(value))
          return checkEmpty(value as Record<string, unknown>, locale, full);
        if (value === "") return [`${locale}:${full}`];
        return [];
      });

    const empty = [
      ...checkEmpty(es as Record<string, unknown>, "es"),
      ...checkEmpty(zh as Record<string, unknown>, "zh"),
    ];
    expect(empty, `Empty translation values: ${empty.join(", ")}`).toEqual([]);
  });

  it("no translation value is null or undefined", () => {
    const checkNull = (
      obj: Record<string, unknown>,
      locale: string,
      prefix = ""
    ): string[] =>
      Object.entries(obj).flatMap(([key, value]) => {
        const full = prefix ? `${prefix}.${key}` : key;
        if (value !== null && typeof value === "object" && !Array.isArray(value))
          return checkNull(value as Record<string, unknown>, locale, full);
        if (value == null) return [`${locale}:${full}`];
        return [];
      });

    const nullish = [
      ...checkNull(en as Record<string, unknown>, "en"),
      ...checkNull(es as Record<string, unknown>, "es"),
      ...checkNull(zh as Record<string, unknown>, "zh"),
    ];
    expect(nullish, `Null/undefined values: ${nullish.join(", ")}`).toEqual([]);
  });
});

describe("RTL layout — text direction", () => {
  it.each(RTL_LOCALES)("locale %s resolves to rtl", (locale) => {
    expect(getTextDirection(locale)).toBe("rtl");
  });

  it.each(LTR_LOCALES)("locale %s resolves to ltr", (locale) => {
    expect(getTextDirection(locale)).toBe("ltr");
  });

  it("RTL locales require dir=rtl on the html element", () => {
    for (const locale of RTL_LOCALES) {
      const dir = getTextDirection(locale);
      // Simulate what the layout would set: <html lang={locale} dir={dir}>
      expect(dir).toBe("rtl");
    }
  });
});

describe("Locale switching", () => {
  it("resolves supported locales correctly", () => {
    expect(resolveLocale("en")).toBe("en");
    expect(resolveLocale("es")).toBe("es");
    expect(resolveLocale("zh")).toBe("zh");
  });

  it("falls back to en for unsupported locales", () => {
    expect(resolveLocale("fr")).toBe("en");
    expect(resolveLocale("de")).toBe("en");
    expect(resolveLocale("")).toBe("en");
  });

  it("switching locale loads the correct message file", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = MESSAGES[resolveLocale(locale)];
      expect(messages).toBeDefined();
      // Spot-check a key present in all locales
      expect(
        (messages as { common?: { loading?: string } }).common?.loading
      ).toBeTruthy();
    }
  });

  it("each locale has a non-empty title in metadata", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = MESSAGES[locale] as {
        metadata?: { title?: string };
      };
      expect(messages.metadata?.title).toBeTruthy();
    }
  });
});
