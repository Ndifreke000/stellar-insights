/**
 * Centralized logging utility for the application
 * 
 * Features:
 * - Environment-aware logging (development vs production)
 * - Structured logging with metadata
 * - Error tracking integration ready
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
 * Send error to tracking service (Sentry, LogRocket, etc.)
 */
function sendToErrorTracking(error: Error, metadata?: LogMetadata): void {
  if (isDevelopment || !isLoggingEnabled) {
    return;
  }
  
  // Integration point for error tracking services
  // Example: Sentry.captureException(error, { extra: metadata });
  
  // For now, we'll just prepare the data structure
  const errorData = {
    message: error.message,
    stack: error.stack,
    metadata: redactSensitiveData(metadata),
    timestamp: new Date().toISOString(),
    userAgent: typeof window !== 'undefined' ? window.navigator.userAgent : 'unknown',
  };
  
  // TODO: Send to actual error tracking service
  // This prevents errors from being lost in production
  if (typeof window !== 'undefined') {
    // Store in sessionStorage as fallback
    try {
      const errors = JSON.parse(sessionStorage.getItem('app_errors') || '[]');
      errors.push(errorData);
      // Keep only last 10 errors
      sessionStorage.setItem('app_errors', JSON.stringify(errors.slice(-10)));
    } catch {
      // Ignore storage errors
    }
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
      console.log(formatMessage('DEBUG', message), redactedMetadata);
    } else {
      console.log(formatMessage('DEBUG', message));
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
    
    if (isDevelopment) {
      if (error instanceof Error) {
        console.error(formatMessage('ERROR', message), error, redactedMetadata);
      } else if (error) {
        console.error(formatMessage('ERROR', message), error, redactedMetadata);
      } else {
        console.error(formatMessage('ERROR', message), redactedMetadata);
      }
    }
    
    // Send to error tracking in production
    if (!isDevelopment && error instanceof Error) {
      sendToErrorTracking(error, { message, ...redactedMetadata });
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
    console.log(formatMessage('WS', `WebSocket ${event}`), redactedData);
  },

  /**
   * Log API requests (only in development)
   */
  api: (method: string, url: string, metadata?: LogMetadata): void => {
    if (!isLoggingEnabled || !isDevelopment) {
      return;
    }
    
    const redactedMetadata = metadata ? redactSensitiveData(metadata) : undefined;
    console.log(formatMessage('API', `${method} ${url}`), redactedMetadata);
  },

  /**
   * Performance logging (only in development)
   */
  performance: (label: string, duration: number, metadata?: LogMetadata): void => {
    if (!isLoggingEnabled || !isDevelopment) {
      return;
    }
    
    const redactedMetadata = metadata ? redactSensitiveData(metadata) : undefined;
    console.log(
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
