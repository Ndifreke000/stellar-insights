/**
 * Calculates the contrast ratio between two hex colors.
 * Formulas based on WCAG 2.1 guidelines.
 */
export function getContrastRatio(foreground: string, background: string): number {
    const getLuminance = (color: string): number => {
        // Remove # if present
        const hex = color.startsWith('#') ? color.slice(1) : color;

        // Convert to RGB
        const rgb = parseInt(hex, 16);
        const r = ((rgb >> 16) & 0xff) / 255;
        const g = ((rgb >> 8) & 0xff) / 255;
        const b = (rgb & 0xff) / 255;

        const [rs, gs, bs] = [r, g, b].map(c =>
            c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4)
        );

        return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
    };

    const l1 = getLuminance(foreground);
    const l2 = getLuminance(background);
    const lighter = Math.max(l1, l2);
    const darker = Math.min(l1, l2);

    return (lighter + 0.05) / (darker + 0.05);
}

/**
 * Checks if a contrast ratio meets WCAG AA standards.
 * @param ratio The calculated contrast ratio.
 * @param isLargeText Whether the text is considered large (18pt/24px or 14pt/18.67px bold).
 */
export function meetsWCAG_AA(ratio: number, isLargeText: boolean = false): boolean {
    return isLargeText ? ratio >= 3 : ratio >= 4.5;
}
