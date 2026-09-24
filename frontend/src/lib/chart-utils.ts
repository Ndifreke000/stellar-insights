/**
 * Chart utility functions for cross-browser compatibility
 * Handles Safari-specific rendering issues and provides consistent styling
 */

/**
 * Detects if the browser is Safari
 * Safari has limited support for backdropFilter in inline styles
 */
export function isSafari(): boolean {
  if (typeof window === 'undefined') return false;
  const ua = window.navigator.userAgent;
  return /^((?!chrome|android).)*safari/i.test(ua);
}

/**
 * Gets tooltip content style with Safari compatibility
 * Safari doesn't support backdropFilter in inline styles, so we use a fallback
 */
export function getTooltipContentStyle(options?: {
  backgroundColor?: string;
  borderRadius?: string;
  border?: string;
  color?: string;
  fontSize?: string;
  fontFamily?: string;
}): Record<string, string | number | undefined> {
  const baseStyle: Record<string, string | number | undefined> = {
    backgroundColor: options?.backgroundColor || 'rgba(15, 23, 42, 0.95)',
    borderRadius: options?.borderRadius || '12px',
    border: options?.border || '1px solid rgba(255, 255, 255, 0.1)',
    color: options?.color || '#f8fafc',
    fontSize: options?.fontSize || '12px',
    fontFamily: options?.fontFamily || 'monospace',
  };

  // Safari doesn't support backdropFilter in inline styles
  // Use a more opaque background instead
  if (!isSafari()) {
    return {
      ...baseStyle,
      backdropFilter: 'blur(8px)',
    };
  }

  // For Safari, increase opacity instead of using blur
  return {
    ...baseStyle,
    backgroundColor: 'rgba(15, 23, 42, 0.98)',
  };
}

/**
 * Gets tooltip label style with Safari compatibility
 */
export function getTooltipLabelStyle(): Record<string, string> {
  return {
    color: '#94a3b8',
    marginBottom: '4px',
  };
}

/**
 * Gets chart container style for proper rendering across browsers
 */
export function getChartContainerStyle(): Record<string, string> {
  return {
    width: '100%',
    height: '100%',
  };
}
