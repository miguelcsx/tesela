//! Authenticated encryption for secrets at rest.
//!
//! The [`Sealer`] port provides encrypt/decrypt with random nonce prepending.
//! [`AesGcmSealer`] is the default implementation using AES-128/192/256-GCM.
//!
//! Key lengths: 16 bytes = AES-128, 24 bytes = AES-192, 32 bytes = AES-256.
//! The nonce (12 bytes) is prepended to the ciphertext on [`Sealer::seal`]
//! and consumed from the front on [`Sealer::open`].

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes128Gcm, Aes256Gcm,
};
use lattice_core::Error;

/// Nonce byte-length for AES-GCM (fixed by the AEAD spec).
pub const NONCE_SIZE: usize = 12;

// ---------------------------------------------------------------------------
// Sealer port trait
// ---------------------------------------------------------------------------

/// Authenticated encryption port.
///
/// Implementations must be safe for concurrent use.  The nonce strategy and
/// key management are left to the implementation.
///
/// For Vault Transit or cloud KMS, implement this trait directly.
pub trait Sealer: Send + Sync {
    /// Encrypt `plaintext` and return `nonce || ciphertext`.
    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, Error>;
    /// Decrypt a payload previously produced by [`seal`](Self::seal).
    fn open(&self, ciphertext: &[u8]) -> Result<Vec<u8>, Error>;
}

// ---------------------------------------------------------------------------
// Internal enum to dispatch between 128-bit and 256-bit ciphers
// ---------------------------------------------------------------------------

enum Inner {
    Aes128(Box<Aes128Gcm>),
    Aes256(Box<Aes256Gcm>),
}

impl Inner {
    fn encrypt(&self, nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, aes_gcm::Error> {
        use aes_gcm::Nonce;
        let n = Nonce::from_slice(nonce);
        match self {
            Self::Aes128(c) => c.encrypt(n, plaintext),
            Self::Aes256(c) => c.encrypt(n, plaintext),
        }
    }

    fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, aes_gcm::Error> {
        use aes_gcm::Nonce;
        let n = Nonce::from_slice(nonce);
        match self {
            Self::Aes128(c) => c.decrypt(n, ciphertext),
            Self::Aes256(c) => c.decrypt(n, ciphertext),
        }
    }
}

// ---------------------------------------------------------------------------
// AesGcmSealer
// ---------------------------------------------------------------------------

/// AES-GCM authenticated encryption sealer.
///
/// The random 12-byte nonce is generated via the OS CSPRNG on every
/// [`seal`](Sealer::seal) call and prepended to the output.
///
/// ```rust
/// use lattice_runtime::crypto::{AesGcmSealer, Sealer};
///
/// let key = [0u8; 32]; // 32 bytes = AES-256-GCM
/// let sealer = AesGcmSealer::new(&key).unwrap();
/// let ct = sealer.seal(b"hello").unwrap();
/// let pt = sealer.open(&ct).unwrap();
/// assert_eq!(pt, b"hello");
/// ```
pub struct AesGcmSealer {
    inner: Inner,
}

impl AesGcmSealer {
    /// Create a sealer from a raw key.
    ///
    /// Key lengths: 16 = AES-128, 32 = AES-256.  AES-192 is not supported
    /// by the underlying crate.
    pub fn new(key: &[u8]) -> Result<Self, Error> {
        let inner = match key.len() {
            16 => Inner::Aes128(Box::new(
                Aes128Gcm::new_from_slice(key)
                    .map_err(|_| Error::validation("invalid AES-128 key length"))?,
            )),
            32 => Inner::Aes256(Box::new(
                Aes256Gcm::new_from_slice(key)
                    .map_err(|_| Error::validation("invalid AES-256 key length"))?,
            )),
            n => {
                return Err(Error::validation(format!(
                    "AES-GCM key must be 16 or 32 bytes, got {}",
                    n
                )));
            }
        };
        Ok(Self { inner })
    }

    /// Derive a sealer from a hex-encoded key string (lowercase or uppercase).
    pub fn from_hex(hex_key: &str) -> Result<Self, Error> {
        let hex = hex_key.trim();
        if !hex.len().is_multiple_of(2) {
            return Err(Error::validation("hex key has odd length"));
        }
        let bytes: Result<Vec<u8>, _> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
            .collect();
        Self::new(&bytes.map_err(|_| Error::validation("invalid hex character in key"))?)
    }
}

impl Sealer for AesGcmSealer {
    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, Error> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .inner
            .encrypt(&nonce, plaintext)
            .map_err(|e| Error::internal(format!("encryption failed: {}", e)))?;
        let mut out = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    fn open(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        if data.len() < NONCE_SIZE {
            return Err(Error::validation("ciphertext too short: missing nonce"));
        }
        let (nonce, ciphertext) = data.split_at(NONCE_SIZE);
        self.inner
            .decrypt(nonce, ciphertext)
            .map_err(|_| Error::validation("decryption failed: invalid key or tampered ciphertext"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_aes256() {
        let key = [0x42u8; 32];
        let s = AesGcmSealer::new(&key).unwrap();
        let ct = s.seal(b"secret data").unwrap();
        let pt = s.open(&ct).unwrap();
        assert_eq!(pt, b"secret data");
    }

    #[test]
    fn round_trip_aes128() {
        let key = [0x11u8; 16];
        let s = AesGcmSealer::new(&key).unwrap();
        let ct = s.seal(b"another secret").unwrap();
        assert_eq!(s.open(&ct).unwrap(), b"another secret");
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let key = [0xBBu8; 32];
        let s = AesGcmSealer::new(&key).unwrap();
        let mut ct = s.seal(b"data").unwrap();
        // Flip a byte in the ciphertext portion.
        *ct.last_mut().unwrap() ^= 0xFF;
        assert!(s.open(&ct).is_err());
    }

    #[test]
    fn invalid_key_length_rejected() {
        assert!(AesGcmSealer::new(&[0u8; 17]).is_err());
    }

    #[test]
    fn nonces_differ_per_call() {
        let key = [0xAAu8; 32];
        let s = AesGcmSealer::new(&key).unwrap();
        let ct1 = s.seal(b"x").unwrap();
        let ct2 = s.seal(b"x").unwrap();
        assert_ne!(
            &ct1[..NONCE_SIZE],
            &ct2[..NONCE_SIZE],
            "nonces must be random"
        );
    }
}
