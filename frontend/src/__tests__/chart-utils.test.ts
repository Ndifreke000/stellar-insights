import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { isSafari, getTooltipContentStyle, getTooltipLabelStyle, getChartContainerStyle } from '@/lib/chart-utils';

describe('chart-utils', () => {
  let originalUserAgent: string;

  beforeEach(() => {
    originalUserAgent = window.navigator.userAgent;
  });

  afterEach(() => {
    Object.defineProperty(window.navigator, 'userAgent', {
      value: originalUserAgent,
      writable: true,
    });
  });

  describe('isSafari', () => {
    it('should return true for Safari user agent', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.1 Safari/605.1.15',
        writable: true,
      });
      expect(isSafari()).toBe(true);
    });

    it('should return true for iOS Safari user agent', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (iPhone; CPU iPhone OS 15_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.0 Mobile/15E148 Safari/604.1',
        writable: true,
      });
      expect(isSafari()).toBe(true);
    });

    it('should return false for Chrome user agent', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36',
        writable: true,
      });
      expect(isSafari()).toBe(false);
    });

    it('should return false for Firefox user agent', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:89.0) Gecko/20100101 Firefox/89.0',
        writable: true,
      });
      expect(isSafari()).toBe(false);
    });

    it('should return false for Edge user agent', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36 Edg/91.0.864.59',
        writable: true,
      });
      expect(isSafari()).toBe(false);
    });

    it('should return false when window is undefined', () => {
      const originalWindow = global.window;
      // @ts-ignore
      delete global.window;
      expect(isSafari()).toBe(false);
      global.window = originalWindow;
    });
  });

  describe('getTooltipContentStyle', () => {
    it('should return style with backdropFilter for non-Safari browsers', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36',
        writable: true,
      });

      const style = getTooltipContentStyle();
      expect(style.backdropFilter).toBe('blur(8px)');
      expect(style.backgroundColor).toBe('rgba(15, 23, 42, 0.95)');
    });

    it('should return style without backdropFilter for Safari browsers', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.1 Safari/605.1.15',
        writable: true,
      });

      const style = getTooltipContentStyle();
      expect(style.backdropFilter).toBeUndefined();
      expect(style.backgroundColor).toBe('rgba(15, 23, 42, 0.98)');
    });

    it('should apply custom options to style', () => {
      const customStyle = getTooltipContentStyle({
        backgroundColor: 'rgba(0, 0, 0, 0.8)',
        borderRadius: '8px',
        border: '2px solid red',
        color: '#000',
        fontSize: '14px',
        fontFamily: 'Arial',
      });

      expect(customStyle.backgroundColor).toBe('rgba(0, 0, 0, 0.8)');
      expect(customStyle.borderRadius).toBe('8px');
      expect(customStyle.border).toBe('2px solid red');
      expect(customStyle.color).toBe('#000');
      expect(customStyle.fontSize).toBe('14px');
      expect(customStyle.fontFamily).toBe('Arial');
    });

    it('should use default values when options are not provided', () => {
      const style = getTooltipContentStyle();
      expect(style.backgroundColor).toBeDefined();
      expect(style.borderRadius).toBe('12px');
      expect(style.border).toBe('1px solid rgba(255, 255, 255, 0.1)');
      expect(style.color).toBe('#f8fafc');
      expect(style.fontSize).toBe('12px');
      expect(style.fontFamily).toBe('monospace');
    });

    it('should preserve custom options while using defaults for unspecified properties', () => {
      const style = getTooltipContentStyle({
        backgroundColor: 'rgba(100, 100, 100, 0.9)',
      });

      expect(style.backgroundColor).toBe('rgba(100, 100, 100, 0.9)');
      expect(style.borderRadius).toBe('12px');
      expect(style.border).toBe('1px solid rgba(255, 255, 255, 0.1)');
    });
  });

  describe('getTooltipLabelStyle', () => {
    it('should return consistent label style', () => {
      const style = getTooltipLabelStyle();
      expect(style.color).toBe('#94a3b8');
      expect(style.marginBottom).toBe('4px');
    });

    it('should return same style on multiple calls', () => {
      const style1 = getTooltipLabelStyle();
      const style2 = getTooltipLabelStyle();
      expect(style1).toEqual(style2);
    });
  });

  describe('getChartContainerStyle', () => {
    it('should return responsive container style', () => {
      const style = getChartContainerStyle();
      expect(style.width).toBe('100%');
      expect(style.height).toBe('100%');
    });

    it('should return same style on multiple calls', () => {
      const style1 = getChartContainerStyle();
      const style2 = getChartContainerStyle();
      expect(style1).toEqual(style2);
    });
  });

  describe('Safari compatibility edge cases', () => {
    it('should handle Safari on iPad', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (iPad; CPU OS 15_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.0 Mobile/15E148 Safari/604.1',
        writable: true,
      });
      expect(isSafari()).toBe(true);
    });

    it('should handle Chrome on macOS (should not be detected as Safari)', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36',
        writable: true,
      });
      expect(isSafari()).toBe(false);
    });

    it('should handle Android Chrome (should not be detected as Safari)', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Linux; Android 11; SM-G991B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.120 Mobile Safari/537.36',
        writable: true,
      });
      expect(isSafari()).toBe(false);
    });
  });
});
