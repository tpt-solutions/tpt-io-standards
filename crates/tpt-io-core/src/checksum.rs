//! Checksum helpers (CRC32, MD5, SHA-256).
//!
//! These wrappers are only compiled when the `checksums` feature is enabled.
//! They are intentionally thin: each delegates to a well-audited, `no_std`
//! capable crate so the workspace keeps its "pure Rust, no FFI" guarantee.

#![cfg(feature = "checksums")]

/// Compute the CRC-32 (IEEE 802.3) of `data`.
pub fn crc32(data: &[u8]) -> u32 {
    crc32fast::hash(data)
}

/// Compute the MD5 digest of `data` as a 16-byte array.
pub fn md5(data: &[u8]) -> [u8; 16] {
    use md5::Digest;
    let mut hasher = md5::Md5::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute the SHA-256 digest of `data` as a 32-byte array.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_known_vector() {
        // CRC-32 of "123456789" (the canonical check string) is 0xCBF43926.
        assert_eq!(crc32(b"123456789"), 0xCBF_43926);
    }

    #[test]
    fn sha256_known_vector() {
        let digest = sha256(b"abc");
        // SHA-256("abc") first bytes are ba78 1602 ...
        assert_eq!(digest[..4], [0xba, 0x78, 0x16, 0xbf]);
    }

    #[test]
    fn md5_known_vector() {
        let digest = md5(b"abc");
        assert_eq!(
            digest,
            [
                0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0, 0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1,
                0x7f, 0x72
            ]
        );
    }
}
