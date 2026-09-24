import * as Sentry from "@sentry/nextjs";

Sentry.init({
  dsn: process.env.NEXT_PUBLIC_SENTRY_DSN,
  tracesSampleRate: 1.0,
  environment: process.env.NODE_ENV,
  
  // Release tracking for source map uploads
  release: process.env.APP_VERSION || process.env.NEXT_PUBLIC_APP_VERSION || process.env.VERCEL_GIT_COMMIT_SHA || "0.1.0",
  
  // Error sampling (100% for now)
  sampleRate: 1.0,
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