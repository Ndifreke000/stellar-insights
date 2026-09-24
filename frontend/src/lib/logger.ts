/**
 * Centralized logging utility for the application
 * 
 * Features:
 * - Environment-aware logging (development vs production)
 * - Structured logging with metadata
 * - Error tracking integration with Sentry
 * - Sensitive data redaction
 * - Type-safe logging methods
 * 
 * Usage:
 * ```typescript
 * import { logger } from '@/lib/logger';
 * 
 * logger.debug('User action', { action: 'click', component: 'Button' });
 * logger.error('API request failed', error, { endpoint: '/api/data' });
 * logger.warn('Deprecated feature used', { feature: 'oldAPI' });
 * ```
 */

const isDevelopment = process.env.NODE_ENV === 'development';
const isTest = process.env.NODE_ENV === 'test';

// Disable all logging in test environment unless explicitly enabled
const isLoggingEnabled = !isTest || process.env.ENABLE_TEST_LOGS === 'true';

interface LogMetadata {
  [key: string]: unknown;
}

/**
 * Redact sensitive data from logs
 */
function redactSensitiveData<T>(data: T): T {
  if (typeof data === 'string') {
    // Redact Stellar addresses (56 chars starting with G)
    let redactedString = data.replace(/G[A-Z0-9]{55}/g, 'G****[REDACTED]');

    // Redact potential API keys
    redactedString = redactedString.replace(/\b[A-Za-z0-9_-]{32,}\b/g, '[REDACTED_KEY]');

    // Redact email addresses
    redactedString = redactedString.replace(/[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/g, '****@[REDACTED]');

    return redactedString as T;
  }

  if (typeof data === 'object' && data !== null) {
    const redacted: Record<string, unknown> = {};

    for (const [key, value] of Object.entries(data)) {
      // Redact sensitive field names
      if (/password|secret|token|key|auth|credential/i.test(key)) {
        redacted[key] = '[REDACTED]';
      } else {
        redacted[key] = redactSensitiveData(value);
      }
    }

    return redacted as T;
  }

  return data;
}

/**
 * Format log message with timestamp and level
 */
function formatMessage(level: string, message: string): string {
  const timestamp = new Date().toISOString();
  return `[${timestamp}] [${level}] ${message}`;
}

/**
 * Convert an arbitrary error payload to an Error for tracking.
 */
function normalizeTrackingError(error: unknown, defaultMessage: string): Error {
  if (error instanceof Error) {
    return error;
  }

  if (typeof error === 'string') {
    return new Error(error);
  }

  try {
    return new Error(JSON.stringify(error));
  } catch {
    return new Error(defaultMessage);
  }
}

export interface UserContext {
  id?: string;
  email?: string;
  username?: string;
  ip_address?: string;
  [key: string]: unknown;
}

export interface Breadcrumb {
  category: string;
  message: string;
  level?: 'debug' | 'info' | 'warn' | 'error';
  data?: Record<string, unknown>;
  timestamp?: string;
}

let currentUserContext: UserContext | null = null;
const breadcrumbsBuffer: Breadcrumb[] = [];
const MAX_BREADCRUMBS = 50;

/**
 * Record a breadcrumb in the in-memory buffer and Sentry if available.
 */
function recordBreadcrumb(breadcrumb: Breadcrumb): void {
  const item: Breadcrumb = {
    ...breadcrumb,
    timestamp: breadcrumb.timestamp || new Date().toISOString(),
    data: breadcrumb.data ? redactSensitiveData(breadcrumb.data) : undefined,
  };

  breadcrumbsBuffer.push(item);
  if (breadcrumbsBuffer.length > MAX_BREADCRUMBS) {
    breadcrumbsBuffer.shift();
  }

  if (typeof window !== 'undefined') {
    import("@sentry/nextjs")
      .then((Sentry) => {
        Sentry.addBreadcrumb({
          category: item.category,
          message: item.message,
          level: item.level || 'info',
          data: item.data,
        });
      })
      .catch(() => {
        // Ignore load failures
      });
  }
}

/**
 * Send error to tracking service (Sentry) and backend fallback.
 */
function sendToErrorTracking(error: unknown, metadata?: LogMetadata): void {
  if (!isLoggingEnabled) {
    return;
  }

  const normalizedError = normalizeTrackingError(error, 'Unknown frontend error');
  const redactedMetadata = metadata ? redactSensitiveData(metadata) : undefined;
  const release = process.env.NEXT_PUBLIC_APP_VERSION || process.env.APP_VERSION || process.env.VERCEL_GIT_COMMIT_SHA || "0.1.0";

  recordBreadcrumb({
    category: 'error',
    message: normalizedError.message,
    level: 'error',
    data: redactedMetadata as Record<string, unknown>,
  });

  import("@sentry/nextjs")
    .then((Sentry) => {
      // Resolve user context
      const userId = currentUserContext?.id || (typeof window !== 'undefined' ? sessionStorage.getItem('user_id') : null);
      if (userId || currentUserContext) {
        Sentry.setUser({
          id: userId || undefined,
          email: currentUserContext?.email,
          username: currentUserContext?.username,
          ...currentUserContext,
        });
      }

      // Add breadcrumb for error context
      Sentry.addBreadcrumb({
        category: 'error',
        message: normalizedError.message,
        level: 'error',
        data: redactedMetadata,
      });

      Sentry.captureException(normalizedError, {
        tags: {
          logger: "frontend",
          environment: process.env.NODE_ENV || "production",
          release,
        },
        extra: {
          ...redactedMetadata,
          breadcrumbs: breadcrumbsBuffer.slice(-10),
          userContext: currentUserContext ? redactSensitiveData(currentUserContext) : undefined,
        },
        level: 'error',
      });
    })
    .catch(() => {
      sendErrorToBackend(normalizedError, redactedMetadata).catch(() => {
        if (typeof window !== 'undefined') {
          const errors = JSON.parse(sessionStorage.getItem('error_logs') || '[]');
          errors.push({
            message: normalizedError.message,
            stack: normalizedError.stack,
            metadata: redactedMetadata,
            release,
            userContext: currentUserContext,
            timestamp: new Date().toISOString(),
          });
          sessionStorage.setItem('error_logs', JSON.stringify(errors));
        }
      });
    });
}

async function sendErrorToBackend(error: Error, metadata?: LogMetadata): Promise<void> {
  if (typeof window === 'undefined') {
    return;
  }

  try {
    await fetch('/api/error-log', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        message: error.message,
        stack: error.stack,
        metadata,
        release: process.env.NEXT_PUBLIC_APP_VERSION || "0.1.0",
        user: currentUserContext,
        breadcrumbs: breadcrumbsBuffer.slice(-10),
      }),
    });
  } catch {
    // Swallow network failures here; sessionStorage fallback is handled by sendToErrorTracking.
  }
}

/**
 * Logger utility with environment-aware methods
 */
export const logger = {
  /**
   * Debug-level logging (only in development)
   * Use for detailed debugging information
   */
  debug: (message: string, metadata?: LogMetadata): void => {
    if (!isLoggingEnabled || !isDevelopment) {
      return;
    }
    
    const redactedMetadata = metadata ? redactSensitiveData(metadata) : undefined;
    
    if (redactedMetadata) {
      console.debug(formatMessage('DEBUG', message), redactedMetadata);
    } else {
      console.debug(formatMessage('DEBUG', message));
    }
  },

  /**
   * Info-level logging (only in development)
   * Use for general informational messages
   */
  info: (message: string, metadata?: LogMetadata): void => {
    if (!isLoggingEnabled || !isDevelopment) {
      return;
    }
    
    const redactedMetadata = metadata ? redactSensitiveData(metadata) : undefined;
    
    if (redactedMetadata) {
      console.info(formatMessage('INFO', message), redactedMetadata);
    } else {
      console.info(formatMessage('INFO', message));
    }
  },

  /**
   * Warning-level logging (only in development)
   * Use for potentially problematic situations
   */
  warn: (message: string, metadata?: LogMetadata): void => {
    if (!isLoggingEnabled || !isDevelopment) {
      return;
    }
    
    const redactedMetadata = metadata ? redactSensitiveData(metadata) : undefined;
    
    if (redactedMetadata) {
      console.warn(formatMessage('WARN', message), redactedMetadata);
    } else {
      console.warn(formatMessage('WARN', message));
    }
  },

  /**
   * Error-level logging
   * Logs to console in development, sends to tracking service in production
   */
  error: (message: string, error?: Error | unknown, metadata?: LogMetadata): void => {
    if (!isLoggingEnabled) {
      return;
    }
    
    const redactedMetadata = metadata ? redactSensitiveData(metadata) : undefined;
    const payload = error ?? message;

    if (isDevelopment) {
      if (error instanceof Error) {
        console.error(formatMessage('ERROR', message), error, redactedMetadata);
      } else if (error) {
        console.error(formatMessage('ERROR', message), error, redactedMetadata);
      } else {
        console.error(formatMessage('ERROR', message), redactedMetadata);
      }
    }

    if (!isDevelopment) {
      sendToErrorTracking(payload, { message, ...redactedMetadata });
    }
  },

  /**
   * Log WebSocket events (only in development)
   */
  websocket: (event: string, data?: unknown): void => {
    if (!isLoggingEnabled || !isDevelopment) {
      return;
    }
    
    const redactedData = data ? redactSensitiveData(data) : undefined;
    console.debug(formatMessage('WS', `WebSocket ${event}`), redactedData);
  },

  /**
   * Log API requests (only in development)
   */
  api: (method: string, url: string, metadata?: LogMetadata): void => {
    recordBreadcrumb({
      category: 'http',
      message: `${method} ${url}`,
      level: 'info',
      data: metadata ? redactSensitiveData(metadata) as Record<string, unknown> : undefined,
    });

    if (!isLoggingEnabled || !isDevelopment) {
      return;
    }
    
    const redactedMetadata = metadata ? redactSensitiveData(metadata) : undefined;
    console.debug(formatMessage('API', `${method} ${url}`), redactedMetadata);
  },

  /**
   * Performance logging (only in development)
   */
  performance: (label: string, duration: number, metadata?: LogMetadata): void => {
    if (!isLoggingEnabled || !isDevelopment) {
      return;
    }
    
    const redactedMetadata = metadata ? redactSensitiveData(metadata) : undefined;
    console.debug(
      formatMessage('PERF', `${label}: ${duration.toFixed(2)}ms`),
      redactedMetadata
    );
  },

  /**
   * Attach user context for error tracking
   */
  setUserContext: (user: UserContext): void => {
    currentUserContext = user;
    if (typeof window !== 'undefined' && user.id) {
      sessionStorage.setItem('user_id', user.id);
    }
    if (typeof window !== 'undefined') {
      import("@sentry/nextjs")
        .then((Sentry) => {
          Sentry.setUser({
            id: user.id,
            email: user.email,
            username: user.username,
            ...user,
          });
        })
        .catch(() => {});
    }
  },

  /**
   * Clear active user context
   */
  clearUserContext: (): void => {
    currentUserContext = null;
    if (typeof window !== 'undefined') {
      sessionStorage.removeItem('user_id');
      import("@sentry/nextjs")
        .then((Sentry) => {
          Sentry.setUser(null);
        })
        .catch(() => {});
    }
  },

  /**
   * Get active user context
   */
  getUserContext: (): UserContext | null => {
    return currentUserContext;
  },

  /**
   * Manually record a breadcrumb for debugging
   */
  addBreadcrumb: (breadcrumb: Breadcrumb): void => {
    recordBreadcrumb(breadcrumb);
  },

  /**
   * Retrieve recorded breadcrumbs
   */
  getBreadcrumbs: (): Breadcrumb[] => {
    return [...breadcrumbsBuffer];
  },

  /**
   * Explicitly capture an exception with Sentry and fallback tracking
   */
  captureException: (error: unknown, metadata?: LogMetadata): void => {
    sendToErrorTracking(error, metadata);
  },
};

/**
 * Create a scoped logger with a prefix
 * Useful for component-specific logging
 */
export function createScopedLogger(scope: string) {
  return {
    debug: (message: string, metadata?: LogMetadata) =>
      logger.debug(`[${scope}] ${message}`, metadata),
    info: (message: string, metadata?: LogMetadata) =>
      logger.info(`[${scope}] ${message}`, metadata),
    warn: (message: string, metadata?: LogMetadata) =>
      logger.warn(`[${scope}] ${message}`, metadata),
    error: (message: string, error?: Error | unknown, metadata?: LogMetadata) =>
      logger.error(`[${scope}] ${message}`, error, metadata),
    websocket: (event: string, data?: unknown) =>
      logger.websocket(`[${scope}] ${event}`, data),
    api: (method: string, url: string, metadata?: LogMetadata) =>
      logger.api(method, `[${scope}] ${url}`, metadata),
    performance: (label: string, duration: number, metadata?: LogMetadata) =>
      logger.performance(`[${scope}] ${label}`, duration, metadata),
  };
}

/**
 * Performance measurement utility
 */
export function measurePerformance<T>(
  label: string,
  fn: () => T,
  metadata?: LogMetadata
): T {
  const start = performance.now();
  try {
    const result = fn();
    const duration = performance.now() - start;
    logger.performance(label, duration, metadata);
    return result;
  } catch (error) {
    const duration = performance.now() - start;
    logger.error(`${label} failed after ${duration.toFixed(2)}ms`, error as Error, metadata);
    throw error;
  }
}

/**
 * Async performance measurement utility
 */
export async function measurePerformanceAsync<T>(
  label: string,
  fn: () => Promise<T>,
  metadata?: LogMetadata
): Promise<T> {
  const start = performance.now();
  try {
    const result = await fn();
    const duration = performance.now() - start;
    logger.performance(label, duration, metadata);
    return result;
  } catch (error) {
    const duration = performance.now() - start;
    logger.error(`${label} failed after ${duration.toFixed(2)}ms`, error as Error, metadata);
    throw error;
  }
}

// Export for testing
export const __testing__ = {
  redactSensitiveData,
  formatMessage,
  isDevelopment,
  isTest,
  isLoggingEnabled,
};
