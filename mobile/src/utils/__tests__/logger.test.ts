/**
 * Tests for the shared logger utility (issue #2626).
 *
 * Key guarantees verified:
 *  - debug/info are no-ops when __DEV__ is false (release mode)
 *  - debug/info call console.log when __DEV__ is true
 *  - warn always calls console.warn (both dev and release)
 *  - error always calls console.error (both dev and release)
 */

// We control __DEV__ through the global object below (its type is already
// declared by React Native's global typings).

const mockLog = jest.spyOn(console, 'log').mockImplementation(() => {});
const mockWarn = jest.spyOn(console, 'warn').mockImplementation(() => {});
const mockError = jest.spyOn(console, 'error').mockImplementation(() => {});

// Helper to reload the module with a given __DEV__ value
function loadLogger(isDev: boolean) {
  jest.resetModules();
  // Set global before require so the module-level check picks it up
  (global as unknown as Record<string, unknown>).__DEV__ = isDev;
  return require('@utils/logger').logger as import('@utils/logger').Logger;
}

beforeEach(() => {
  mockLog.mockClear();
  mockWarn.mockClear();
  mockError.mockClear();
});

afterAll(() => {
  mockLog.mockRestore();
  mockWarn.mockRestore();
  mockError.mockRestore();
});

describe('logger — DEV mode (__DEV__ = true)', () => {
  let logger: import('@utils/logger').Logger;

  beforeAll(() => {
    logger = loadLogger(true);
  });

  it('debug() calls console.log', () => {
    logger.debug('[Test]', 'hello');
    expect(mockLog).toHaveBeenCalledTimes(1);
    expect(mockLog).toHaveBeenCalledWith('[DEBUG] [Test]', 'hello');
  });

  it('info() calls console.log', () => {
    logger.info('[Test]', 'world');
    expect(mockLog).toHaveBeenCalledTimes(1);
    expect(mockLog).toHaveBeenCalledWith('[INFO] [Test]', 'world');
  });

  it('warn() calls console.warn', () => {
    logger.warn('[Test]', 'oops');
    expect(mockWarn).toHaveBeenCalledTimes(1);
    expect(mockWarn).toHaveBeenCalledWith('[WARN] [Test]', 'oops');
    expect(mockLog).not.toHaveBeenCalled();
  });

  it('error() calls console.error', () => {
    const err = new Error('boom');
    logger.error('[Test]', err);
    expect(mockError).toHaveBeenCalledTimes(1);
    expect(mockError).toHaveBeenCalledWith('[ERROR] [Test]', err);
    expect(mockLog).not.toHaveBeenCalled();
  });
});

describe('logger — RELEASE mode (__DEV__ = false)', () => {
  let logger: import('@utils/logger').Logger;

  beforeAll(() => {
    logger = loadLogger(false);
  });

  it('debug() is a no-op', () => {
    logger.debug('[Test]', 'secret stuff');
    expect(mockLog).not.toHaveBeenCalled();
  });

  it('info() is a no-op', () => {
    logger.info('[Test]', 'more stuff');
    expect(mockLog).not.toHaveBeenCalled();
  });

  it('warn() still calls console.warn (kept for Sentry)', () => {
    logger.warn('[Test]', 'still visible');
    expect(mockWarn).toHaveBeenCalledTimes(1);
  });

  it('error() still calls console.error (kept for crash reporters)', () => {
    logger.error('[Test]', 'still visible');
    expect(mockError).toHaveBeenCalledTimes(1);
  });
});
