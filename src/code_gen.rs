use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::{Sha256, Digest};
use uuid::Uuid;
use subtle::ConstantTimeEq;
use hex;

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
    let mut mac = Hmac::<Sha256>::new_from_slice(master).expect("HMAC can take key of any size");
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
pub fn decrypt_code(encrypted: &[u8], master_secret: &[u8], session_id: &Uuid) -> Result<String, AppError> {
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
    
    let decrypted_bytes = cipher.decrypt(nonce, payload)
        .map_err(|_| AppError::Internal("Decryption failed or tampered".to_string()))?;
        
    String::from_utf8(decrypted_bytes).map_err(|_| AppError::Internal("Invalid UTF-8 in decrypted code".to_string()))
}
