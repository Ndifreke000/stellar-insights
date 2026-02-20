/**
 * Stellar account address utilities (G-addresses and M-addresses / muxed accounts).
 * M-addresses are 69 characters and start with 'M' (SEP-0023).
 */

export const G_ADDRESS_LEN = 56;
export const MUXED_ADDRESS_LEN = 69;

/**
 * Returns true if the string is a Stellar muxed account (M-address).
 * M-addresses are 69 characters and start with 'M'.
 */
export function isMuxedAddress(addr: string): boolean {
  if (!addr || typeof addr !== "string") return false;
  const s = addr.trim();
  return (
    s.length === MUXED_ADDRESS_LEN &&
    s.startsWith("M") &&
    /^[A-Z0-9]+$/.test(s)
  );
}

/**
 * Returns true if the string is a Stellar public key (G-address).
 * G-addresses are 56 characters and start with 'G'.
 */
export function isGAddress(addr: string): boolean {
  if (!addr || typeof addr !== "string") return false;
  const s = addr.trim();
  return s.length === G_ADDRESS_LEN && s.startsWith("G") && /^[A-Z0-9]+$/.test(s);
}

/**
 * Returns true if the string is a valid Stellar account address (G or M).
 */
export function isStellarAccountAddress(addr: string): boolean {
  return isGAddress(addr) || isMuxedAddress(addr);
}

/**
 * Truncate an address for display (G or M).
 * G: 56 chars -> "GXXX...XXX"
 * M: 69 chars -> "MXXX...XXX"
 */
export function formatAddressShort(
  address: string,
  startChars = 8,
  endChars = 8
): string {
  if (!address) return "";
  const s = address.trim();
  if (s.length <= startChars + endChars) return s;
  return `${s.slice(0, startChars)}...${s.slice(-endChars)}`;
}

/**
 * Format address for display: short by default, with optional full on hover/tooltip.
 * Use formatAddressShort for consistent truncation of G and M addresses.
 */
export function formatAddress(
  address: string,
  options?: { short?: boolean; maxLength?: number }
): string {
  if (!address) return "";
  const s = address.trim();
  if (options?.short ?? true) {
    const max = options?.maxLength ?? 20;
    if (s.length <= max) return s;
    return formatAddressShort(s, 6, 6);
  }
  return s;
}

/**
 * Validation message for address inputs (anchors, account lookups).
 */
export function getAddressValidationError(addr: string): string | null {
  const s = addr?.trim() ?? "";
  if (!s) return "Address is required.";
  if (isGAddress(s)) return null;
  if (isMuxedAddress(s)) return null;
  if (s.startsWith("G"))
    return "Invalid G-address: must be 56 characters (A-Z, 0-9).";
  if (s.startsWith("M"))
    return "Invalid M-address: must be 69 characters (muxed account).";
  return "Invalid address: must start with G (account) or M (muxed account).";
}
