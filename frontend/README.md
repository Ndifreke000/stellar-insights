# Frontend

Next.js frontend for Stellar Insights.

## Getting started

```bash
npm install
npm run dev
```

## Demo pages and components

Demo routes and components are gated behind the `NEXT_PUBLIC_ENABLE_DEMOS`
environment flag so they are not shipped in production builds.

- `/demo/notifications` — demo notifications page (`src/app/demo/notifications/page.tsx`)
- `WebSocketDemo` — WebSocket demo component (`src/components/WebSocketDemo.tsx`)
- `PWAExample` — PWA example component (`src/components/examples/PWAExample.tsx`)

To enable demos locally, set the flag before starting the dev server:

```bash
NEXT_PUBLIC_ENABLE_DEMOS=true npm run dev
```

When the flag is unset (the default, and always in production), the demo route
returns a 404 and the demo components are not rendered.
