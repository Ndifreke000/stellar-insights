# Frontend: i18n Completeness Check and RTL Layout Support

**Issue:** [#2839](https://github.com/Ndifreke000/stellar-insights/issues/2839)  
**Difficulty:** Hard  
**Estimated size:** L — roughly 1–2 weeks  

---

## Summary

Routes are localised but there is no CI check for missing or unused translation keys, and all layouts assume left-to-right text direction. Arabic-speaking remittance corridors are a target market. This issue adds a CI completeness script, migrates layout CSS to logical properties, wires the `dir` attribute per locale, formats numbers/currencies/dates via `Intl`, and ships an RTL locale (Arabic).

---

## Problem

| File | Lines | Issue |
|---|---|---|
| `frontend/src/app/[locale]/layout.tsx` | 91 | Sets locale but does not set `dir`; CSS uses physical properties (`left`, `right`, `margin-left`) |
| `frontend/messages/` | — | Per-locale JSON files; no tooling to verify completeness |

When a new translation key is added for one locale and forgotten in another, the missing key either falls back to the key name or throws at runtime — neither is caught in CI today.

---

## Proposed Implementation

### 1. Missing/unused key detection script

Create `frontend/scripts/check-i18n.ts`. It reads all locale message files and the English baseline, then reports:

- **Missing keys:** present in `en.json` but absent from another locale file.
- **Unused keys:** present in a locale file but absent from `en.json` (likely stale after a rename).
- **Extra keys:** present in a non-English locale but absent from `en.json` (possible rogue addition).

```ts
// scripts/check-i18n.ts
import { readdirSync, readFileSync } from 'fs';
import { join } from 'path';

const messagesDir = join(__dirname, '../messages');
const baseline = JSON.parse(readFileSync(join(messagesDir, 'en.json'), 'utf8'));
const baseKeys = new Set(Object.keys(baseline));

let exitCode = 0;

for (const file of readdirSync(messagesDir).filter(f => f !== 'en.json')) {
  const locale = file.replace('.json', '');
  const messages = JSON.parse(readFileSync(join(messagesDir, file), 'utf8'));
  const localeKeys = new Set(Object.keys(messages));

  const missing = [...baseKeys].filter(k => !localeKeys.has(k));
  const extra   = [...localeKeys].filter(k => !baseKeys.has(k));

  if (missing.length) { console.error(`[${locale}] Missing keys: ${missing.join(', ')}`); exitCode = 1; }
  if (extra.length)   { console.warn(`[${locale}] Extra keys: ${extra.join(', ')}`); }
}

process.exit(exitCode);
```

Add to `package.json`:

```json
"check:i18n": "tsx scripts/check-i18n.ts"
```

Add to CI:

```yaml
- name: i18n completeness check
  run: cd frontend && npm run check:i18n
```

### 2. Logical CSS properties

Replace all physical directional CSS properties with their logical equivalents throughout `src/`:

| Physical | Logical |
|---|---|
| `margin-left` / `margin-right` | `margin-inline-start` / `margin-inline-end` |
| `padding-left` / `padding-right` | `padding-inline-start` / `padding-inline-end` |
| `left` / `right` (positioned) | `inset-inline-start` / `inset-inline-end` |
| `border-left` / `border-right` | `border-inline-start` / `border-inline-end` |
| `text-align: left` | `text-align: start` |
| `text-align: right` | `text-align: end` |

Run a codemod to catch the bulk of occurrences:

```bash
npx @codemod/cli --transform logical-css src/
```

Review the diff manually for cases where a physical direction is intentional (e.g., an icon that must always face right).

### 3. `dir` attribute per locale

In `frontend/src/app/[locale]/layout.tsx`, map each locale to its text direction and apply it to the `<html>` element:

```tsx
// frontend/src/app/[locale]/layout.tsx
const RTL_LOCALES = new Set(['ar', 'he', 'fa', 'ur']);

export default function LocaleLayout({ children, params }: Props) {
  const { locale } = params;
  const dir = RTL_LOCALES.has(locale) ? 'rtl' : 'ltr';

  return (
    <html lang={locale} dir={dir}>
      <body>{children}</body>
    </html>
  );
}
```

This single attribute, combined with logical CSS properties, flips the entire layout for RTL locales with no component-level changes needed.

### 4. Number, currency, and date formatting via `Intl`

Replace all manual `toLocaleString()` calls and hard-coded format strings with a central formatting utility that respects the active locale:

```ts
// frontend/src/lib/format.ts
export function formatCurrency(amount: number, currency: string, locale: string): string {
  return new Intl.NumberFormat(locale, { style: 'currency', currency }).format(amount);
}

export function formatDate(date: Date, locale: string, options?: Intl.DateTimeFormatOptions): string {
  return new Intl.DateTimeFormat(locale, options).format(date);
}

export function formatNumber(value: number, locale: string, options?: Intl.NumberFormatOptions): string {
  return new Intl.NumberFormat(locale, options).format(value);
}
```

Key considerations:
- Arabic uses Arabic-Indic numerals (`٠١٢٣٤٥٦٧٨٩`) by default via `Intl`. If the design spec requires Latin numerals, pass `{ numberingSystem: 'latn' }`.
- Currency symbols position and direction are handled automatically by `Intl.NumberFormat`.
- Hijri calendar dates require a separate `Intl.DateTimeFormat` with `{ calendar: 'islamic-umalqura' }` if needed; that is out of scope for this issue.

### 5. Add an Arabic (`ar`) locale

Create `frontend/messages/ar.json` with Arabic translations for all keys present in `en.json`. Initial scope: all keys used on the core dashboard, preflight, and bookmarks pages. Remaining pages can use English fallback with a `// TODO: translate` comment tracked in a follow-up issue.

```json
{
  "nav.home": "الرئيسية",
  "nav.dashboard": "لوحة التحكم",
  "preflight.source_asset": "الأصل المصدر",
  ...
}
```

Wire the locale in `next.config.ts`:

```ts
i18n: {
  locales: ['en', 'ar', /* existing locales */],
  defaultLocale: 'en',
}
```

---

## Tasks

- [ ] `scripts/check-i18n.ts` completeness script wired to CI
- [ ] Logical CSS properties throughout `src/` (codemod + manual review)
- [ ] `dir` attribute applied per locale in `layout.tsx`
- [ ] `formatCurrency`, `formatDate`, `formatNumber` utility in `lib/format.ts`; existing call sites migrated
- [ ] `ar.json` locale file (core pages)
- [ ] Update docs / README where behaviour changes
- [ ] Add or update tests (see test plan)

---

## Test Plan

**CI script:** Add a locale file with a deliberately missing key → `check:i18n` exits non-zero.

**Visual regression (Playwright):** Key pages under the `ar` locale render without layout overflow, with text flowing right-to-left.

```ts
// tests/rtl.spec.ts
const RTL_PAGES = ['/', '/dashboard', '/bookmarks'];

for (const path of RTL_PAGES) {
  test(`${path} renders correctly in RTL`, async ({ page }) => {
    await page.goto(`/ar${path}`);
    await expect(page.locator('html')).toHaveAttribute('dir', 'rtl');
    await expect(page).toHaveScreenshot(`rtl${path.replace('/', '-')}.png`);
  });
}
```

**Additional cases:**
- Currency `formatCurrency(2500, 'NGN', 'ar')` returns a string containing the correct symbol in the correct position.
- Switching locale from `en` to `ar` at runtime flips `dir` without a full page reload.
- The completeness script passes with the new `ar.json` file present.

---

## Acceptance Criteria

- [ ] Missing-key CI check runs and fails the build on incomplete locale files
- [ ] `ar` RTL locale renders correctly on key pages
- [ ] CI is green (lint, type-check, tests)
- [ ] No new TODOs without a linked issue

---

## Implementation Notes

Write a design note in the PR covering:

- Scope of the initial `ar.json` translation (which pages are covered vs. deferred).
- Decision on Arabic-Indic vs. Latin numerals and where the override lives.
- Whether Tailwind's `rtl:` variant is used in addition to logical CSS properties, and the boundary between the two approaches.

Tailwind v3 supports `rtl:` and `ltr:` variants natively. If the codebase uses Tailwind, prefer those variants for one-off directional overrides rather than a separate stylesheet.

---

## Getting Started

```bash
git clone https://github.com/Ndifreke000/stellar-insights.git
cd stellar-insights

# Backend
cd backend && cp .env.example .env && cargo run
cargo test

# Frontend
cd ../frontend && npm install && npm run dev

# SDKs
cd ../sdk/typescript && npm install && npm test
cd ../python && pip install -e ".[dev]" && pytest

# Contracts
cd ../../contracts && cargo test --workspace
```

Verify the core endpoint:

```bash
curl -s "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
```

---

## Definition of Done

- [ ] Code merged to `main` with tests passing
- [ ] All acceptance criteria checked
- [ ] PR description explains the change, links this issue (`Closes #2839`), and lists anything deferred
- [ ] Docs updated where user-facing or integrator-facing behaviour changed
