import * as Sentry from "@sentry/nextjs";

Sentry.init({
  dsn: process.env.NEXT_PUBLIC_SENTRY_DSN,
  tracesSampleRate: 1.0,
  environment: process.env.NODE_ENV,
  
  // Release tracking for source map uploads
  release: process.env.APP_VERSION || "unknown",
  
  // Error sampling (100% for now)
  errorSampleRate: 1.0,
  
  // Attach server context to errors
  initialScope: {
    tags: {
      component: "backend",
      platform: "server",
    },
  },
  
  integrations: [
    // Breadcrumbs for server-side events
    new Sentry.Breadcrumbs({
      console: true,
      sentry: true,
    }),
  ],
});