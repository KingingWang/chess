//! Password-keyed symmetric encryption for the game link.
//!
//! Every wire frame is sealed with **ChaCha20-Poly1305** (an AEAD cipher) using
//! a 256-bit key derived from the room password. Because the cipher is
//! authenticated, a peer that supplies the wrong password cannot decrypt the
//! very first handshake frame — so the password doubles as access control:
//! a mismatch surfaces as a decryption error and the connection is refused.
//!
//! The key is derived with **Argon2id** (a slow, memory-hard KDF) over the
//! password and a per-room random `salt`. The salt is *not* secret: it is
//! exchanged in the clear (a plaintext prelude on LAN, or via the relay server
//! for internet play) so both peers derive the same key. Argon2id makes brute
//! forcing a weak password from captured ciphertext far more expensive than a
//! plain hash would. The relay server never sees the password or the key, so
//! game data stays end-to-end encrypted.
//!
//! Wire layout of one sealed frame (before transport framing):
//! `nonce (12 bytes) || ciphertext+tag`.

use argon2::Argon2;
use chacha20poly1305::aead::{Aead, AeadCore, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce};

const NONCE_LEN: usize = 12;
/// Length of the per-room random salt fed to the KDF.
pub const SALT_LEN: usize = 16;

/// Generate a fresh random salt for a new room (host side).
pub fn random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    getrandom::fill(&mut salt).expect("system RNG must be available");
    salt
}

/// A symmetric cipher keyed by the room password and salt.
#[derive(Clone)]
pub struct Cipher {
    aead: ChaCha20Poly1305,
}

impl Cipher {
    /// Derive the per-room key from `password` and `salt` using Argon2id.
    ///
    /// Both peers must use the same password and salt to interoperate; a
    /// mismatch yields a different key and every frame fails to authenticate.
    pub fn from_password_salt(password: &str, salt: &[u8]) -> Cipher {
        let mut key_bytes = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), salt, &mut key_bytes)
            .expect("Argon2id KDF is infallible for a 32-byte output");
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

    const SALT: &[u8] = b"0123456789abcdef";

    #[test]
    fn roundtrip_same_password() {
        let c = Cipher::from_password_salt("hunter2", SALT);
        let sealed = c.seal(b"hello world");
        assert_eq!(c.open(&sealed).unwrap(), b"hello world");
    }

    #[test]
    fn wrong_password_fails_to_open() {
        let a = Cipher::from_password_salt("correct horse", SALT);
        let b = Cipher::from_password_salt("battery staple", SALT);
        let sealed = a.seal(b"secret move");
        assert!(b.open(&sealed).is_none(), "wrong password must not decrypt");
    }

    #[test]
    fn different_salt_yields_different_key() {
        let a = Cipher::from_password_salt("pw", b"0123456789abcdef");
        let b = Cipher::from_password_salt("pw", b"fedcba9876543210");
        let sealed = a.seal(b"x");
        assert!(b.open(&sealed).is_none(), "different salt must not decrypt");
    }

    #[test]
    fn nonces_differ_across_messages() {
        let c = Cipher::from_password_salt("pw", SALT);
        let s1 = c.seal(b"x");
        let s2 = c.seal(b"x");
        assert_ne!(s1, s2, "each frame should use a fresh random nonce");
    }

    #[test]
    fn random_salt_is_nonzero_and_unique() {
        let a = random_salt();
        let b = random_salt();
        assert_ne!(a, b, "salts should differ across rooms");
    }
}
