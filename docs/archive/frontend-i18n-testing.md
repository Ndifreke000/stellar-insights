# Frontend Internationalization Testing

Tracking issue: #2413

## Summary

The frontend uses **next-intl** with locale-prefixed routes (`frontend/src/app/[locale]/`).
The relevant pieces are:

| Piece              | Location                                         |
|--------------------|--------------------------------------------------|
| Supported locales  | `frontend/src/i18n/routing.ts`: `en` (default), `es`, `zh`, with `localePrefix: "always"` |
| Message catalogs   | `frontend/messages/{en,es,zh}.json`: 131 leaf keys each |
| Request config     | `frontend/src/i18n/request.ts`                   |
| Navigation helpers | `frontend/src/i18n/navigation.ts`                |
| i18n test suite    | `frontend/src/__tests__/i18n.test.ts`            |

## Running the checks

```bash
cd frontend
npm run test:i18n        # vitest run src/__tests__/i18n.test.ts
npm run i18n:coverage    # same suite, verbose reporter (per-check output)
```

## What the suite covers

- **Key parity:** every key in `en.json` exists in `es.json` and `zh.json`, and neither
  catalog has keys that `en.json` lacks.
- **Missing translations:** no translation value is an empty string.
- **Text direction:** `getTextDirection` maps `ar`, `he`, `fa` and `ur` to `rtl` and all other
  locales to `ltr`.
- **Locale switching:** `resolveLocale` falls back to `en` for unsupported locales, and each
  supported locale resolves to its own catalog.

`en.json` is the **source of truth**. Add new keys there first.

## Adding or changing a translation key

1. Add the key to `messages/en.json`.
2. Add the same key path to **every** other catalog. If a translation isn't ready, use the
   English text rather than an empty string, because empty strings fail the suite.
3. Use the key through `useTranslations("<namespace>")` or `getTranslations`.
4. Run `npm run test:i18n`.

## Adding a new locale

1. Create `messages/<locale>.json` by copying `en.json` and translating it.
2. Add the locale to `locales` in `src/i18n/routing.ts`.
3. In `i18n.test.ts`, add it to `SUPPORTED_LOCALES` and `MESSAGES`, and add a parity check
   (import it and compare its key set with `enKeys`).
4. **For an RTL locale** (for example `ar` or `he`):
   - Make sure `app/[locale]/layout.tsx` sets `<html lang={locale} dir={getTextDirection(locale)}>`.
   - Use logical CSS properties or Tailwind utilities (`ms-*`, `me-*`, `ps-*`, `pe-*`,
     `start-*`, `end-*`) instead of `ml-*`, `mr-*`, `left-*` and `right-*`.
   - Mirror directional icons such as arrows and chevrons with `rtl:rotate-180`.
   - Check charts, tables and the sidebar by hand at `/<locale>`.

## Manual RTL / locale-switching checklist

- [ ] `/en`, `/es` and `/zh` all render without raw key strings such as `common.loading`.
- [ ] The locale switcher keeps the current path (for example `/es/anchors` → `/zh/anchors`).
- [ ] An unsupported prefix (`/xx/...`) redirects to or falls back on the default locale.
- [ ] Dates and numbers are formatted for the active locale.
- [ ] For RTL locales: `dir="rtl"` is set on `<html>`, the layout is mirrored, and text in
      inputs is right-aligned.

## Known gaps / follow-ups

- **No RTL locale ships today.** RTL support is only checked at the helper level. No real page
  has been rendered and verified in `dir="rtl"`.
- **Not in CI.** No workflow under `.github/workflows/` runs `npm run test:i18n`. Add it as a
  step in `frontend-build.yml`:

  ```yaml
  - name: i18n checks
    run: npm run test:i18n
    working-directory: frontend
  ```

- **No unused-key detection.** Keys that no component references anymore are not flagged.
- **`i18n:coverage` is just a verbose run.** It does not produce a per-locale percentage report.
- The locale lists are duplicated in `routing.ts` and the test file. Importing `routing.locales`
  in the test would keep them in sync.
