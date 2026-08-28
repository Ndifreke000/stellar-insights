# Translation completeness audit — 2026-07-26

Addresses stellar-insights#1872.

## Supported locales

Enumerated from `src/i18n/routing.ts` (the single source of truth for
`next-intl` routing):

- `en` (default)
- `es`
- `zh`

Message files live at `messages/<locale>.json`, one flat-ish nested JSON tree
per locale, all three present.

## Key-set completeness

A dependency-free script, `scripts/i18n-audit.js`, walks
`src/i18n/routing.ts` to get the locale list, flattens each
`messages/<locale>.json` into dot-separated keys, and diffs every non-default
locale against `en`. Run with:

```
node scripts/i18n-audit.js
```

Result as of this audit:

```
es: 131 / 131 keys, 0 missing, 0 extra, 0 empty/null
zh: 131 / 131 keys, 0 missing, 0 extra, 0 empty/null
```

**No missing or extra translation keys in either locale.** This corroborates
(and now has a standalone, CI-independent script backing) the equivalent
checks already present in `src/__tests__/i18n.test.ts`
("Translation key coverage" / "Missing translation detection" describe
blocks), which run the same flatten+diff logic as Vitest unit tests. The two
approaches are complementary: the Vitest suite catches regressions in normal
`npm test` runs, while `scripts/i18n-audit.js` can be run standalone (no
test runner, no install) for a quick manual or CI-step check, and it
additionally reports string-length deltas (see below), which the existing
test suite does not.

## Layout risk: string-length deltas

The script also flags translated strings that are ≥1.8x longer or ≤0.5x
shorter than their `en` counterpart, as a proxy for the "obviously broken
layout from significantly longer/shorter translated strings" spot-check
called for in the issue's third acceptance criterion.

**es** — 5 strings ≥1.8x longer than `en`, 1 string ≤0.5x shorter:

| key | en | es |
|---|---|---|
| `common.retry` | Retry (5) | Reintentar (10) |
| `layout.sidebar.trustlines` | Trustlines (10) | Líneas de confianza (19) |
| `contact.form.email` | Email (5) | Correo electrónico (18) |
| `dashboard.topAssets` | Top Assets (10) | Activos principales (19) |
| `dashboard.lastUpdate` | Last Update (11) | Última actualización (20) |
| `layout.sidebar.network` | Network (7) | Red (3) |

These are all short UI labels (sidebar nav items, buttons, form labels) — the
kind of string most likely to wrap or truncate in a fixed-width nav item or
button if not given flexible layout. **Recommended manual spot-check**: the
sidebar (`layout.sidebar.*`) and the dashboard top-assets card in the `es`
locale, to confirm these don't clip or overflow.

**zh** — 0 strings longer than `en`; 114 of 131 strings are ≤0.5x shorter
(expected: CJK text is inherently far more compact per-character than Latin
script, e.g. `common.error`: "Error" (5) → "错误" (2)). This is not a
completeness or layout risk in the usual sense — short CJK strings are the
correct, expected outcome — but it's included in the audit for visibility to
whoever performs the manual `zh` spot-check, in case any of the 17
non-shrunk keys were left in Latin script by mistake or fell back to `en`.

## What this doesn't cover

Per the current task's scope (no install/build/dev-server), this audit does
not run the app to visually confirm layout in a browser — it identifies
*which* strings are the highest-risk candidates for a human to spot-check,
per the issue's own acceptance criteria wording ("a script comparing key
sets across locale files is the reliable way to do [completeness checking],
not manual spot-checking" — layout itself still needs a human look). A
maintainer with a running dev server can visit `/es/dashboard` and
`/es/*` sidebar-visible routes and check the flagged keys above.
