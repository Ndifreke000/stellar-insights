import * as Sentry from "@sentry/nextjs";

Sentry.init({
  dsn: process.env.NEXT_PUBLIC_SENTRY_DSN,
  tracesSampleRate: 1.0,
  environment: process.env.NODE_ENV,
  replaysOnErrorSampleRate: 1.0,
  replaysSessionSampleRate: 0.1,
  
  // Release tracking for source map uploads
  release: process.env.NEXT_PUBLIC_APP_VERSION || "unknown",
  
  // Error sampling (100% for now, can be adjusted in production)
  errorSampleRate: 1.0,
  
  // Attach user context to errors
  initialScope: {
    tags: {
      component: "frontend",
      platform: "web",
    },
  },
  
  integrations: [
    new Sentry.Replay({
      maskAllText: true,
      blockAllMedia: true,
    }),
    // Breadcrumbs for user actions
    new Sentry.Breadcrumbs({
      console: true,
      dom: true,
      fetch: true,
      history: true,
      sentry: true,
      xhr: true,
    }),
  ],
  
  // Ignore certain errors
  ignoreErrors: [
    // Browser extensions
    "top.GLOBALS",
    // Random plugins/extensions
    "chrome-extension://",
    "moz-extension://",
    // Network errors that are expected
    "NetworkError",
    "Network request failed",
  ],
  
  // Before sending to Sentry
  beforeSend(event, hint) {
    // Filter out errors from browser extensions
    if (event.exception) {
      const error = hint.originalException;
      if (error && typeof error === "string" && error.includes("chrome-extension")) {
        return null;
      }
    }
    return event;
  },
});
