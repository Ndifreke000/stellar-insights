# QA: Verify Notifications Center End-to-End in a Browser

**Issue:** #1821
**Type:** QA / Manual Verification
**Component:** Frontend
**Routes:** `/[locale]/notifications`, `/demo/notifications`

## Context

A prior stabilization pass got backend, contracts, and frontend build/typecheck/lint green, but nothing was clicked through in a running browser. This doc was meant to record that manual walkthrough of the Enhanced Notification Center's four tabs (Notifications / Analytics / Filters / Settings).

**That manual walkthrough could not be completed as scoped**, because static investigation turned up problems that make the acceptance criteria, as written, unsatisfiable against the two named routes. Filing those findings here per the issue's own instruction ("If something's broken, file a follow-up bug with the specific failure, and note it here").

## 🔴 Finding 1 — Neither named route renders the component under test

The issue asks to verify the "Enhanced Notification Center" (`frontend/src/components/notifications/EnhancedNotificationCenter/EnhancedNotificationCenter.tsx`), but:

- [`/[locale]/notifications`](frontend/src/app/[locale]/notifications/page.tsx) renders `NotificationsDemo`, a page of `showToast()` demo buttons plus `WalletConnectionDemo` — it never imports `EnhancedNotificationCenter`.
- [`/demo/notifications`](frontend/src/app/demo/notifications/page.tsx) renders `NotificationCenterDemo` ([frontend/src/components/notifications/NotificationCenterDemo.tsx](frontend/src/components/notifications/NotificationCenterDemo.tsx)), a different, older component — it also never imports `EnhancedNotificationCenter`.

A repo-wide search confirms `EnhancedNotificationCenter` is not imported anywhere outside its own directory:
```
grep -rln "EnhancedNotificationCenter" frontend/src --include=*.tsx --include=*.ts | grep -v "EnhancedNotificationCenter/"
# (no results)
```
There is no route in the app that currently mounts this component, so "confirm tab switching works" etc. can't be verified by visiting either URL in the issue — the four-tab UI simply isn't there to click.

## 🔴 Finding 2 — The component has unresolved imports even if mounted

Isolating just this component subtree with `tsc` (rather than the whole project — see Finding 4 on why that hid these) surfaces real `TS2307: Cannot find module` errors:

| File | Missing import |
|---|---|
| [EnhancedNotificationCenter.tsx:12](frontend/src/components/notifications/EnhancedNotificationCenter/EnhancedNotificationCenter.tsx#L12) | `@/components/ui/tabs` |
| [EnhancedNotificationCenter.tsx:13](frontend/src/components/notifications/EnhancedNotificationCenter/EnhancedNotificationCenter.tsx#L13) | `@/components/ui/tooltip` |
| [NotificationHeader.tsx:6](frontend/src/components/notifications/EnhancedNotificationCenter/NotificationHeader.tsx#L6) | `@/components/ui/tooltip` |
| [FiltersTabView.tsx:9](frontend/src/components/notifications/EnhancedNotificationCenter/FiltersTabView.tsx#L9) | `@/components/ui/checkbox` |
| [NotificationItem.tsx:14](frontend/src/components/notifications/EnhancedNotificationCenter/NotificationItem.tsx#L14) | `@/components/ui/checkbox` |
| [NotificationItem.tsx:23](frontend/src/components/notifications/EnhancedNotificationCenter/NotificationItem.tsx#L23) | `@/components/ui/dropdown-menu` |
| [NotificationItem.tsx:24](frontend/src/components/notifications/EnhancedNotificationCenter/NotificationItem.tsx#L24) | `./Constants` (no `Constants.ts`/`.tsx` file exists in this directory) |

None of `tabs.tsx`, `tooltip.tsx`, `checkbox.tsx`, or `dropdown-menu.tsx` exist under `frontend/src/components/ui/` — confirmed with `find frontend/src -iname "*tabs*" -o -iname "*tooltip*" -o -iname "*checkbox*" -o -iname "*dropdown-menu*"`, which returns nothing. These aren't missing npm packages, they're missing local files — the "hand-rolled Tabs primitive" mentioned as fixed this session does not exist in the working tree.

## 🔴 Finding 3 — Analytics tab calls a method that doesn't exist

[AnalyticsView.tsx:27](frontend/src/components/notifications/EnhancedNotificationCenter/AnalyticsView.tsx#L27) calls:
```ts
NotificationService.getInstance().getAnalytics(notifications)
```
but [`NotificationService`](frontend/src/services/notificationService.ts) has no `getAnalytics` method — the actual method is `generateAnalytics` ([notificationService.ts:154](frontend/src/services/notificationService.ts#L154)). This is a plain name mismatch (`TS2339: Property 'getAnalytics' does not exist on type 'NotificationService'`) and would break the Analytics tab specifically, one of the four tabs this issue asks to verify.

## 🔴 Finding 4 — The "green" project-wide typecheck is not actually checking this code

Running `npx tsc --noEmit -p tsconfig.json` from `frontend/` (with `tsconfig.tsbuildinfo` cleared, so it's not reusing a stale incremental cache) reports only 8 errors total, all in two unrelated files:
```
src/app/[locale]/anchors/components/AnchorCards.tsx(142,13): error TS17002: Expected corresponding JSX closing tag for 'article'.
src/components/notifications/NotificationList.tsx(220,7): error TS17002: Expected corresponding JSX closing tag for 'article'.
src/lib/zustand/middleware.ts(8,46): error TS1005: ';' expected.
... (5 more in middleware.ts)
```
None of the Finding 1–3 errors show up in that run — but isolating `EnhancedNotificationCenter.tsx` alone (via a scratch tsconfig including just that file) immediately surfaces 20+ real errors, including all the missing-module errors above. The parse errors in `AnchorCards.tsx`/`middleware.ts` appear to short-circuit the rest of the project-wide check. **This means a passing `tsc` run for this project cannot currently be trusted as evidence that a given component compiles** — worth its own follow-up, separate from the notification center itself.

## 🔴 Finding 5 — No installed dependencies in this environment

`frontend/node_modules` does not exist in this working copy, so `npm run dev` cannot be started as-is; `npm install` (inside `frontend/`, not the repo root — the repo root `package.json` belongs to an unrelated Vite project, not this one) is a precondition for any of this issue's acceptance criteria. Flagging since the issue's checklist assumes `npm run dev` "just works."

## Acceptance Criteria — status

- [ ] ~~`npm run dev` loads the route with no console errors~~ — not attempted; no installed dependencies in this environment (Finding 5), and the target component isn't on either route regardless (Finding 1)
- [ ] ~~The feature works as intended~~ — cannot be exercised; `EnhancedNotificationCenter` isn't mounted anywhere in the app (Finding 1), and would fail to compile/render if it were (Findings 2–3)
- [ ] ~~API calls succeed against a real backend~~ — not reached
- [x] Filed follow-up findings above with specific failures, as the issue's own acceptance criteria ask for

## Suggested Next Steps

1. Decide whether `EnhancedNotificationCenter` is meant to replace `NotificationCenterDemo`/the bell-triggered `NotificationSystem.tsx` — right now it looks like unfinished, unwired work. If it's meant to ship, wire it into a route (or into [`layout/notification-center.tsx`](frontend/src/components/layout/notification-center.tsx) / the header bell) so it's reachable.
2. Add the missing `frontend/src/components/ui/{tabs,tooltip,checkbox,dropdown-menu}.tsx` primitives (or import existing equivalents if they exist under a different name) and the missing `./Constants` file in `EnhancedNotificationCenter/`.
3. Fix `AnalyticsView.tsx`'s `getAnalytics` → `generateAnalytics` call.
4. Fix the two parse errors in `AnchorCards.tsx` and `lib/zustand/middleware.ts` so `tsc --noEmit` stops masking downstream errors, then re-run a full project typecheck to see the true error count.
5. Once the above compiles, re-run this QA pass for real in a browser against whichever route ends up hosting the component.
