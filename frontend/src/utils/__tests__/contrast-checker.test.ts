import { describe, it, expect } from 'vitest';
import {
  getContrastRatio,
  meetsWCAG_AA,
  meetsWCAG_AAA,
  getComplianceLevel,
  validateColorPair,
} from '../contrast-checker';

describe('Color Contrast Checker', () => {
  describe('getContrastRatio', () => {
    it('should calculate correct ratio for black on white', () => {
      const ratio = getContrastRatio('#000000', '#ffffff');
      expect(ratio).toBeCloseTo(21, 1);
    });

    it('should calculate correct ratio for white on black', () => {
      const ratio = getContrastRatio('#ffffff', '#000000');
      expect(ratio).toBeCloseTo(21, 1);
    });

    it('should return 1:1 for same colors', () => {
      const ratio = getContrastRatio('#ffffff', '#ffffff');
      expect(ratio).toBe(1);
    });
  });

  describe('WCAG AA Compliance', () => {
    it('should pass for primary text (Gray-900 on white)', () => {
      const ratio = getContrastRatio('#111827', '#ffffff');
      expect(meetsWCAG_AA(ratio)).toBe(true);
      expect(ratio).toBeGreaterThan(4.5);
    });

    it('should pass for secondary text (Gray-700 on white)', () => {
      const ratio = getContrastRatio('#374151', '#ffffff');
      expect(meetsWCAG_AA(ratio)).toBe(true);
      expect(ratio).toBeGreaterThan(4.5);
    });

    it('should pass for muted text (Gray-600 on white)', () => {
      const ratio = getContrastRatio('#4b5563', '#ffffff');
      expect(meetsWCAG_AA(ratio)).toBe(true);
      expect(ratio).toBeGreaterThan(4.5);
    });

    it('should pass for disabled text (Gray-500 on white)', () => {
      const ratio = getContrastRatio('#6b7280', '#ffffff');
      expect(meetsWCAG_AA(ratio)).toBe(true);
      expect(ratio).toBeGreaterThan(4.5);
    });

    it('should fail for old muted-foreground (Gray-400 on white)', () => {
      const ratio = getContrastRatio('#9ca3af', '#ffffff');
      expect(meetsWCAG_AA(ratio)).toBe(false);
      expect(ratio).toBeLessThan(4.5);
    });
  });

  describe('Dark Mode Compliance', () => {
    it('should pass for primary text (Gray-50 on dark)', () => {
      const ratio = getContrastRatio('#f9fafb', '#020617');
      expect(meetsWCAG_AA(ratio)).toBe(true);
    });

    it('should pass for secondary text (Gray-200 on dark)', () => {
      const ratio = getContrastRatio('#e5e7eb', '#020617');
      expect(meetsWCAG_AA(ratio)).toBe(true);
    });

    it('should pass for muted text (Gray-300 on dark)', () => {
      const ratio = getContrastRatio('#d1d5db', '#020617');
      expect(meetsWCAG_AA(ratio)).toBe(true);
    });
  });

  describe('Link Colors', () => {
    it('should pass for primary link (Blue-600 on white)', () => {
      const ratio = getContrastRatio('#2563eb', '#ffffff');
      expect(meetsWCAG_AA(ratio)).toBe(true);
      expect(ratio).toBeGreaterThan(4.5);
    });

    it('should pass for link hover (Blue-700 on white)', () => {
      const ratio = getContrastRatio('#1d4ed8', '#ffffff');
      expect(meetsWCAG_AA(ratio)).toBe(true);
    });
  });

  describe('Status Colors', () => {
    it('should pass for success (Green-600 on white)', () => {
      const ratio = getContrastRatio('#059669', '#ffffff');
      expect(meetsWCAG_AA(ratio)).toBe(true);
    });

    it('should pass for warning (Amber-600 on white)', () => {
      const ratio = getContrastRatio('#d97706', '#ffffff');
      expect(meetsWCAG_AA(ratio)).toBe(true);
    });

    it('should pass for error (Red-600 on white)', () => {
      const ratio = getContrastRatio('#dc2626', '#ffffff');
      expect(meetsWCAG_AA(ratio)).toBe(true);
    });
  });

  describe('Large Text', () => {
    it('should pass AA for 3:1 ratio with large text', () => {
      expect(meetsWCAG_AA(3.5, true)).toBe(true);
    });

    it('should fail AA for 2.5:1 ratio even with large text', () => {
      expect(meetsWCAG_AA(2.5, true)).toBe(false);
    });
  });

  describe('getComplianceLevel', () => {
    it('should return AAA for high contrast', () => {
      const ratio = getContrastRatio('#000000', '#ffffff');
      expect(getComplianceLevel(ratio)).toBe('AAA');
    });

    it('should return AA for medium contrast', () => {
      const ratio = getContrastRatio('#6b7280', '#ffffff');
      expect(getComplianceLevel(ratio)).toBe('AA');
    });

    it('should return Fail for low contrast', () => {
      const ratio = getContrastRatio('#9ca3af', '#ffffff');
      expect(getComplianceLevel(ratio)).toBe('Fail');
    });
  });

  describe('validateColorPair', () => {
    it('should return complete validation results', () => {
      const result = validateColorPair('#111827', '#ffffff');
      expect(result.meetsAA).toBe(true);
      expect(result.level).toBe('AAA');
      expect(result.formatted).toMatch(/\d+\.\d+:1/);
    });
  });
});
