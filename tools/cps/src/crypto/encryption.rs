///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//
///////////////////////////////////////////////////////////////////////////////
//! Multi-algorithm AEAD encryption with sr25519 key derivation.
//!
//! This module implements encryption schemes for Robonomics CPS supporting multiple AEAD ciphers:
//! - XChaCha20-Poly1305 (default, 24-byte nonce)
//! - AES-256-GCM (12-byte nonce, hardware-accelerated)
//! - ChaCha20-Poly1305 (12-byte nonce, portable)
//!
//! # Encryption Scheme
//!
//! 1. **Key Agreement**: Derive shared secret from sender's secret and receiver's public key
//! 2. **Key Derivation**: Use HKDF-SHA256 with algorithm-specific info string
//! 3. **Encryption**: AEAD cipher with random nonce
//!
//! # Message Format
//!
//! Encrypted messages are JSON-encoded with the following structure:
//!
//! ```json
//! {
//!   "version": 1,
//!   "algorithm": "xchacha20",
//!   "from": "5GrwvaEF...",
//!   "nonce": "base64-encoded-nonce",
//!   "ciphertext": "base64-encoded-data"
//! }
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use libcps::crypto::{encrypt, decrypt, EncryptionAlgorithm};
//! use sp_core::{Pair, sr25519};
//!
//! # fn example() -> anyhow::Result<()> {
//! let (sender, _) = sr25519::Pair::generate();
//! let (receiver, _) = sr25519::Pair::generate();
//! let plaintext = b"secret message";
//!
//! // Encrypt with specific algorithm
//! let encrypted = encrypt(
//!     plaintext,
//!     &sender,
//!     &receiver.public(),
//!     EncryptionAlgorithm::AesGcm256
//! )?;
//!
//! // Decrypt (algorithm auto-detected)
//! // With sender verification (recommended)
//! let decrypted = decrypt(&encrypted, &receiver, Some(&sender.public()))?;
//! assert_eq!(plaintext, &decrypted[..]);
//!
//! // Without sender verification (accepts from any sender)
//! let decrypted_any = decrypt(&encrypted, &receiver, None)?;
//! assert_eq!(plaintext, &decrypted_any[..]);
//! # Ok(())
//! # }
//! ```
//!
//! # Security
//!
//! - **AEAD**: All algorithms provide authenticated encryption
//! - **Nonce**: Random nonces from secure source (OsRng)
//! - **KDF**: HKDF-SHA256 ensures proper key derivation
//! - **Domain Separation**: Algorithm-specific info strings

use anyhow::{anyhow, Result};
use aes_gcm::{
    aead::{Aead as AesAead, AeadCore as AesAeadCore, KeyInit as AesKeyInit},
    Aes256Gcm, Nonce as AesNonce,
};
use chacha20poly1305::{
    aead::OsRng,
    ChaCha20Poly1305, Nonce as ChachaNonce, XChaCha20Poly1305, XNonce,
};
use serde::{Deserialize, Serialize};
use sp_core::crypto::Pair;

/// Encrypted message format stored on-chain.
///
/// This structure is JSON-serialized for storage and transmission.
///
/// # Fields
///
/// * `version` - Message format version (currently 1)
/// * `algorithm` - Encryption algorithm used (xchacha20, aesgcm256, or chacha20)
/// * `from` - Sender's public key in base58 encoding
/// * `nonce` - AEAD nonce in base64 encoding (size depends on algorithm)
/// * `ciphertext` - Encrypted data in base64 encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    /// Message format version
    pub version: u8,
    /// Encryption algorithm (for backward compat, defaults to xchacha20)
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
    /// Sender's sr25519 public key (base58-encoded)
    pub from: String,
    /// AEAD nonce (base64-encoded, size varies by algorithm)
    pub nonce: String,
    /// Encrypted ciphertext (base64-encoded)
    pub ciphertext: String,
}

fn default_algorithm() -> String {
    "xchacha20".to_string()
}

/// Encrypt data using ECDH key agreement and AEAD cipher.
///
/// # Process
///
/// 1. Derive shared secret using ECDH (support for both SR25519 and ED25519)
/// 2. Use HKDF-SHA256 to derive 32-byte encryption key from shared secret
/// 3. Encrypt plaintext with specified AEAD cipher using random nonce
/// 4. Return JSON-encoded message with version, algorithm, sender, nonce, and ciphertext
///
/// # Type Parameters
///
/// * `P` - Keypair type implementing `Pair` + `DeriveSharedSecret` traits
///
/// # Arguments
///
/// * `plaintext` - Data to encrypt
/// * `sender` - Sender's keypair (for signing and ECDH)
/// * `receiver_public` - Receiver's public key
/// * `algorithm` - AEAD encryption algorithm to use
///
/// # Returns
///
/// JSON-encoded [`EncryptedMessage`] with base64-encoded nonce and ciphertext
///
/// # Errors
///
/// Returns an error if:
/// - ECDH key agreement fails
/// - HKDF expansion fails
/// - Encryption fails
/// - JSON serialization fails
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{encrypt, EncryptionAlgorithm};
/// use sp_core::{Pair, sr25519};
///
/// # fn example() -> anyhow::Result<()> {
/// let (sender, _) = sr25519::Pair::generate();
/// let (receiver, _) = sr25519::Pair::generate();
/// let plaintext = b"secret message";
///
/// let encrypted = encrypt(
///     plaintext,
///     &sender,
///     &receiver.public(),
///     EncryptionAlgorithm::AesGcm256
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn encrypt<P>(
    plaintext: &[u8],
    sender: &P,
    receiver_public: &P::Public,
    algorithm: crate::crypto::EncryptionAlgorithm,
) -> Result<Vec<u8>>
where
    P: Pair + crate::crypto::DeriveSharedSecret,
{
    use base64::{engine::general_purpose, Engine as _};

    // Step 1: Derive shared secret using ECDH
    let shared_secret = <P as crate::crypto::DeriveSharedSecret>::derive(sender, receiver_public)?;
    
    // Step 2: Derive encryption key using HKDF
    let encryption_key = shared_secret.derive_encryption_key(algorithm.info_string())?;

    // Step 4: Encrypt with specified algorithm
    let (nonce_bytes, ciphertext) = match algorithm {
        crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305 => {
            let cipher = XChaCha20Poly1305::new(&encryption_key.into());
            let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
            let ct = cipher
                .encrypt(&nonce, plaintext)
                .map_err(|e| anyhow!("XChaCha20 encryption failed: {e}"))?;
            (nonce.to_vec(), ct)
        }
        crate::crypto::EncryptionAlgorithm::AesGcm256 => {
            let cipher = Aes256Gcm::new(&encryption_key.into());
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
            let ct = cipher
                .encrypt(&nonce, plaintext)
                .map_err(|e| anyhow!("AES-GCM encryption failed: {e}"))?;
            (nonce.to_vec(), ct)
        }
        crate::crypto::EncryptionAlgorithm::ChaCha20Poly1305 => {
            let cipher = ChaCha20Poly1305::new(&encryption_key.into());
            let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
            let ct = cipher
                .encrypt(&nonce, plaintext)
                .map_err(|e| anyhow!("ChaCha20 encryption failed: {e}"))?;
            (nonce.to_vec(), ct)
        }
    };

    // Step 3: Create message structure
    let sender_public = sender.public();
    let sender_public_bytes = sender_public.as_ref();
    let algorithm_str = match algorithm {
        crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305 => "xchacha20",
        crate::crypto::EncryptionAlgorithm::AesGcm256 => "aesgcm256",
        crate::crypto::EncryptionAlgorithm::ChaCha20Poly1305 => "chacha20",
    };
    
    let message = EncryptedMessage {
        version: 1,
        algorithm: algorithm_str.to_string(),
        from: bs58::encode(sender_public_bytes).into_string(),
        nonce: general_purpose::STANDARD.encode(&nonce_bytes),
        ciphertext: general_purpose::STANDARD.encode(&ciphertext),
    };

    // Step 4: Serialize to JSON
    serde_json::to_vec(&message).map_err(|e| anyhow!("JSON serialization failed: {e}"))
}

/// Decrypt data using ECDH key agreement and AEAD cipher (algorithm auto-detected).
///
/// # Process
///
/// 1. Parse JSON-encoded encrypted message
/// 2. Detect encryption algorithm from message
/// 3. Optionally verify sender's public key matches expected sender (if provided)
/// 4. Derive shared secret using ECDH (supports SR25519 and ED25519)
/// 5. Use HKDF-SHA256 to derive encryption key with algorithm-specific info
/// 6. Decrypt ciphertext with appropriate AEAD cipher
///
/// # Type Parameters
///
/// * `P` - Keypair type implementing `Pair` + `DeriveSharedSecret` traits
///
/// # Arguments
///
/// * `encrypted_data` - JSON-encoded [`EncryptedMessage`]
/// * `receiver` - Receiver's keypair
/// * `expected_sender_public` - Optional expected sender's public key for verification.
///   If `Some`, verifies the message sender matches the expected sender.
///   If `None`, skips sender verification (decrypts from any sender).
///
/// # Returns
///
/// Decrypted plaintext bytes
///
/// # Errors
///
/// Returns an error if:
/// - Cannot parse JSON message
/// - Unsupported message version or algorithm
/// - Invalid sender public key
/// - Sender public key doesn't match expected sender (when `expected_sender_public` is `Some`)
/// - ECDH key agreement fails
/// - HKDF expansion fails
/// - Cannot decode base64 nonce or ciphertext
/// - Decryption fails (wrong key or corrupted data)
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{encrypt, decrypt, EncryptionAlgorithm};
/// use sp_core::{Pair, sr25519};
///
/// # fn example() -> anyhow::Result<()> {
/// let (sender, _) = sr25519::Pair::generate();
/// let (receiver, _) = sr25519::Pair::generate();
/// let plaintext = b"secret message";
///
/// let encrypted = encrypt(plaintext, &sender, &receiver.public(), EncryptionAlgorithm::XChaCha20Poly1305)?;
///
/// // Decrypt with sender verification
/// let decrypted = decrypt(&encrypted, &receiver, Some(&sender.public()))?;
/// assert_eq!(plaintext, &decrypted[..]);
///
/// // Decrypt without sender verification (accepts from any sender)
/// let decrypted_any = decrypt(&encrypted, &receiver, None)?;
/// assert_eq!(plaintext, &decrypted_any[..]);
/// # Ok(())
/// # }
/// ```
pub fn decrypt<P>(
    encrypted_data: &[u8],
    receiver: &P,
    expected_sender_public: Option<&P::Public>,
) -> Result<Vec<u8>>
where
    P: Pair + crate::crypto::DeriveSharedSecret,
    P::Public: AsRef<[u8]> + sp_core::crypto::UncheckedFrom<[u8; 32]>,
{
    use base64::{engine::general_purpose, Engine as _};
    use std::str::FromStr;

    // Step 1: Parse message
    let message: EncryptedMessage = serde_json::from_slice(encrypted_data)
        .map_err(|e| anyhow!("Failed to parse encrypted message: {e}"))?;

    if message.version != 1 {
        return Err(anyhow!("Unsupported encryption version: {}", message.version));
    }

    // Step 2: Parse algorithm
    let algorithm = crate::crypto::EncryptionAlgorithm::from_str(&message.algorithm)
        .map_err(|e| anyhow!("Unsupported algorithm: {}", e))?;

    // Step 3: Decode and parse sender's public key
    let sender_public_bytes = bs58::decode(&message.from)
        .into_vec()
        .map_err(|e| anyhow!("Failed to decode sender public key: {e}"))?;
    
    if sender_public_bytes.len() != 32 {
        return Err(anyhow!("Invalid sender public key length: expected 32 bytes"));
    }
    
    // Convert bytes to fixed-size array
    let mut sender_pk_array = [0u8; 32];
    sender_pk_array.copy_from_slice(&sender_public_bytes);
    
    // Use UncheckedFrom to construct the public key from raw bytes
    let sender_public = <P::Public as sp_core::crypto::UncheckedFrom<[u8; 32]>>::unchecked_from(sender_pk_array);

    // Step 4: Optionally verify sender matches expected sender
    if let Some(expected_pk) = expected_sender_public {
        if sender_public.as_ref() != expected_pk.as_ref() {
            return Err(anyhow!(
                "Sender public key mismatch: message from unexpected sender"
            ));
        }
    }

    // Step 5: Derive shared secret using ECDH
    let shared_secret = <P as crate::crypto::DeriveSharedSecret>::derive(receiver, &sender_public)?;
    
    // Step 6: Derive encryption key using HKDF
    let encryption_key = shared_secret.derive_encryption_key(algorithm.info_string())?;

    // Step 7: Decode nonce and ciphertext
    let nonce_bytes = general_purpose::STANDARD
        .decode(&message.nonce)
        .map_err(|e| anyhow!("Failed to decode nonce: {e}"))?;

    let ciphertext = general_purpose::STANDARD
        .decode(&message.ciphertext)
        .map_err(|e| anyhow!("Failed to decode ciphertext: {e}"))?;

    // Step 8: Decrypt with appropriate algorithm
    match algorithm {
        crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305 => {
            if nonce_bytes.len() != 24 {
                return Err(anyhow!("Invalid XChaCha20 nonce length: expected 24 bytes"));
            }
            let nonce = XNonce::from_slice(&nonce_bytes);
            let cipher = XChaCha20Poly1305::new(&encryption_key.into());
            cipher
                .decrypt(nonce, ciphertext.as_ref())
                .map_err(|e| anyhow!("XChaCha20 decryption failed: {e}"))
        }
        crate::crypto::EncryptionAlgorithm::AesGcm256 => {
            if nonce_bytes.len() != 12 {
                return Err(anyhow!("Invalid AES-GCM nonce length: expected 12 bytes"));
            }
            let nonce = AesNonce::from_slice(&nonce_bytes);
            let cipher = Aes256Gcm::new(&encryption_key.into());
            cipher
                .decrypt(nonce, ciphertext.as_ref())
                .map_err(|e| anyhow!("AES-GCM decryption failed: {e}"))
        }
        crate::crypto::EncryptionAlgorithm::ChaCha20Poly1305 => {
            if nonce_bytes.len() != 12 {
                return Err(anyhow!("Invalid ChaCha20 nonce length: expected 12 bytes"));
            }
            let nonce = ChachaNonce::from_slice(&nonce_bytes);
            let cipher = ChaCha20Poly1305::new(&encryption_key.into());
            cipher
                .decrypt(nonce, ciphertext.as_ref())
                .map_err(|e| anyhow!("ChaCha20 decryption failed: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::EncryptionAlgorithm;
    use schnorrkel::{Keypair, MiniSecretKey};
    
    /// Generate a test keypair from a seed
    fn test_keypair(seed: u8) -> Keypair {
        let mini_secret = MiniSecretKey::from_bytes(&[seed; 32]).unwrap();
        mini_secret.expand_to_keypair(schnorrkel::ExpansionMode::Ed25519)
    }
    
    #[test]
    fn test_shared_secret_creation() {
        // Create two keypairs
        let alice = test_keypair(1);
        let bob = test_keypair(2);
        
        // Derive shared secrets from both sides
        let shared_alice_bob = SharedSecret::new(&alice.secret, &bob.public).unwrap();
        let shared_bob_alice = SharedSecret::new(&bob.secret, &alice.public).unwrap();
        
        // Shared secrets should be identical
        assert_eq!(shared_alice_bob.as_bytes(), shared_bob_alice.as_bytes());
        
        // Shared secret should be 32 bytes
        assert_eq!(shared_alice_bob.as_bytes().len(), 32);
        
        // Shared secret should not be all zeros
        assert_ne!(shared_alice_bob.as_bytes(), &[0u8; 32]);
    }
    
    #[test]
    fn test_shared_secret_different_pairs() {
        // Create three keypairs
        let alice = test_keypair(1);
        let bob = test_keypair(2);
        let charlie = test_keypair(3);
        
        // Derive different shared secrets
        let shared_alice_bob = SharedSecret::new(&alice.secret, &bob.public).unwrap();
        let shared_alice_charlie = SharedSecret::new(&alice.secret, &charlie.public).unwrap();
        
        // Different pairs should produce different shared secrets
        assert_ne!(shared_alice_bob.as_bytes(), shared_alice_charlie.as_bytes());
    }
    
    #[test]
    fn test_derive_encryption_key() {
        let alice = test_keypair(1);
        let bob = test_keypair(2);
        let shared_secret = SharedSecret::new(&alice.secret, &bob.public).unwrap();
        
        // Derive encryption key twice
        let key1 = shared_secret.derive_encryption_key(crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305).unwrap();
        let key2 = shared_secret.derive_encryption_key(crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305).unwrap();
        
        // Same shared secret should produce same key
        assert_eq!(key1, key2);
        
        // Key should be 32 bytes
        assert_eq!(key1.len(), 32);
        
        // Key should be different from shared secret (HKDF transforms it)
        assert_ne!(&key1, shared_secret.as_bytes());
    }
    
    #[test]
    fn test_derive_encryption_key_different_secrets() {
        let alice = test_keypair(1);
        let bob = test_keypair(2);
        let charlie = test_keypair(3);
        
        let shared_secret1 = SharedSecret::new(&alice.secret, &bob.public).unwrap();
        let shared_secret2 = SharedSecret::new(&alice.secret, &charlie.public).unwrap();
        
        let key1 = shared_secret1.derive_encryption_key(crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305).unwrap();
        let key2 = shared_secret2.derive_encryption_key(crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305).unwrap();
        
        // Different shared secrets should produce different keys
        assert_ne!(key1, key2);
    }
    
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        // Create sender and receiver keypairs
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        
        let plaintext = b"Hello, Robonomics CPS!";
        
        // Encrypt
        let encrypted = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            EncryptionAlgorithm::default(),
        ).unwrap();
        
        // Encrypted data should not be empty
        assert!(!encrypted.is_empty());
        
        // Encrypted data should be valid JSON
        let _: EncryptedMessage = serde_json::from_slice(&encrypted).unwrap();
        
        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret, Some(&sender.public.to_bytes())).unwrap();
        
        // Decrypted should match original plaintext
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Same message";
        
        // Encrypt same message twice
        let encrypted1 = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes(), EncryptionAlgorithm::default()).unwrap();
        let encrypted2 = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes(), EncryptionAlgorithm::default()).unwrap();
        
        // Should produce different ciphertexts due to random nonces
        assert_ne!(encrypted1, encrypted2);
        
        // But both should decrypt to the same plaintext
        let decrypted1 = decrypt(&encrypted1, &receiver.secret, Some(&sender.public.to_bytes())).unwrap();
        let decrypted2 = decrypt(&encrypted2, &receiver.secret, Some(&sender.public.to_bytes())).unwrap();
        assert_eq!(decrypted1, plaintext);
        assert_eq!(decrypted2, plaintext);
    }
    
    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let wrong_receiver = test_keypair(3);
        
        let plaintext = b"Secret message";
        
        // Encrypt for receiver
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes(), EncryptionAlgorithm::default()).unwrap();
        
        // Try to decrypt with wrong key
        let result = decrypt(&encrypted, &wrong_receiver.secret, Some(&sender.public.to_bytes()));
        
        // Should fail
        assert!(result.is_err());
    }
    
    #[test]
    fn test_decrypt_with_corrupted_data_fails() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test message";
        
        // Encrypt
        let mut encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes(), EncryptionAlgorithm::default()).unwrap();
        
        // Corrupt the data
        if encrypted.len() > 10 {
            encrypted[10] ^= 0xFF;
        }
        
        // Try to decrypt corrupted data
        let result = decrypt(&encrypted, &receiver.secret, Some(&sender.public.to_bytes()));
        
        // Should fail (either parse error or authentication failure)
        assert!(result.is_err());
    }
    
    #[test]
    fn test_encrypt_empty_message() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"";
        
        // Encrypt empty message
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes(), EncryptionAlgorithm::default()).unwrap();
        
        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret, Some(&sender.public.to_bytes())).unwrap();
        
        // Should get empty message back
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_encrypt_large_message() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = vec![42u8; 10000]; // 10KB message
        
        // Encrypt
        let encrypted = encrypt(&plaintext, &sender.secret, &receiver.public.to_bytes(), EncryptionAlgorithm::default()).unwrap();
        
        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret, Some(&sender.public.to_bytes())).unwrap();
        
        // Should match
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_encrypted_message_structure() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test";
        
        // Encrypt
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes(), EncryptionAlgorithm::default()).unwrap();
        
        // Parse the encrypted message
        let message: EncryptedMessage = serde_json::from_slice(&encrypted).unwrap();
        
        // Check version
        assert_eq!(message.version, 1);
        
        // Check sender public key is encoded
        assert!(!message.from.is_empty());
        let decoded_from = bs58::decode(&message.from).into_vec().unwrap();
        assert_eq!(decoded_from.len(), 32);
        
        // Check nonce is base64 encoded and correct size
        use base64::{engine::general_purpose, Engine as _};
        let nonce = general_purpose::STANDARD.decode(&message.nonce).unwrap();
        assert_eq!(nonce.len(), 24); // XChaCha20 nonce size
        
        // Check ciphertext is base64 encoded
        let ciphertext = general_purpose::STANDARD.decode(&message.ciphertext).unwrap();
        assert!(!ciphertext.is_empty());
    }
    
    #[test]
    fn test_decrypt_rejects_wrong_version() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test";
        
        // Encrypt
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes(), EncryptionAlgorithm::default()).unwrap();
        
        // Parse and modify version
        let mut message: EncryptedMessage = serde_json::from_slice(&encrypted).unwrap();
        message.version = 2;
        let modified = serde_json::to_vec(&message).unwrap();
        
        // Try to decrypt
        let result = decrypt(&modified, &receiver.secret, Some(&sender.public.to_bytes()));
        
        // Should fail due to unsupported version
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported encryption version"));
    }

    #[test]
    fn test_decrypt_rejects_wrong_sender() {
        // Create three keypairs
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let wrong_sender = test_keypair(3);
        
        let plaintext = b"Test message";
        
        // Encrypt from sender to receiver
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes(), EncryptionAlgorithm::default()).unwrap();
        
        // Try to decrypt with wrong expected sender
        let result = decrypt(&encrypted, &receiver.secret, Some(&wrong_sender.public.to_bytes()));
        
        // Should fail due to sender mismatch
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Sender public key mismatch"));
    }

    #[test]
    fn test_decrypt_without_sender_verification() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        
        let plaintext = b"Test message";
        
        // Encrypt from sender to receiver
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes(), EncryptionAlgorithm::default()).unwrap();
        
        // Decrypt without sender verification (None)
        let decrypted = decrypt(&encrypted, &receiver.secret, None).unwrap();
        
        // Should successfully decrypt without checking sender
        assert_eq!(decrypted, plaintext);
    }

    // ========== AES-GCM-256 Algorithm Tests ==========

    #[test]
    fn test_aesgcm256_encrypt_decrypt_roundtrip() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test message for AES-GCM-256";

        // Encrypt using AES-GCM-256
        let encrypted = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();

        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret, Some(&sender.public.to_bytes())).unwrap();

        // Should match
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aesgcm256_message_structure() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test AES-GCM";

        // Encrypt using AES-GCM-256
        let encrypted = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();

        // Parse the encrypted message
        let message: EncryptedMessage = serde_json::from_slice(&encrypted).unwrap();

        // Check algorithm field
        assert_eq!(message.algorithm, "aesgcm256");

        // Check nonce is correct size for AES-GCM (12 bytes)
        use base64::{engine::general_purpose, Engine as _};
        let nonce = general_purpose::STANDARD.decode(&message.nonce).unwrap();
        assert_eq!(nonce.len(), 12);
    }

    #[test]
    fn test_aesgcm256_empty_message() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"";

        // Encrypt empty message with AES-GCM-256
        let encrypted = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();

        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret, Some(&sender.public.to_bytes())).unwrap();

        // Should get empty message back
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aesgcm256_large_message() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = vec![0xAB; 50000]; // 50KB message

        // Encrypt with AES-GCM-256
        let encrypted = encrypt(
            &plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();

        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret, Some(&sender.public.to_bytes())).unwrap();

        // Should match
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aesgcm256_wrong_key_fails() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let wrong_receiver = test_keypair(3);

        let plaintext = b"Secret AES-GCM message";

        // Encrypt for receiver using AES-GCM-256
        let encrypted = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();

        // Try to decrypt with wrong key
        let result = decrypt(&encrypted, &wrong_receiver.secret, Some(&sender.public.to_bytes()));

        // Should fail (authentication error)
        assert!(result.is_err());
    }

    #[test]
    fn test_aesgcm256_corrupted_data_fails() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"AES-GCM test message";

        // Encrypt with AES-GCM-256
        let mut encrypted = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();

        // Corrupt the ciphertext
        if encrypted.len() > 20 {
            encrypted[20] ^= 0xFF;
        }

        // Try to decrypt corrupted data
        let result = decrypt(&encrypted, &receiver.secret, Some(&sender.public.to_bytes()));

        // Should fail (authentication tag will not verify)
        assert!(result.is_err());
    }

    #[test]
    fn test_aesgcm256_produces_different_ciphertexts() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Same AES-GCM message";

        // Encrypt same message twice with AES-GCM-256
        let encrypted1 = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();
        let encrypted2 = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();

        // Should produce different ciphertexts due to random nonces
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same plaintext
        let decrypted1 = decrypt(&encrypted1, &receiver.secret, Some(&sender.public.to_bytes())).unwrap();
        let decrypted2 = decrypt(&encrypted2, &receiver.secret, Some(&sender.public.to_bytes())).unwrap();
        assert_eq!(decrypted1, plaintext);
        assert_eq!(decrypted2, plaintext);
    }

    #[test]
    fn test_aesgcm256_nonce_size_validation() {
        use base64::{engine::general_purpose, Engine as _};
        
        // This test verifies that decrypt properly validates AES-GCM nonce size
        let sender = test_keypair(1);
        let receiver = test_keypair(2);

        // Create a message with invalid nonce size
        let message = EncryptedMessage {
            version: 1,
            algorithm: "aesgcm256".to_string(),
            from: bs58::encode(sender.public.to_bytes()).into_string(),
            nonce: general_purpose::STANDARD.encode([0u8; 24]), // Wrong size (should be 12)
            ciphertext: general_purpose::STANDARD.encode(b"fake"),
        };

        let encrypted = serde_json::to_vec(&message).unwrap();

        // Try to decrypt
        let result = decrypt(&encrypted, &receiver.secret, Some(&sender.public.to_bytes()));

        // Should fail due to invalid nonce size
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid AES-GCM nonce length"));
    }

    // ========== ChaCha20-Poly1305 Algorithm Tests ==========

    #[test]
    fn test_chacha20poly1305_encrypt_decrypt_roundtrip() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test message for ChaCha20-Poly1305";

        // Encrypt using ChaCha20-Poly1305
        let encrypted = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::ChaCha20Poly1305,
        )
        .unwrap();

        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret, Some(&sender.public.to_bytes())).unwrap();

        // Should match
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_chacha20poly1305_message_structure() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test ChaCha20";

        // Encrypt using ChaCha20-Poly1305
        let encrypted = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::ChaCha20Poly1305,
        )
        .unwrap();

        // Parse the encrypted message
        let message: EncryptedMessage = serde_json::from_slice(&encrypted).unwrap();

        // Check algorithm field
        assert_eq!(message.algorithm, "chacha20");

        // Check nonce is correct size for ChaCha20 (12 bytes)
        use base64::{engine::general_purpose, Engine as _};
        let nonce = general_purpose::STANDARD.decode(&message.nonce).unwrap();
        assert_eq!(nonce.len(), 12);
    }

    // ========== Cross-Algorithm Compatibility Tests ==========

    #[test]
    fn test_different_algorithms_produce_different_outputs() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Cross-algorithm test";

        // Encrypt with all three algorithms
        let xchacha = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();

        let aesgcm = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();

        let chacha = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
            crate::crypto::EncryptionAlgorithm::ChaCha20Poly1305,
        )
        .unwrap();

        // All should be different due to different algorithms and nonces
        assert_ne!(xchacha, aesgcm);
        assert_ne!(xchacha, chacha);
        assert_ne!(aesgcm, chacha);

        // But all should decrypt correctly
        assert_eq!(decrypt(&xchacha, &receiver.secret, Some(&sender.public.to_bytes())).unwrap(), plaintext);
        assert_eq!(decrypt(&aesgcm, &receiver.secret, Some(&sender.public.to_bytes())).unwrap(), plaintext);
        assert_eq!(decrypt(&chacha, &receiver.secret, Some(&sender.public.to_bytes())).unwrap(), plaintext);
    }
}
