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
function redactSensitiveData(data: unknown): unknown {
  if (typeof data === 'string') {
    // Redact Stellar addresses (56 chars starting with G)
    data = data.replace(/G[A-Z0-9]{55}/g, 'G****[REDACTED]');
    
    // Redact potential API keys
    data = data.replace(/\b[A-Za-z0-9_-]{32,}\b/g, '[REDACTED_KEY]');
    
    // Redact email addresses
    data = data.replace(/[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/g, '****@[REDACTED]');
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
    
    return redacted;
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

/**
 * Send error to tracking service (Sentry) and backend fallback.
 */
function sendToErrorTracking(error: unknown, metadata?: LogMetadata): void {
  if (isDevelopment || !isLoggingEnabled) {
    return;
  }

  const normalizedError = normalizeTrackingError(error, 'Unknown frontend error');
  const redactedMetadata = metadata ? redactSensitiveData(metadata) : undefined;

  import("@sentry/nextjs")
    .then((Sentry) => {
      Sentry.captureException(normalizedError, {
        tags: {
          logger: "frontend",
        },
        extra: redactedMetadata,
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
