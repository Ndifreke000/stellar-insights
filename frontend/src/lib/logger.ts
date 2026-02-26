const isDevelopment = process.env.NODE_ENV === "development";

export const logger = {
  debug: (message: string, ...args: any[]) => {
    if (isDevelopment) {
      console.log(`[DEBUG] ${message}`, ...args);
    }
  },
  error: (message: string, error?: Error | any) => {
    if (isDevelopment) {
      console.error(`[ERROR] ${message}`, error);
    }
    // Send to error tracking service in production
    if (!isDevelopment && typeof window !== "undefined") {
      // Sentry, LogRocket, etc. could be integrated here
    }
  },
  warn: (message: string, ...args: any[]) => {
    if (isDevelopment) {
      console.warn(`[WARN] ${message}`, ...args);
    }
  },
  info: (message: string, ...args: any[]) => {
    if (isDevelopment) {
      console.info(`[INFO] ${message}`, ...args);
    }
  },
};
