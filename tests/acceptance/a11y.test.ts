import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

/**
 * Automated accessibility scan (stellar-insights#1871).
 *
 * Static jsdom-rendered component tests (frontend/src/components/__tests__/accessibility.a11y.test.tsx)
 * and ESLint's jsx-a11y plugin only catch a fraction of real accessibility issues — they never
 * render real CSS (no color-contrast checking), never lay out a full page (no landmark/heading
 * structure or focus-order checking), and don't exercise live-region announcements for dynamic
 * content. This suite runs axe-core against every real route in a real browser via the existing
 * root Playwright setup (playwright.config.ts), which is the setup this issue calls for.
 *
 * Routes are every static (non-dynamic-segment) page under src/app, scanned in the default
 * locale (`en`). Dynamic-segment routes (anchors/[address], corridors/[pair], governance/[id])
 * are intentionally excluded — they need a real, valid identifier to render meaningful content
 * rather than a not-found/error state, so scanning them requires seeding fixture data first;
 * that's a natural follow-up once this baseline is in CI.
 */

const LOCALE = 'en';

const LOCALIZED_ROUTES = [
  '',
  'about',
  'analytics',
  'analytics/api',
  'analytics/export',
  'anchors',
  'calculator',
  'contact',
  'corridors',
  'corridors/compare',
  'dashboard',
  'deposit-withdraw',
  'developer/keys',
  'governance',
  'health',
  'how-to-use',
  'internal/monitoring',
  'liquidity',
  'liquidity-pools',
  'network',
  'notifications',
  'performance',
  'prediction',
  'quests',
  'send-payment',
  'sep10-demo',
  'sep6',
  'settings',
  'transactions/builder',
  'trustlines',
].map((route) => `/${LOCALE}${route ? `/${route}` : ''}`);

const UNLOCALIZED_ROUTES = [
  '/alerts',
  '/api-docs',
  '/api-docs/examples',
  '/api-docs/playground',
  '/components/time-range',
  '/demo/notifications',
  '/offline',
  '/settings/gdpr',
];

const ROUTES = [...LOCALIZED_ROUTES, ...UNLOCALIZED_ROUTES];

// Impact levels that fail the build outright. "minor"/"moderate" findings are reported but
// don't fail CI — see the triage note in frontend/docs/a11y-scan-2026-07-26.md.
const BLOCKING_IMPACTS = ['critical', 'serious'];

for (const route of ROUTES) {
  test(`axe-core scan: ${route}`, async ({ page }) => {
    await page.goto(route);

    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
      .analyze();

    const blocking = results.violations.filter((v) =>
      BLOCKING_IMPACTS.includes(v.impact ?? 'minor')
    );

    if (results.violations.length > 0) {
      // eslint-disable-next-line no-console
      console.log(
        `[a11y] ${route}: ${results.violations.length} violation(s) — ` +
          results.violations
            .map((v) => `${v.id} (${v.impact}, ${v.nodes.length} node(s))`)
            .join(', ')
      );
    }

    expect(
      blocking,
      `${route} has ${blocking.length} critical/serious axe violation(s): ` +
        blocking.map((v) => `${v.id}: ${v.help}`).join('; ')
    ).toEqual([]);
  });
}
