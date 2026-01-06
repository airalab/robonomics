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
//! Consolidated cipher implementation with direct ECDH operations.
//!
//! This module provides a simplified `Cypher` struct that stores only the
//! 32-byte secret key and implements encryption/decryption directly without
//! depending on the full `sp_core::Pair` trait.

use aes_gcm::{
    aead::{Aead as AesAead, AeadCore as AesAeadCore, KeyInit as AesKeyInit},
    Aes256Gcm, Nonce as AesNonce,
};
use anyhow::{anyhow, Result};
use chacha20poly1305::{
    aead::OsRng, ChaCha20Poly1305, Nonce as ChachaNonce, XChaCha20Poly1305, XNonce,
};
use serde::{Deserialize, Serialize};
use sp_core::crypto::{Pair, UncheckedFrom};
use std::str::FromStr;

use crate::crypto::{EncryptedMessage, EncryptionAlgorithm, KeypairType};

/// Cryptographic scheme for ECDH operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoScheme {
    /// SR25519 (Schnorrkel/Ristretto) scheme
    Sr25519,
    /// ED25519 (Edwards curve) scheme
    Ed25519,
}

impl CryptoScheme {
    /// Get the info suffix for HKDF based on the scheme
    pub fn info_suffix(&self) -> &'static str {
        match self {
            CryptoScheme::Sr25519 => "sr25519",
            CryptoScheme::Ed25519 => "ed25519",
        }
    }
}

/// Simplified cipher that stores only the 32-byte secret key.
///
/// This struct consolidates encryption/decryption logic and performs ECDH
/// operations directly using low-level crypto crates without requiring the
/// full `sp_core::Pair` trait overhead.
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{Cypher, EncryptionAlgorithm, KeypairType};
///
/// # fn example() -> anyhow::Result<()> {
/// // Create from SURI
/// let cypher = Cypher::new("//Alice", KeypairType::Sr25519, EncryptionAlgorithm::XChaCha20Poly1305)?;
///
/// // Encrypt for a receiver
/// let receiver_public = [0u8; 32]; // Receiver's public key
/// let plaintext = b"Secret message";
/// let encrypted = cypher.encrypt(plaintext, &receiver_public)?;
///
/// // Decrypt (receiver side)
/// let decrypted = cypher.decrypt(&encrypted, Some(&cypher.derive_public_key()?))?;
/// assert_eq!(plaintext, &decrypted[..]);
/// # Ok(())
/// # }
/// ```
pub struct Cypher {
    /// 32-byte secret key
    secret: [u8; 32],
    /// Encryption algorithm to use
    algorithm: EncryptionAlgorithm,
    /// Cryptographic scheme (Sr25519 or Ed25519)
    scheme: CryptoScheme,
}

impl Cypher {
    /// Create a new Cypher from a SURI string.
    ///
    /// Parses the SURI to extract the 32-byte secret key, then drops the Pair.
    ///
    /// # Arguments
    ///
    /// * `suri` - Secret URI (e.g., "//Alice", mnemonic, or hex seed)
    /// * `keypair_type` - Type of keypair (Sr25519 or Ed25519)
    /// * `algorithm` - Encryption algorithm to use
    ///
    /// # Returns
    ///
    /// New `Cypher` instance with extracted secret key
    ///
    /// # Errors
    ///
    /// Returns error if SURI parsing fails
    pub fn new(
        suri: &str,
        keypair_type: KeypairType,
        algorithm: EncryptionAlgorithm,
    ) -> Result<Self> {
        match keypair_type {
            KeypairType::Sr25519 => {
                let pair = sp_core::sr25519::Pair::from_string(suri, None)
                    .map_err(|e| anyhow!("Failed to parse SR25519 SURI: {:?}", e))?;
                let secret_bytes = pair.to_raw_vec();
                let mut secret = [0u8; 32];
                secret.copy_from_slice(&secret_bytes[..32]);
                Ok(Self {
                    secret,
                    algorithm,
                    scheme: CryptoScheme::Sr25519,
                })
            }
            KeypairType::Ed25519 => {
                let pair = sp_core::ed25519::Pair::from_string(suri, None)
                    .map_err(|e| anyhow!("Failed to parse ED25519 SURI: {:?}", e))?;
                let secret_bytes = pair.to_raw_vec();
                let mut secret = [0u8; 32];
                secret.copy_from_slice(&secret_bytes[..32]);
                Ok(Self {
                    secret,
                    algorithm,
                    scheme: CryptoScheme::Ed25519,
                })
            }
        }
    }

    /// Create a Cypher directly from a 32-byte seed.
    ///
    /// # Arguments
    ///
    /// * `seed` - 32-byte secret seed
    /// * `keypair_type` - Type of keypair (Sr25519 or Ed25519)
    /// * `algorithm` - Encryption algorithm to use
    pub fn from_seed(
        seed: [u8; 32],
        keypair_type: KeypairType,
        algorithm: EncryptionAlgorithm,
    ) -> Self {
        let scheme = match keypair_type {
            KeypairType::Sr25519 => CryptoScheme::Sr25519,
            KeypairType::Ed25519 => CryptoScheme::Ed25519,
        };
        Self {
            secret: seed,
            algorithm,
            scheme,
        }
    }

    /// Derive the public key from the secret key.
    ///
    /// This is used for metadata and message formatting.
    ///
    /// # Returns
    ///
    /// 32-byte public key corresponding to the secret
    ///
    /// # Errors
    ///
    /// Returns error if public key derivation fails
    pub fn derive_public_key(&self) -> Result<[u8; 32]> {
        match self.scheme {
            CryptoScheme::Sr25519 => {
                let pair = sp_core::sr25519::Pair::from_seed(&self.secret);
                let public = pair.public();
                Ok(public.0)
            }
            CryptoScheme::Ed25519 => {
                let pair = sp_core::ed25519::Pair::from_seed(&self.secret);
                let public = pair.public();
                Ok(public.0)
            }
        }
    }

    /// Derive a shared secret using ECDH.
    ///
    /// Performs elliptic curve Diffie-Hellman key agreement:
    /// - For Sr25519: Uses Ristretto255 curve with Schnorrkel
    /// - For Ed25519: Uses X25519 via Montgomery conversion
    ///
    /// # Arguments
    ///
    /// * `receiver_public` - Receiver's 32-byte public key
    ///
    /// # Returns
    ///
    /// 32-byte shared secret
    ///
    /// # Errors
    ///
    /// Returns error if ECDH computation fails (e.g., invalid public key)
    fn derive_shared_secret(&self, receiver_public: &[u8; 32]) -> Result<[u8; 32]> {
        match self.scheme {
            CryptoScheme::Sr25519 => {
                use curve25519_dalek::ristretto::CompressedRistretto;
                use curve25519_dalek::scalar::Scalar;
                use sha2::{Digest, Sha512};

                // Create scalar from secret key
                let scalar = Scalar::from_bytes_mod_order(self.secret);

                // Decompress receiver's public key
                let public_compressed = CompressedRistretto(*receiver_public);
                let public_point = public_compressed
                    .decompress()
                    .ok_or_else(|| anyhow!("Failed to decompress Ristretto255 public key"))?;

                // Perform scalar multiplication: shared_point = scalar * public_point
                let shared_point = scalar * public_point;

                // Compress and hash the result for uniform distribution
                let shared_compressed = shared_point.compress();

                let mut hasher = Sha512::new();
                hasher.update(b"robonomics-cps-ecdh");
                hasher.update(shared_compressed.as_bytes());
                let hash_output = hasher.finalize();

                let mut result = [0u8; 32];
                result.copy_from_slice(&hash_output[..32]);
                Ok(result)
            }
            CryptoScheme::Ed25519 => {
                use curve25519_dalek::edwards::CompressedEdwardsY;
                use sha2::{Digest, Sha512};

                // Hash and clamp the secret for X25519
                let mut hasher = Sha512::new();
                hasher.update(self.secret);
                let hash = hasher.finalize();

                let mut scalar_bytes = [0u8; 32];
                scalar_bytes.copy_from_slice(&hash[..32]);

                // Clamp the scalar for X25519
                scalar_bytes[0] &= 248;
                scalar_bytes[31] &= 127;
                scalar_bytes[31] |= 64;

                let my_x25519_secret = x25519_dalek::StaticSecret::from(scalar_bytes);

                // Convert Ed25519 public key to X25519
                let compressed_edwards = CompressedEdwardsY(*receiver_public);
                let edwards_point = compressed_edwards
                    .decompress()
                    .ok_or_else(|| anyhow!("Failed to decompress ED25519 public key"))?;

                let montgomery_point = edwards_point.to_montgomery();
                let their_x25519_public = x25519_dalek::PublicKey::from(montgomery_point.to_bytes());

                // Perform X25519 ECDH
                let shared_secret = my_x25519_secret.diffie_hellman(&their_x25519_public);
                Ok(*shared_secret.as_bytes())
            }
        }
    }

    /// Derive an encryption key from the shared secret using HKDF-SHA256.
    ///
    /// # Arguments
    ///
    /// * `shared_secret` - 32-byte shared secret from ECDH
    ///
    /// # Returns
    ///
    /// 32-byte encryption key suitable for the configured algorithm
    ///
    /// # Errors
    ///
    /// Returns error if HKDF expansion fails
    fn derive_encryption_key(&self, shared_secret: &[u8; 32]) -> Result<[u8; 32]> {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let info = self.algorithm.info_string();
        let hkdf = Hkdf::<Sha256>::new(None, shared_secret);
        let mut encryption_key = [0u8; 32];
        hkdf.expand(info, &mut encryption_key)
            .map_err(|e| anyhow!("HKDF expansion failed: {e}"))?;
        Ok(encryption_key)
    }

    /// Encrypt plaintext for a receiver.
    ///
    /// # Process
    ///
    /// 1. Derive shared secret using ECDH
    /// 2. Derive encryption key using HKDF-SHA256
    /// 3. Encrypt with configured AEAD cipher (XChaCha20/AES-GCM/ChaCha20)
    /// 4. Format as JSON-encoded `EncryptedMessage`
    ///
    /// # Arguments
    ///
    /// * `plaintext` - Data to encrypt
    /// * `receiver_public` - Receiver's 32-byte public key
    ///
    /// # Returns
    ///
    /// JSON-encoded encrypted message with version, algorithm, sender, nonce, and ciphertext
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - ECDH key agreement fails
    /// - HKDF expansion fails
    /// - Encryption fails
    /// - JSON serialization fails
    pub fn encrypt(&self, plaintext: &[u8], receiver_public: &[u8; 32]) -> Result<Vec<u8>> {
        use base64::{engine::general_purpose, Engine as _};

        // Step 1: Derive shared secret
        let shared_secret = self.derive_shared_secret(receiver_public)?;

        // Step 2: Derive encryption key
        let encryption_key = self.derive_encryption_key(&shared_secret)?;

        // Step 3: Encrypt with specified algorithm
        let (nonce_bytes, ciphertext) = match self.algorithm {
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                let cipher = XChaCha20Poly1305::new(&encryption_key.into());
                let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
                let ct = cipher
                    .encrypt(&nonce, plaintext)
                    .map_err(|e| anyhow!("XChaCha20 encryption failed: {e}"))?;
                (nonce.to_vec(), ct)
            }
            EncryptionAlgorithm::AesGcm256 => {
                let cipher = Aes256Gcm::new(&encryption_key.into());
                let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
                let ct = cipher
                    .encrypt(&nonce, plaintext)
                    .map_err(|e| anyhow!("AES-GCM encryption failed: {e}"))?;
                (nonce.to_vec(), ct)
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(&encryption_key.into());
                let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
                let ct = cipher
                    .encrypt(&nonce, plaintext)
                    .map_err(|e| anyhow!("ChaCha20 encryption failed: {e}"))?;
                (nonce.to_vec(), ct)
            }
        };

        // Step 4: Get sender's public key
        let sender_public = self.derive_public_key()?;

        // Step 5: Determine algorithm string for message
        let algorithm_str = match self.algorithm {
            EncryptionAlgorithm::XChaCha20Poly1305 => "xchacha20",
            EncryptionAlgorithm::AesGcm256 => "aesgcm256",
            EncryptionAlgorithm::ChaCha20Poly1305 => "chacha20",
        };

        // Step 6: Create message structure
        let message = EncryptedMessage {
            version: 1,
            algorithm: algorithm_str.to_string(),
            from: bs58::encode(sender_public).into_string(),
            nonce: general_purpose::STANDARD.encode(&nonce_bytes),
            ciphertext: general_purpose::STANDARD.encode(&ciphertext),
        };

        // Step 7: Serialize to JSON
        serde_json::to_vec(&message).map_err(|e| anyhow!("JSON serialization failed: {e}"))
    }

    /// Decrypt an encrypted message.
    ///
    /// # Process
    ///
    /// 1. Parse JSON-encoded message
    /// 2. Verify sender (if provided)
    /// 3. Derive shared secret using ECDH
    /// 4. Derive encryption key using HKDF-SHA256
    /// 5. Decrypt with appropriate AEAD cipher
    ///
    /// # Arguments
    ///
    /// * `encrypted_data` - JSON-encoded `EncryptedMessage`
    /// * `expected_sender_public` - Optional expected sender's public key.
    ///   If `Some`, verifies the message sender matches the expected sender.
    ///   If `None`, skips sender verification.
    ///
    /// # Returns
    ///
    /// Decrypted plaintext bytes
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Cannot parse JSON message
    /// - Unsupported message version or algorithm
    /// - Invalid sender public key
    /// - Sender doesn't match expected sender (when provided)
    /// - ECDH key agreement fails
    /// - HKDF expansion fails
    /// - Cannot decode base64 data
    /// - Decryption fails (wrong key or corrupted data)
    pub fn decrypt(
        &self,
        encrypted_data: &[u8],
        expected_sender_public: Option<&[u8; 32]>,
    ) -> Result<Vec<u8>> {
        use base64::{engine::general_purpose, Engine as _};

        // Step 1: Parse message
        let message: EncryptedMessage = serde_json::from_slice(encrypted_data)
            .map_err(|e| anyhow!("Failed to parse encrypted message: {e}"))?;

        if message.version != 1 {
            return Err(anyhow!(
                "Unsupported encryption version: {}",
                message.version
            ));
        }

        // Step 2: Parse algorithm
        let algorithm = EncryptionAlgorithm::from_str(&message.algorithm)
            .map_err(|e| anyhow!("Unsupported algorithm: {e}"))?;

        // Step 3: Decode sender's public key
        let sender_public_bytes = bs58::decode(&message.from)
            .into_vec()
            .map_err(|e| anyhow!("Failed to decode sender public key: {e}"))?;

        if sender_public_bytes.len() != 32 {
            return Err(anyhow!(
                "Invalid sender public key length: expected 32 bytes"
            ));
        }

        let mut sender_public = [0u8; 32];
        sender_public.copy_from_slice(&sender_public_bytes);

        // Step 4: Optionally verify sender
        if let Some(expected_pk) = expected_sender_public {
            if &sender_public != expected_pk {
                return Err(anyhow!(
                    "Sender public key mismatch: message from unexpected sender"
                ));
            }
        }

        // Step 5: Derive shared secret
        let shared_secret = self.derive_shared_secret(&sender_public)?;

        // Step 6: Derive encryption key with the message's algorithm
        let info = algorithm.info_string();
        let hkdf = hkdf::Hkdf::<sha2::Sha256>::new(None, &shared_secret);
        let mut encryption_key = [0u8; 32];
        hkdf.expand(info, &mut encryption_key)
            .map_err(|e| anyhow!("HKDF expansion failed: {e}"))?;

        // Step 7: Decode nonce and ciphertext
        let nonce_bytes = general_purpose::STANDARD
            .decode(&message.nonce)
            .map_err(|e| anyhow!("Failed to decode nonce: {e}"))?;

        let ciphertext = general_purpose::STANDARD
            .decode(&message.ciphertext)
            .map_err(|e| anyhow!("Failed to decode ciphertext: {e}"))?;

        // Step 8: Decrypt with appropriate algorithm
        match algorithm {
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                if nonce_bytes.len() != 24 {
                    return Err(anyhow!("Invalid XChaCha20 nonce length: expected 24 bytes"));
                }
                let nonce = XNonce::from_slice(&nonce_bytes);
                let cipher = XChaCha20Poly1305::new(&encryption_key.into());
                cipher
                    .decrypt(nonce, ciphertext.as_ref())
                    .map_err(|e| anyhow!("XChaCha20 decryption failed: {e}"))
            }
            EncryptionAlgorithm::AesGcm256 => {
                if nonce_bytes.len() != 12 {
                    return Err(anyhow!("Invalid AES-GCM nonce length: expected 12 bytes"));
                }
                let nonce = AesNonce::from_slice(&nonce_bytes);
                let cipher = Aes256Gcm::new(&encryption_key.into());
                cipher
                    .decrypt(nonce, ciphertext.as_ref())
                    .map_err(|e| anyhow!("AES-GCM decryption failed: {e}"))
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
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

    /// Get the encryption algorithm used by this Cypher.
    pub fn algorithm(&self) -> EncryptionAlgorithm {
        self.algorithm
    }

    /// Get the cryptographic scheme used by this Cypher.
    pub fn scheme(&self) -> CryptoScheme {
        self.scheme
    }
}

impl std::fmt::Debug for Cypher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cypher")
            .field("secret", &"[REDACTED]")
            .field("algorithm", &self.algorithm)
            .field("scheme", &self.scheme)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cypher_creation() {
        // Test SR25519 creation from SURI
        let cypher = Cypher::new(
            "//Alice",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();
        assert_eq!(cypher.scheme(), CryptoScheme::Sr25519);
        assert_eq!(cypher.algorithm(), EncryptionAlgorithm::XChaCha20Poly1305);

        // Test ED25519 creation from SURI
        let cypher = Cypher::new(
            "//Bob",
            KeypairType::Ed25519,
            EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();
        assert_eq!(cypher.scheme(), CryptoScheme::Ed25519);
        assert_eq!(cypher.algorithm(), EncryptionAlgorithm::AesGcm256);
    }

    #[test]
    fn test_cypher_from_seed() {
        let seed = [42u8; 32];
        let cypher = Cypher::from_seed(
            seed,
            KeypairType::Sr25519,
            EncryptionAlgorithm::ChaCha20Poly1305,
        );
        assert_eq!(cypher.scheme(), CryptoScheme::Sr25519);
        assert_eq!(cypher.algorithm(), EncryptionAlgorithm::ChaCha20Poly1305);
    }

    #[test]
    fn test_derive_public_key() {
        let cypher = Cypher::new(
            "//Alice",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();
        let public_key = cypher.derive_public_key().unwrap();
        assert_eq!(public_key.len(), 32);
        assert_ne!(public_key, [0u8; 32]); // Should not be all zeros
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_sr25519() {
        // Create sender and receiver
        let sender = Cypher::new(
            "//Alice",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();
        let receiver = Cypher::new(
            "//Bob",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();

        let plaintext = b"Hello, Robonomics!";
        let receiver_public = receiver.derive_public_key().unwrap();
        let sender_public = sender.derive_public_key().unwrap();

        // Encrypt
        let encrypted = sender.encrypt(plaintext, &receiver_public).unwrap();

        // Decrypt with sender verification
        let decrypted = receiver.decrypt(&encrypted, Some(&sender_public)).unwrap();
        assert_eq!(decrypted, plaintext);

        // Decrypt without sender verification
        let decrypted_any = receiver.decrypt(&encrypted, None).unwrap();
        assert_eq!(decrypted_any, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_ed25519() {
        // Create sender and receiver with ED25519
        let sender = Cypher::new(
            "//Alice",
            KeypairType::Ed25519,
            EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();
        let receiver = Cypher::new(
            "//Bob",
            KeypairType::Ed25519,
            EncryptionAlgorithm::AesGcm256,
        )
        .unwrap();

        let plaintext = b"ED25519 encrypted message";
        let receiver_public = receiver.derive_public_key().unwrap();
        let sender_public = sender.derive_public_key().unwrap();

        // Encrypt
        let encrypted = sender.encrypt(plaintext, &receiver_public).unwrap();

        // Decrypt
        let decrypted = receiver.decrypt(&encrypted, Some(&sender_public)).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let sender = Cypher::new(
            "//Alice",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();
        let receiver = Cypher::new(
            "//Bob",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();

        let plaintext = b"Same message";
        let receiver_public = receiver.derive_public_key().unwrap();

        // Encrypt same message twice
        let encrypted1 = sender.encrypt(plaintext, &receiver_public).unwrap();
        let encrypted2 = sender.encrypt(plaintext, &receiver_public).unwrap();

        // Should produce different ciphertexts due to random nonces
        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let sender = Cypher::new(
            "//Alice",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();
        let receiver = Cypher::new(
            "//Bob",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();
        let wrong_receiver = Cypher::new(
            "//Charlie",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();

        let plaintext = b"Secret message";
        let receiver_public = receiver.derive_public_key().unwrap();
        let sender_public = sender.derive_public_key().unwrap();

        // Encrypt for receiver
        let encrypted = sender.encrypt(plaintext, &receiver_public).unwrap();

        // Try to decrypt with wrong key
        let result = wrong_receiver.decrypt(&encrypted, Some(&sender_public));
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_rejects_wrong_sender() {
        let sender = Cypher::new(
            "//Alice",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();
        let receiver = Cypher::new(
            "//Bob",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();
        let wrong_sender = Cypher::new(
            "//Charlie",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();

        let plaintext = b"Test message";
        let receiver_public = receiver.derive_public_key().unwrap();
        let wrong_sender_public = wrong_sender.derive_public_key().unwrap();

        // Encrypt from sender
        let encrypted = sender.encrypt(plaintext, &receiver_public).unwrap();

        // Try to decrypt with wrong expected sender
        let result = receiver.decrypt(&encrypted, Some(&wrong_sender_public));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Sender public key mismatch"));
    }

    #[test]
    fn test_empty_message() {
        let sender = Cypher::new(
            "//Alice",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();
        let receiver = Cypher::new(
            "//Bob",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();

        let plaintext = b"";
        let receiver_public = receiver.derive_public_key().unwrap();
        let sender_public = sender.derive_public_key().unwrap();

        let encrypted = sender.encrypt(plaintext, &receiver_public).unwrap();
        let decrypted = receiver.decrypt(&encrypted, Some(&sender_public)).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_large_message() {
        let sender = Cypher::new(
            "//Alice",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();
        let receiver = Cypher::new(
            "//Bob",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();

        let plaintext = vec![42u8; 10000]; // 10KB message
        let receiver_public = receiver.derive_public_key().unwrap();
        let sender_public = sender.derive_public_key().unwrap();

        let encrypted = sender.encrypt(&plaintext, &receiver_public).unwrap();
        let decrypted = receiver.decrypt(&encrypted, Some(&sender_public)).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_all_algorithms() {
        let algorithms = vec![
            EncryptionAlgorithm::XChaCha20Poly1305,
            EncryptionAlgorithm::AesGcm256,
            EncryptionAlgorithm::ChaCha20Poly1305,
        ];

        for algorithm in algorithms {
            let sender = Cypher::new("//Alice", KeypairType::Sr25519, algorithm).unwrap();
            let receiver = Cypher::new("//Bob", KeypairType::Sr25519, algorithm).unwrap();

            let plaintext = format!("Test message for {:?}", algorithm);
            let receiver_public = receiver.derive_public_key().unwrap();
            let sender_public = sender.derive_public_key().unwrap();

            let encrypted = sender.encrypt(plaintext.as_bytes(), &receiver_public).unwrap();
            let decrypted = receiver.decrypt(&encrypted, Some(&sender_public)).unwrap();
            assert_eq!(decrypted, plaintext.as_bytes());
        }
    }

    #[test]
    fn test_corrupted_data_fails() {
        let sender = Cypher::new(
            "//Alice",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();
        let receiver = Cypher::new(
            "//Bob",
            KeypairType::Sr25519,
            EncryptionAlgorithm::XChaCha20Poly1305,
        )
        .unwrap();

        let plaintext = b"Test message";
        let receiver_public = receiver.derive_public_key().unwrap();
        let sender_public = sender.derive_public_key().unwrap();

        let mut encrypted = sender.encrypt(plaintext, &receiver_public).unwrap();

        // Corrupt the data
        if encrypted.len() > 10 {
            encrypted[10] ^= 0xFF;
        }

        let result = receiver.decrypt(&encrypted, Some(&sender_public));
        assert!(result.is_err());
    }
}
