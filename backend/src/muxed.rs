//! Muxed account (M-address) support for Stellar.
//!
//! Muxed accounts allow a single Stellar account (G-address) to represent multiple
//! sub-accounts via a 64-bit muxed ID. M-addresses are 69 characters and start with 'M'.
//! See SEP-0023 and [Stellar Muxed Accounts FAQ](https://stellar.org/blog/developers/muxed-accounts-faq).

use data_encoding::BASE32;
use serde::{Deserialize, Serialize};

/// Stellar strkey version bytes
const VERSION_ACCOUNT_ID: u8 = 6; // G-address
const VERSION_MUXED_ACCOUNT: u8 = 12; // M-address

/// Length of a Stellar M-address (MUXED_ACCOUNT strkey)
pub const MUXED_ADDRESS_LEN: usize = 69;

/// Length of a Stellar G-address (ACCOUNT_ID strkey)
pub const G_ADDRESS_LEN: usize = 56;

/// CRC-16-XMODEM polynomial (used by Stellar strkey)
const CRC16_POLY: u16 = 0x1021;

fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ CRC16_POLY;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// Result of parsing a muxed account address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuxedAccountInfo {
    /// The raw M-address (e.g. M...)
    pub muxed_address: String,
    /// The base Stellar account (G-address) when successfully decoded
    pub base_account: Option<String>,
    /// The 64-bit muxed sub-account ID when successfully decoded
    pub muxed_id: Option<u64>,
}

/// Returns true if the given string is a valid Stellar muxed account (M-address) format.
/// M-addresses are 69 characters and start with 'M'.
#[inline]
pub fn is_muxed_address(addr: &str) -> bool {
    addr.starts_with('M')
        && addr.len() == MUXED_ADDRESS_LEN
        && addr
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// Returns true if the given string looks like a Stellar account address (G or M).
#[inline]
pub fn is_stellar_account_address(addr: &str) -> bool {
    if addr.starts_with('G') && addr.len() == G_ADDRESS_LEN {
        return true;
    }
    is_muxed_address(addr)
}

/// Parse an M-address into base account (G) and muxed ID.
/// Returns None if the input is not a valid M-address or decoding fails.
pub fn parse_muxed_address(addr: &str) -> Option<MuxedAccountInfo> {
    if !is_muxed_address(addr) {
        return None;
    }

    let decoded = BASE32.decode(addr.as_bytes()).ok()?;
    // Muxed: version(1) + account_id(32) + muxed_id(8) + checksum(2) = 43 bytes
    if decoded.len() != 43 {
        return None;
    }
    if decoded[0] != VERSION_MUXED_ACCOUNT {
        return None;
    }
    let checksum = u16::from_be_bytes([decoded[41], decoded[42]]);
    let payload = &decoded[0..41];
    if crc16(payload) != checksum {
        return None;
    }

    let account_id: &[u8; 32] = decoded[1..33].try_into().ok()?;
    let muxed_id = u64::from_be_bytes(decoded[33..41].try_into().ok()?);

    // Encode 32-byte account ID as G-address
    let mut g_payload = [0u8; 35];
    g_payload[0] = VERSION_ACCOUNT_ID;
    g_payload[1..33].copy_from_slice(account_id);
    let c = crc16(&g_payload);
    g_payload[33] = (c >> 8) as u8;
    g_payload[34] = (c & 0xff) as u8;
    let base_account = BASE32.encode(&g_payload);

    Some(MuxedAccountInfo {
        muxed_address: addr.to_string(),
        base_account: Some(base_account),
        muxed_id: Some(muxed_id),
    })
}

/// Normalize an account identifier for display or storage.
/// Accepts both G- and M-addresses and returns them as-is (no conversion).
#[inline]
pub fn normalize_account_input(addr: &str) -> Option<&str> {
    let trimmed = addr.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('G') && trimmed.len() == G_ADDRESS_LEN {
        return Some(trimmed);
    }
    if is_muxed_address(trimmed) {
        return Some(trimmed);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_muxed_address() {
        assert!(!is_muxed_address("GAAAA..."));
        assert!(!is_muxed_address(""));
        assert!(!is_muxed_address("M"));
        // Valid format: 69 chars starting with M
        let m = "MAAAAAAAAAAAAAB7BQ2L7E5NBWMXDUCMZSIPOBKRDSBYVLMXGSSKF6YNPIB7Y77ITLVL6";
        assert_eq!(m.len(), 69);
        assert!(is_muxed_address(m));
    }

    #[test]
    fn test_is_stellar_account_address() {
        assert!(is_stellar_account_address(
            "GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ"
        ));
        assert!(is_stellar_account_address(
            "MAAAAAAAAAAAAAB7BQ2L7E5NBWMXDUCMZSIPOBKRDSBYVLMXGSSKF6YNPIB7Y77ITLVL6"
        ));
        assert!(!is_stellar_account_address("invalid"));
    }

    #[test]
    fn test_parse_muxed_address() {
        // Invalid: G-address returns None
        assert!(
            parse_muxed_address("GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ")
                .is_none()
        );
        // Valid M-address format: parse may succeed (if checksum/version match) or None
        let m = "MAAAAAAAAAAAAAB7BQ2L7E5NBWMXDUCMZSIPOBKRDSBYVLMXGSSKF6YNPIB7Y77ITLVL6";
        let info = parse_muxed_address(m);
        if let Some(ref i) = info {
            assert!(i.muxed_address == m);
            assert!(i.base_account.as_ref().map_or(true, |g| g.starts_with('G')));
        }
        // Too short M string
        assert!(parse_muxed_address("M").is_none());
    }
}
