'use client';

import { sanitizeText } from '@/lib/sanitize';

interface SafeTextProps {
  value: string | null | undefined;
  className?: string;
  as?: 'span' | 'p' | 'div';
}

/**
 * Renders network or user-controlled text with DOMPurify sanitization.
 */
export function SafeText({ value, className, as: Tag = 'span' }: SafeTextProps) {
  return <Tag className={className}>{sanitizeText(value)}</Tag>;
}
