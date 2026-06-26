import DOMPurify from 'dompurify';

/**
 * Strip HTML from user-controlled or network-sourced strings before rendering as text.
 */
export function sanitizeText(value: string | null | undefined): string {
  if (!value) {
    return '';
  }
  return DOMPurify.sanitize(value, { ALLOWED_TAGS: [], ALLOWED_ATTR: [] });
}

/**
 * Sanitize URLs from untrusted metadata before using in href/src attributes.
 */
export function sanitizeUrl(value: string | null | undefined): string | null {
  if (!value) {
    return null;
  }

  const cleaned = DOMPurify.sanitize(value, { ALLOWED_TAGS: [], ALLOWED_ATTR: [] }).trim();
  if (!cleaned) {
    return null;
  }

  if (/^https?:\/\//i.test(cleaned) || cleaned.startsWith('mailto:')) {
    return cleaned;
  }

  return null;
}

/**
 * Sanitize email addresses from untrusted metadata.
 */
export function sanitizeEmail(value: string | null | undefined): string | null {
  if (!value) {
    return null;
  }

  const cleaned = sanitizeText(value).trim();
  if (!cleaned || cleaned.includes('<') || cleaned.includes('>')) {
    return null;
  }

  return cleaned;
}
