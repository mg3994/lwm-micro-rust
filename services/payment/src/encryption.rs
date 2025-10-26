use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key,
};
use base64::{Engine as _, engine::general_purpose};

use linkwithmentor_common::AppError;

#[derive(Clone)]
pub struct EncryptionService {
    cipher: Aes256Gcm,
}

impl EncryptionService {
    pub fn new(key: &str) -> Result<Self, AppError> {
        // Ensure key is 32 bytes for AES-256
        let key_bytes = if key.len() >= 32 {
            &key.as_bytes()[..32]
        } else {
            return Err(AppError::Internal("Encryption key must be at least 32 characters".to_string()));
        };

        let key = Key::<Aes256Gcm>::from_slice(key_bytes);
        let cipher = Aes256Gcm::new(key);

        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, AppError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self.cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| AppError::Internal(format!("Encryption failed: {}", e)))?;

        // Combine nonce and ciphertext
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(general_purpose::STANDARD.encode(&result))
    }

    pub fn decrypt(&self, encrypted_data: &str) -> Result<String, AppError> {
        let data = general_purpose::STANDARD
            .decode(encrypted_data)
            .map_err(|e| AppError::Internal(format!("Base64 decode failed: {}", e)))?;

        if data.len() < 12 {
            return Err(AppError::Internal("Invalid encrypted data".to_string()));
        }

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| AppError::Internal(format!("UTF-8 conversion failed: {}", e)))
    }

    pub fn encrypt_payment_method(&self, payment_method: &crate::models::PaymentMethodDetails) -> Result<String, AppError> {
        let json = serde_json::to_string(payment_method)
            .map_err(|e| AppError::Internal(format!("Serialization failed: {}", e)))?;
        self.encrypt(&json)
    }

    pub fn decrypt_payment_method(&self, encrypted_data: &str) -> Result<crate::models::PaymentMethodDetails, AppError> {
        let json = self.decrypt(encrypted_data)?;
        serde_json::from_str(&json)
            .map_err(|e| AppError::Internal(format!("Deserialization failed: {}", e)))
    }
}