//! Password-keyed symmetric encryption for the LAN link.
//!
//! Every wire frame is sealed with **ChaCha20-Poly1305** (an AEAD cipher) using
//! a 256-bit key derived from the room password. Because the cipher is
//! authenticated, a peer that supplies the wrong password cannot decrypt the
//! very first handshake frame — so the password doubles as access control:
//! a mismatch surfaces as a decryption error and the connection is refused.
//!
//! Wire layout of one sealed frame (before base64 line-encoding):
//! `nonce (12 bytes) || ciphertext+tag`.

use chacha20poly1305::aead::{Aead, AeadCore, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce};
use sha2::{Digest, Sha256};

const NONCE_LEN: usize = 12;
/// Domain-separation tag so the derived key is specific to this application.
const KDF_DOMAIN: &[u8] = b"xiangqi-lan-aead-v1";

/// A symmetric cipher keyed by the room password.
#[derive(Clone)]
pub struct Cipher {
    aead: ChaCha20Poly1305,
}

impl Cipher {
    /// Derive the per-room key from `password` (may be empty; both peers must
    /// then agree on the empty password).
    pub fn from_password(password: &str) -> Cipher {
        let mut hasher = Sha256::new();
        hasher.update(KDF_DOMAIN);
        hasher.update(password.as_bytes());
        let key_bytes = hasher.finalize();
        let key = Key::from_slice(&key_bytes);
        Cipher {
            aead: ChaCha20Poly1305::new(key),
        }
    }

    /// Seal `plaintext`, returning `nonce || ciphertext`.
    pub fn seal(&self, plaintext: &[u8]) -> Vec<u8> {
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = self
            .aead
            .encrypt(&nonce, plaintext)
            .expect("AEAD encryption is infallible for valid keys");
        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(nonce.as_slice());
        out.extend_from_slice(&ciphertext);
        out
    }

    /// Open a `nonce || ciphertext` frame. Returns `None` if authentication
    /// fails (tampered data or, most commonly, a wrong password).
    pub fn open(&self, sealed: &[u8]) -> Option<Vec<u8>> {
        if sealed.len() < NONCE_LEN {
            return None;
        }
        let (nonce_bytes, ciphertext) = sealed.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.aead.decrypt(nonce, ciphertext).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_same_password() {
        let c = Cipher::from_password("hunter2");
        let sealed = c.seal(b"hello world");
        assert_eq!(c.open(&sealed).unwrap(), b"hello world");
    }

    #[test]
    fn wrong_password_fails_to_open() {
        let a = Cipher::from_password("correct horse");
        let b = Cipher::from_password("battery staple");
        let sealed = a.seal(b"secret move");
        assert!(b.open(&sealed).is_none(), "wrong password must not decrypt");
    }

    #[test]
    fn nonces_differ_across_messages() {
        let c = Cipher::from_password("pw");
        let s1 = c.seal(b"x");
        let s2 = c.seal(b"x");
        assert_ne!(s1, s2, "each frame should use a fresh random nonce");
    }
}
