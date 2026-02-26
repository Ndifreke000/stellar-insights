/**
 * Locale-aware formatting utilities.
 * Use these in Client Components with useFormatter from next-intl,
 * or pass locale to Intl APIs in components that don't have access to the formatter.
 */

/**
 * Format a number with the current locale.
 * Use in Client Components: const format = useFormatter(); format.number(value, options)
 * Or use directly with locale: formatNumber(value, locale, options)
 */
export function formatNumber(
  value: number,
  locale: string,
  options?: Intl.NumberFormatOptions
): string {
  return new Intl.NumberFormat(locale, {
    ...options,
  }).format(value);
}

/**
 * Format a number as currency.
 */
export function formatCurrency(
  value: number,
  locale: string,
  currency = "USD",
  options?: Intl.NumberFormatOptions
): string {
  return new Intl.NumberFormat(locale, {
    style: "currency",
    currency,
    ...options,
  }).format(value);
}

/**
 * Format a date with the current locale.
 */
export function formatDate(
  date: Date | string | number,
  locale: string,
  options?: Intl.DateTimeFormatOptions
): string {
  const d = typeof date === "object" && "getTime" in date ? date : new Date(date);
  return new Intl.DateTimeFormat(locale, {
    ...options,
  }).format(d);
}

/**
 * Format a date and time with the current locale.
 */
export function formatDateTime(
  date: Date | string | number,
  locale: string,
  options?: Intl.DateTimeFormatOptions
): string {
  const d = typeof date === "object" && "getTime" in date ? date : new Date(date);
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "short",
    timeStyle: "short",
    ...options,
  }).format(d);
}
