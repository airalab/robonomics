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
//! Cypher configuration and encryption/decryption operations with inlined ECDH.
//!
//! This module provides the `Cypher` struct which encapsulates cryptographic
//! configuration and operations with direct ECDH implementation for maximum performance.

use aes_gcm::{
    aead::{Aead as AesAead, AeadCore as AesAeadCore, KeyInit as AesKeyInit},
    Aes256Gcm, Nonce as AesNonce,
};
use anyhow::{anyhow, Result};
use chacha20poly1305::{
    aead::OsRng, ChaCha20Poly1305, Nonce as ChachaNonce, XChaCha20Poly1305, XNonce,
};
use serde::{Deserialize, Serialize};
use sp_core::Pair;

/// Encrypted message format stored on-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedMessage {
    version: u8,
    #[serde(default = "default_algorithm")]
    algorithm: String,
    from: String,
    nonce: String,
    ciphertext: String,
}

fn default_algorithm() -> String {
    "xchacha20".to_string()
}

/// Cypher configuration for encryption and decryption operations.
///
/// Stores only the 32-byte secret key and algorithm for optimal performance.
/// Uses direct ECDH implementations:
/// - SR25519: Ristretto255 scalar multiplication via schnorrkel
/// - ED25519: X25519 key agreement via curve25519-dalek
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{Cypher, EncryptionAlgorithm, CryptoScheme};
///
/// let cypher = Cypher::new(
///     "//Alice".to_string(),
///     EncryptionAlgorithm::XChaCha20Poly1305,
///     CryptoScheme::Sr25519,
/// ).unwrap();
///
/// let plaintext = b"secret message";
/// let receiver_public = &[0u8; 32]; // receiver's public key
/// let encrypted = cypher.encrypt(plaintext, receiver_public).unwrap();
/// let decrypted = cypher.decrypt(&encrypted, None).unwrap();
/// ```
pub struct Cypher {
    /// 32-byte secret key
    secret: [u8; 32],
    /// Encryption algorithm
    algorithm: crate::crypto::EncryptionAlgorithm,
    /// Cryptographic scheme
    scheme: crate::crypto::CryptoScheme,
}

impl Cypher {
    /// Create a new Cypher configuration.
    ///
    /// Extracts and stores only the 32-byte secret key for optimal performance.
    ///
    /// # Arguments
    ///
    /// * `suri` - Secret URI for the keypair
    /// * `algorithm` - Encryption algorithm to use
    /// * `scheme` - Cryptographic scheme to use
    ///
    /// # Returns
    ///
    /// Returns a Cypher instance with the secret key
    ///
    /// # Errors
    ///
    /// Returns error if keypair parsing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libcps::crypto::{Cypher, EncryptionAlgorithm, CryptoScheme};
    ///
    /// let cypher = Cypher::new(
    ///     "//Alice".to_string(),
    ///     EncryptionAlgorithm::XChaCha20Poly1305,
    ///     CryptoScheme::Sr25519,
    /// ).unwrap();
    /// ```
    pub fn new(
        suri: String,
        algorithm: crate::crypto::EncryptionAlgorithm,
        scheme: crate::crypto::CryptoScheme,
    ) -> Result<Self> {
        let secret = match scheme {
            crate::crypto::CryptoScheme::Sr25519 => {
                let pair = sp_core::sr25519::Pair::from_string(&suri, None)
                    .map_err(|e| anyhow!("Failed to parse SR25519 keypair: {:?}", e))?;
                let secret_bytes = pair.to_raw_vec();
                let mut secret = [0u8; 32];
                secret.copy_from_slice(&secret_bytes[..32]);
                secret
            }
            crate::crypto::CryptoScheme::Ed25519 => {
                let pair = sp_core::ed25519::Pair::from_string(&suri, None)
                    .map_err(|e| anyhow!("Failed to parse ED25519 keypair: {:?}", e))?;
                let secret_bytes = pair.to_raw_vec();
                let mut secret = [0u8; 32];
                secret.copy_from_slice(&secret_bytes[..32]);
                secret
            }
        };
        Ok(Cypher {
            secret,
            algorithm,
            scheme,
        })
    }

    /// Get the encryption algorithm.
    pub fn algorithm(&self) -> crate::crypto::EncryptionAlgorithm {
        self.algorithm
    }

    /// Get the cryptographic scheme.
    pub fn scheme(&self) -> crate::crypto::CryptoScheme {
        self.scheme
    }

    /// Derive shared secret using direct ECDH.
    ///
    /// # Arguments
    ///
    /// * `receiver_public` - The receiver's public key (32 bytes)
    ///
    /// # Returns
    ///
    /// Returns 32-byte shared secret
    ///
    /// # Errors
    ///
    /// Returns error if ECDH fails
    fn derive_shared_secret(&self, receiver_public: &[u8; 32]) -> Result<[u8; 32]> {
        match self.scheme {
            crate::crypto::CryptoScheme::Sr25519 => {
                // SR25519: Use Ristretto255 for ECDH
                use curve25519_dalek::ristretto::CompressedRistretto;
                use curve25519_dalek::scalar::Scalar;
                use sha2::{Digest, Sha512};

                // Create scalar from secret key
                let scalar = Scalar::from_bytes_mod_order(self.secret);

                // Decompress receiver's public key as Ristretto point
                let public_compressed = CompressedRistretto(*receiver_public);
                let public_point = public_compressed
                    .decompress()
                    .ok_or_else(|| anyhow!("Failed to decompress Ristretto255 public key"))?;

                // Perform scalar multiplication
                let shared_point = scalar * public_point;
                let shared_compressed = shared_point.compress();

                // Hash for uniform distribution
                let mut hasher = Sha512::new();
                hasher.update(b"robonomics-cps-ecdh");
                hasher.update(shared_compressed.as_bytes());
                let hash_output = hasher.finalize();

                let mut result = [0u8; 32];
                result.copy_from_slice(&hash_output[..32]);
                Ok(result)
            }
            crate::crypto::CryptoScheme::Ed25519 => {
                // ED25519: Use X25519 for ECDH
                use curve25519_dalek::edwards::CompressedEdwardsY;
                use sha2::{Digest, Sha512};

                // Hash and clamp secret for X25519
                let mut hasher = Sha512::new();
                hasher.update(&self.secret);
                let hash = hasher.finalize();

                let mut scalar_bytes = [0u8; 32];
                scalar_bytes.copy_from_slice(&hash[..32]);

                // Clamp for X25519
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

    /// Derive encryption key from shared secret using HKDF-SHA256.
    fn derive_encryption_key(&self, shared_secret: &[u8; 32]) -> Result<[u8; 32]> {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let hkdf = Hkdf::<Sha256>::new(None, shared_secret);
        let mut okm = [0u8; 32];
        hkdf.expand(self.algorithm.info_string(), &mut okm)
            .map_err(|e| anyhow!("HKDF expansion failed: {e}"))?;
        Ok(okm)
    }

    /// Get sender's public key.
    fn public_key(&self) -> Result<[u8; 32]> {
        match self.scheme {
            crate::crypto::CryptoScheme::Sr25519 => {
                // Derive public key from secret for SR25519
                use curve25519_dalek::ristretto::RistrettoPoint;
                use curve25519_dalek::scalar::Scalar;

                let scalar = Scalar::from_bytes_mod_order(self.secret);
                let public_point = RistrettoPoint::mul_base(&scalar);
                Ok(public_point.compress().to_bytes())
            }
            crate::crypto::CryptoScheme::Ed25519 => {
                // Derive public key from secret for ED25519
                use sha2::{Digest, Sha512};

                let mut hasher = Sha512::new();
                hasher.update(&self.secret);
                let hash = hasher.finalize();

                let mut scalar_bytes = [0u8; 32];
                scalar_bytes.copy_from_slice(&hash[..32]);
                scalar_bytes[0] &= 248;
                scalar_bytes[31] &= 127;
                scalar_bytes[31] |= 64;

                let scalar = curve25519_dalek::scalar::Scalar::from_bytes_mod_order(scalar_bytes);
                let public_point = curve25519_dalek::constants::ED25519_BASEPOINT_TABLE * &scalar;
                
                Ok(public_point.compress().to_bytes())
            }
        }
    }

    /// Encrypt data for a specific receiver with inlined AEAD.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The data to encrypt
    /// * `receiver_public` - The recipient's public key (exactly 32 bytes)
    ///
    /// # Returns
    ///
    /// Returns encrypted bytes in JSON format
    ///
    /// # Errors
    ///
    /// Returns error if encryption fails
    pub fn encrypt(&self, plaintext: &[u8], receiver_public: &[u8; 32]) -> Result<Vec<u8>> {
        use base64::{engine::general_purpose, Engine as _};

        // Step 1: Derive shared secret using direct ECDH
        let shared_secret = self.derive_shared_secret(receiver_public)?;

        // Step 2: Derive encryption key using HKDF
        let encryption_key = self.derive_encryption_key(&shared_secret)?;

        // Step 3: Encrypt with specified algorithm
        let (nonce_bytes, ciphertext) = match self.algorithm {
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

        // Step 4: Get sender's public key
        let sender_public = self.public_key()?;

        // Step 5: Create message structure
        let algorithm_str = match self.algorithm {
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305 => "xchacha20",
            crate::crypto::EncryptionAlgorithm::AesGcm256 => "aesgcm256",
            crate::crypto::EncryptionAlgorithm::ChaCha20Poly1305 => "chacha20",
        };

        let message = EncryptedMessage {
            version: 1,
            algorithm: algorithm_str.to_string(),
            from: bs58::encode(&sender_public).into_string(),
            nonce: general_purpose::STANDARD.encode(&nonce_bytes),
            ciphertext: general_purpose::STANDARD.encode(&ciphertext),
        };

        // Step 6: Serialize to JSON
        serde_json::to_vec(&message).map_err(|e| anyhow!("JSON serialization failed: {e}"))
    }

    /// Decrypt data with inlined AEAD (algorithm auto-detected).
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - JSON-formatted encrypted data
    /// * `expected_sender` - Optional sender public key for verification
    ///
    /// # Returns
    ///
    /// Returns decrypted plaintext bytes
    ///
    /// # Errors
    ///
    /// Returns error if decryption fails or sender verification fails
    pub fn decrypt(&self, ciphertext: &[u8], expected_sender: Option<&[u8; 32]>) -> Result<Vec<u8>> {
        use base64::{engine::general_purpose, Engine as _};
        use std::str::FromStr;

        // Step 1: Parse message
        let message: EncryptedMessage = serde_json::from_slice(ciphertext)
            .map_err(|e| anyhow!("Failed to parse encrypted message: {e}"))?;

        if message.version != 1 {
            return Err(anyhow!(
                "Unsupported encryption version: {}",
                message.version
            ));
        }

        // Step 2: Parse algorithm
        let algorithm = crate::crypto::EncryptionAlgorithm::from_str(&message.algorithm)
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

        let mut sender_pk_array = [0u8; 32];
        sender_pk_array.copy_from_slice(&sender_public_bytes);

        // Step 4: Optionally verify sender
        if let Some(expected_pk) = expected_sender {
            if &sender_pk_array != expected_pk {
                return Err(anyhow!(
                    "Sender public key mismatch: message from unexpected sender"
                ));
            }
        }

        // Step 5: Derive shared secret using direct ECDH
        let shared_secret = self.derive_shared_secret(&sender_pk_array)?;

        // Step 6: Derive encryption key using HKDF
        use hkdf::Hkdf;
        use sha2::Sha256;
        let hkdf = Hkdf::<Sha256>::new(None, &shared_secret);
        let mut encryption_key = [0u8; 32];
        hkdf.expand(algorithm.info_string(), &mut encryption_key)
            .map_err(|e| anyhow!("HKDF expansion failed: {e}"))?;

        // Step 7: Decode nonce and ciphertext
        let nonce_bytes = general_purpose::STANDARD
            .decode(&message.nonce)
            .map_err(|e| anyhow!("Failed to decode nonce: {e}"))?;

        let ciphertext_bytes = general_purpose::STANDARD
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
                    .decrypt(nonce, ciphertext_bytes.as_ref())
                    .map_err(|e| anyhow!("XChaCha20 decryption failed: {e}"))
            }
            crate::crypto::EncryptionAlgorithm::AesGcm256 => {
                if nonce_bytes.len() != 12 {
                    return Err(anyhow!("Invalid AES-GCM nonce length: expected 12 bytes"));
                }
                let nonce = AesNonce::from_slice(&nonce_bytes);
                let cipher = Aes256Gcm::new(&encryption_key.into());
                cipher
                    .decrypt(nonce, ciphertext_bytes.as_ref())
                    .map_err(|e| anyhow!("AES-GCM decryption failed: {e}"))
            }
            crate::crypto::EncryptionAlgorithm::ChaCha20Poly1305 => {
                if nonce_bytes.len() != 12 {
                    return Err(anyhow!("Invalid ChaCha20 nonce length: expected 12 bytes"));
                }
                let nonce = ChachaNonce::from_slice(&nonce_bytes);
                let cipher = ChaCha20Poly1305::new(&encryption_key.into());
                cipher
                    .decrypt(nonce, ciphertext_bytes.as_ref())
                    .map_err(|e| anyhow!("ChaCha20 decryption failed: {e}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cypher_creation() {
        let cypher = Cypher::new(
            "//Alice".to_string(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
            crate::crypto::CryptoScheme::Sr25519,
        ).unwrap();
        
        assert_eq!(cypher.algorithm(), crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305);
        assert_eq!(cypher.scheme(), crate::crypto::CryptoScheme::Sr25519);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_sr25519() {
        let cypher = Cypher::new(
            "//Alice".to_string(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
            crate::crypto::CryptoScheme::Sr25519,
        ).unwrap();

        // Get Alice's public key for self-encryption
        let public_key = cypher.public_key().unwrap();

        let plaintext = b"Hello, World!";
        let encrypted = cypher.encrypt(plaintext, &public_key).unwrap();
        let decrypted = cypher.decrypt(&encrypted, None).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_ed25519() {
        let cypher = Cypher::new(
            "//Alice".to_string(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
            crate::crypto::CryptoScheme::Ed25519,
        ).unwrap();

        // Get Alice's public key for self-encryption
        let public_key = cypher.public_key().unwrap();

        let plaintext = b"Hello, World!";
        let encrypted = cypher.encrypt(plaintext, &public_key).unwrap();
        let decrypted = cypher.decrypt(&encrypted, None).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_cross_party_encryption_sr25519() {
        let alice = Cypher::new(
            "//Alice".to_string(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
            crate::crypto::CryptoScheme::Sr25519,
        ).unwrap();

        let bob = Cypher::new(
            "//Bob".to_string(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
            crate::crypto::CryptoScheme::Sr25519,
        ).unwrap();

        let bob_public = bob.public_key().unwrap();
        let alice_public = alice.public_key().unwrap();

        let plaintext = b"Secret from Alice to Bob";
        let encrypted = alice.encrypt(plaintext, &bob_public).unwrap();
        let decrypted = bob.decrypt(&encrypted, Some(&alice_public)).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_sender_verification_fails() {
        let alice = Cypher::new(
            "//Alice".to_string(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
            crate::crypto::CryptoScheme::Sr25519,
        ).unwrap();

        let bob = Cypher::new(
            "//Bob".to_string(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
            crate::crypto::CryptoScheme::Sr25519,
        ).unwrap();

        let charlie = Cypher::new(
            "//Charlie".to_string(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
            crate::crypto::CryptoScheme::Sr25519,
        ).unwrap();

        let bob_public = bob.public_key().unwrap();
        let charlie_public = charlie.public_key().unwrap();

        let plaintext = b"From Alice";
        let encrypted = alice.encrypt(plaintext, &bob_public).unwrap();

        // Should fail: expecting message from Charlie, but it's from Alice
        let result = bob.decrypt(&encrypted, Some(&charlie_public));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("mismatch"));
    }
}
