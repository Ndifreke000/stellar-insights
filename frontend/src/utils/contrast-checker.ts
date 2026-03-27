/**
 * WCAG 2.1 Color Contrast Checker
 * Validates color combinations meet accessibility standards
 */

/**
 * Calculate relative luminance of a color
 * @param color - Hex color string (e.g., "#ffffff")
 * @returns Relative luminance value (0-1)
 */
function getLuminance(color: string): number {
  // Remove # if present
  const hex = color.replace('#', '');
  
  // Parse RGB values
  const rgb = parseInt(hex, 16);
  const r = ((rgb >> 16) & 0xff) / 255;
  const g = ((rgb >> 8) & 0xff) / 255;
  const b = (rgb & 0xff) / 255;
  
  // Apply gamma correction
  const [rs, gs, bs] = [r, g, b].map(c =>
    c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4)
  );
  
  // Calculate luminance using ITU-R BT.709 coefficients
  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
}

/**
 * Calculate contrast ratio between two colors
 * @param foreground - Foreground color hex string
 * @param background - Background color hex string
 * @returns Contrast ratio (1-21)
 */
export function getContrastRatio(foreground: string, background: string): number {
  const l1 = getLuminance(foreground);
  const l2 = getLuminance(background);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  
  return (lighter + 0.05) / (darker + 0.05);
}

/**
 * Check if contrast ratio meets WCAG AA standards
 * @param ratio - Contrast ratio to check
 * @param isLargeText - Whether text is large (18pt+ or 14pt+ bold)
 * @returns True if meets WCAG AA standards
 */
export function meetsWCAG_AA(ratio: number, isLargeText: boolean = false): boolean {
  return isLargeText ? ratio >= 3 : ratio >= 4.5;
}

/**
 * Check if contrast ratio meets WCAG AAA standards
 * @param ratio - Contrast ratio to check
 * @param isLargeText - Whether text is large (18pt+ or 14pt+ bold)
 * @returns True if meets WCAG AAA standards
 */
export function meetsWCAG_AAA(ratio: number, isLargeText: boolean = false): boolean {
  return isLargeText ? ratio >= 4.5 : ratio >= 7;
}

/**
 * Get WCAG compliance level for a contrast ratio
 * @param ratio - Contrast ratio to check
 * @param isLargeText - Whether text is large
 * @returns Compliance level string
 */
export function getComplianceLevel(ratio: number, isLargeText: boolean = false): string {
  if (meetsWCAG_AAA(ratio, isLargeText)) return 'AAA';
  if (meetsWCAG_AA(ratio, isLargeText)) return 'AA';
  return 'Fail';
}

/**
 * Validate color pair and return detailed results
 */
export function validateColorPair(
  foreground: string,
  background: string,
  isLargeText: boolean = false
): {
  ratio: number;
  meetsAA: boolean;
  meetsAAA: boolean;
  level: string;
  formatted: string;
} {
  const ratio = getContrastRatio(foreground, background);
  return {
    ratio,
    meetsAA: meetsWCAG_AA(ratio, isLargeText),
    meetsAAA: meetsWCAG_AAA(ratio, isLargeText),
    level: getComplianceLevel(ratio, isLargeText),
    formatted: `${ratio.toFixed(2)}:1`,
  };
}
