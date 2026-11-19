/**
 * Shared application logger for PayRaider mobile.
 *
 * ## Why this exists
 * Bare `console.log` calls in production:
 *  - leak internal state and token prefixes to device logs accessible via
 *    `adb logcat` / the macOS Console app on non-jailbroken devices,
 *  - cost measurable JSThread time at scale,
 *  - make it impossible to filter signal from noise in crash reports.
 *
 * ## Levels
 * | Level | __DEV__ | Release |
 * |-------|---------|---------|
 * | debug | ✅      | ❌      |
 * | info  | ✅      | ❌      |
 * | warn  | ✅      | ✅ (console.warn — visible in Sentry / crash tooling) |
 * | error | ✅      | ✅ (console.error) |
 *
 * ## Usage
 * ```ts
 * import { logger } from '@utils/logger';
 *
 * logger.debug('[Auth]', 'token refreshed');
 * logger.info('[Notifications]', 'FCM token registered');
 * logger.warn('[Notifications]', 'Dropping malformed payload:', detail);
 * logger.error('[App]', 'Failed to initialise:', err);
 * ```
 *
 * ## Stripping in release
 * `__DEV__` is set to `false` by the React Native Metro bundler in release
 * builds, which makes the `debug` and `info` branches dead code.  Terser /
 * Hermes bytecode compilation will tree-shake them out entirely, so there is
 * zero runtime overhead in production for those two levels.
 *
 * `warn` and `error` are intentionally preserved in release so that
 * Sentry (or any `console` interceptor wired at app boot) can capture them.
 * They never log secret values — callers must sanitise arguments before
 * passing them here.
 */


export interface Logger {
  /**
   * Verbose development-only tracing.  Stripped entirely from release builds.
   */
  debug(tag: string, ...args: unknown[]): void;

  /**
   * Informational messages useful during development.  Stripped from release.
   */
  info(tag: string, ...args: unknown[]): void;

  /**
   * Unexpected-but-recoverable conditions.  Kept in release so crash-reporting
   * tools (Sentry, Firebase Crashlytics) can surface them.
   * NEVER pass secret values (tokens, seeds, PII) to this method.
   */
  warn(tag: string, ...args: unknown[]): void;

  /**
   * Errors that should be investigated.  Always logged; forwarded to crash
   * reporters in release.
   * NEVER pass secret values (tokens, seeds, PII) to this method.
   */
  error(tag: string, ...args: unknown[]): void;
}

const _logger: Logger = {
  debug(tag: string, ...args: unknown[]): void {
    if (__DEV__) {
      console.log(`[DEBUG] ${tag}`, ...args);
    }
  },

  info(tag: string, ...args: unknown[]): void {
    if (__DEV__) {
      console.log(`[INFO] ${tag}`, ...args);
    }
  },

  warn(tag: string, ...args: unknown[]): void {
    // warn is preserved in release for Sentry / crash tooling
    console.warn(`[WARN] ${tag}`, ...args);
  },

  error(tag: string, ...args: unknown[]): void {
    // error is preserved in release for Sentry / crash tooling
    console.error(`[ERROR] ${tag}`, ...args);
  },
};

export const logger: Logger = _logger;
