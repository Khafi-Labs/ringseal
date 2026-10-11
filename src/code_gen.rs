use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use hex;
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::errors::AppError;

/// Hash a high-entropy secret (API keys). SHA-256 is fine for 256-bit keys.
pub fn hash_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}

/// Verify a high-entropy secret against its hash.
pub fn verify_secret(secret: &str, hash: &str) -> bool {
    let expected = hash_secret(secret);
    expected.as_bytes().ct_eq(hash.as_bytes()).into()
}

/// Derive a purpose-specific key from the master secret.
/// Uses HMAC-SHA256(master, label) as a one-block KDF.
pub fn derive_key(master: &[u8], label: &str) -> [u8; 32] {
    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(master).expect("HMAC can take key of any size");
    mac.update(label.as_bytes());
    let result = mac.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result.into_bytes()[..32]);
    key
}

/// Generate a 6-digit code with rejection sampling (no modulo bias).
pub fn generate_code() -> String {
    let mut rng = rand::thread_rng();
    let mut code = String::with_capacity(6);
    for _ in 0..6 {
        loop {
            let v: u8 = rng.gen();
            if v < 250 {
                code.push((b'0' + (v % 10)) as char);
                break;
            }
        }
    }
    code
}

/// Encrypt a code with AES-256-GCM. AAD = session_id bytes for binding.
pub fn encrypt_code(code: &str, master_secret: &[u8], session_id: &Uuid) -> Vec<u8> {
    let key_bytes = derive_key(master_secret, "code-encryption");
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits

    let mut aad = Vec::new();
    aad.extend_from_slice(session_id.as_bytes());

    let payload = aes_gcm::aead::Payload {
        msg: code.as_bytes(),
        aad: &aad,
    };

    let ciphertext = cipher.encrypt(&nonce, payload).expect("encryption failure");

    let mut result = Vec::with_capacity(nonce.len() + ciphertext.len());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);
    result
}

/// Decrypt a code. Returns error if tampered, wrong key, or wrong session.
pub fn decrypt_code(
    encrypted: &[u8],
    master_secret: &[u8],
    session_id: &Uuid,
) -> Result<String, AppError> {
    if encrypted.len() < 12 {
        return Err(AppError::Internal("Invalid ciphertext length".to_string()));
    }

    let key_bytes = derive_key(master_secret, "code-encryption");
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce = Nonce::from_slice(&encrypted[..12]);
    let ciphertext = &encrypted[12..];

    let mut aad = Vec::new();
    aad.extend_from_slice(session_id.as_bytes());

    let payload = aes_gcm::aead::Payload {
        msg: ciphertext,
        aad: &aad,
    };

    let decrypted_bytes = cipher
        .decrypt(nonce, payload)
        .map_err(|_| AppError::Internal("Decryption failed or tampered".to_string()))?;

    String::from_utf8(decrypted_bytes)
        .map_err(|_| AppError::Internal("Invalid UTF-8 in decrypted code".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER: &[u8] = b"0123456789abcdef0123456789abcdef";

    #[test]
    fn generated_codes_are_six_digits() {
        for _ in 0..500 {
            let code = generate_code();
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn encrypt_then_decrypt_round_trips() {
        let id = Uuid::new_v4();
        let encrypted = encrypt_code("482913", MASTER, &id);
        assert_eq!(decrypt_code(&encrypted, MASTER, &id).unwrap(), "482913");
    }

    #[test]
    fn decrypt_fails_with_wrong_associated_id() {
        let encrypted = encrypt_code("482913", MASTER, &Uuid::new_v4());
        assert!(decrypt_code(&encrypted, MASTER, &Uuid::new_v4()).is_err());
    }

    #[test]
    fn decrypt_fails_with_wrong_master_secret() {
        let id = Uuid::new_v4();
        let encrypted = encrypt_code("482913", MASTER, &id);
        assert!(decrypt_code(&encrypted, b"another-master-secret-of-32-bytes!", &id).is_err());
    }

    #[test]
    fn decrypt_fails_when_ciphertext_is_tampered() {
        let id = Uuid::new_v4();
        let mut encrypted = encrypt_code("482913", MASTER, &id);
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0x01;
        assert!(decrypt_code(&encrypted, MASTER, &id).is_err());
    }

    #[test]
    fn derived_keys_depend_on_label() {
        assert_ne!(
            derive_key(MASTER, "code-encryption"),
            derive_key(MASTER, "other")
        );
        assert_eq!(
            derive_key(MASTER, "code-encryption"),
            derive_key(MASTER, "code-encryption")
        );
    }

    #[test]
    fn secret_hash_verifies_only_the_right_secret() {
        let hash = hash_secret("rsk_example");
        assert!(verify_secret("rsk_example", &hash));
        assert!(!verify_secret("rsk_other", &hash));
    }
}
