import { describe, it, expect, beforeEach, vi } from 'vitest';
import { logger } from '../logger';

describe('Logger utility with Sentry integration', () => {
  beforeEach(() => {
    logger.clearUserContext();
    vi.clearAllMocks();
  });

  it('manages user context correctly', () => {
    expect(logger.getUserContext()).toBeNull();

    logger.setUserContext({
      id: 'usr_123',
      email: 'test@payraider.com',
      username: 'payuser',
    });

    const user = logger.getUserContext();
    expect(user).not.toBeNull();
    expect(user?.id).toBe('usr_123');
    expect(user?.email).toBe('test@payraider.com');
    expect(user?.username).toBe('payuser');

    logger.clearUserContext();
    expect(logger.getUserContext()).toBeNull();
  });

  it('records breadcrumbs correctly and retrieves them', () => {
    const beforeCount = logger.getBreadcrumbs().length;

    logger.addBreadcrumb({
      category: 'ui.click',
      message: 'User clicked submit button',
      level: 'info',
      data: { buttonId: 'submit-payment' },
    });

    const breadcrumbs = logger.getBreadcrumbs();
    expect(breadcrumbs.length).toBeGreaterThan(beforeCount);
    const last = breadcrumbs[breadcrumbs.length - 1];
    expect(last.category).toBe('ui.click');
    expect(last.message).toBe('User clicked submit button');
    expect(last.data?.buttonId).toBe('submit-payment');
  });

  it('records breadcrumbs on API calls', () => {
    logger.api('GET', '/api/v1/corridors');

    const breadcrumbs = logger.getBreadcrumbs();
    const match = breadcrumbs.find(
      (b) => b.category === 'http' && b.message === 'GET /api/v1/corridors'
    );
    expect(match).toBeDefined();
  });

  it('allows capturing exceptions without throwing', () => {
    expect(() => {
      logger.captureException(new Error('Test error for error tracking'), {
        context: 'test-suite',
      });
    }).not.toThrow();
  });
});
