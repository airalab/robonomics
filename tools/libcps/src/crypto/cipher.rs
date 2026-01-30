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
//! Cipher implementation for encryption and decryption.
//!
//! This module provides the Cipher struct which handles ECDH key agreement,
//! HKDF key derivation, and AEAD encryption/decryption operations.

use super::types::{CryptoScheme, EncryptedMessage, EncryptionAlgorithm};
use aes_gcm::{
    aead::{Aead as AesAead, AeadCore as AesAeadCore, KeyInit as AesKeyInit},
    Aes256Gcm, Nonce as AesNonce,
};
use anyhow::{anyhow, Result};
use chacha20poly1305::{
    aead::OsRng, ChaCha20Poly1305, Nonce as ChachaNonce, XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use log::{debug, trace};
use parity_scale_codec::Encode;
use sha2::Sha256;
use sp_core::Pair;

/// HKDF salt for key derivation.
const HKDF_SALT: &[u8] = b"robonomics-network";

/// Cipher configuration for encryption and decryption operations.
///
/// Stores only the 32-byte secret key and algorithm for optimal performance.
/// Uses direct ECDH implementations:
/// - SR25519: Ristretto255 scalar multiplication via schnorrkel
/// - ED25519: X25519 key agreement via curve25519-dalek
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{Cipher, EncryptionAlgorithm, CryptoScheme};
///
/// let cipher = Cipher::new(
///     "//Alice".to_string(),
///     CryptoScheme::Sr25519,
/// ).unwrap();
///
/// let plaintext = b"secret message";
/// let receiver_public = &[0u8; 32]; // receiver's public key
/// let encrypted_msg = cipher.encrypt(plaintext, receiver_public, EncryptionAlgorithm::XChaCha20Poly1305).unwrap();
/// let decrypted = cipher.decrypt(&encrypted_msg, None).unwrap();
/// ```
pub struct Cipher {
    /// 32-byte secret key
    secret: [u8; 32],
    /// Cached public key (derived once in constructor)
    public_key: [u8; 32],
    /// Cryptographic scheme
    scheme: CryptoScheme,
}
impl Cipher {
    /// Create a new Cipher configuration.
    ///
    /// Extracts and stores only the 32-byte secret key for optimal performance.
    ///
    /// # Arguments
    ///
    /// * `suri` - Secret URI for the keypair
    /// * `scheme` - Cryptographic scheme to use (Sr25519 or Ed25519)
    ///
    /// # Returns
    ///
    /// Returns a Cipher instance with the secret key and public key
    ///
    /// # Errors
    ///
    /// Returns error if keypair parsing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libcps::crypto::{Cipher, CryptoScheme};
    ///
    /// let cipher = Cipher::new(
    ///     "//Alice".to_string(),
    ///     CryptoScheme::Sr25519,
    /// ).unwrap();
    /// ```
    pub fn new(suri: String, scheme: CryptoScheme) -> Result<Self> {
        debug!("Creating new Cipher with scheme: {:?}", scheme);
        trace!("SURI length: {} chars", suri.len());

        let (secret, public_key) = match scheme {
            CryptoScheme::Sr25519 => {
                trace!("Parsing SR25519 keypair from SURI");
                let pair = sp_core::sr25519::Pair::from_string(&suri, None)
                    .map_err(|e| anyhow!("Failed to parse SR25519 keypair: {:?}", e))?;
                let secret_bytes = pair.to_raw_vec();
                let mut secret = [0u8; 32];
                secret.copy_from_slice(&secret_bytes[..32]);
                // Derive public key using Pair interface
                let public_key = pair.public().0;
                debug!("SR25519 keypair created successfully");
                (secret, public_key)
            }
            CryptoScheme::Ed25519 => {
                trace!("Parsing ED25519 keypair from SURI");
                let pair = sp_core::ed25519::Pair::from_string(&suri, None)
                    .map_err(|e| anyhow!("Failed to parse ED25519 keypair: {:?}", e))?;
                let secret_bytes = pair.to_raw_vec();
                let mut secret = [0u8; 32];
                secret.copy_from_slice(&secret_bytes[..32]);
                // Derive public key using Pair interface
                let public_key = pair.public().0;
                debug!("ED25519 keypair created successfully");
                (secret, public_key)
            }
        };
        Ok(Cipher {
            secret,
            public_key,
            scheme,
        })
    }

    /// Get the cryptographic scheme.
    pub fn scheme(&self) -> CryptoScheme {
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
    /// Returns error if the public key cannot be decompressed into a valid curve point.
    /// Not all 32-byte arrays represent valid curve points - decompression validates
    /// the point is on the curve and meets other curve-specific requirements.
    fn derive_shared_secret(&self, receiver_public: &[u8; 32]) -> Result<[u8; 32]> {
        match self.scheme {
            CryptoScheme::Sr25519 => {
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
            CryptoScheme::Ed25519 => {
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
                let their_x25519_public =
                    x25519_dalek::PublicKey::from(montgomery_point.to_bytes());

                // Perform X25519 ECDH
                let shared_secret = my_x25519_secret.diffie_hellman(&their_x25519_public);
                Ok(*shared_secret.as_bytes())
            }
        }
    }

    /// Derive encryption key from shared secret using HKDF-SHA256.
    ///
    /// This is an internal helper function that performs the HKDF key derivation
    /// with a specified algorithm. Used by both encryption and decryption paths.
    ///
    /// # Arguments
    ///
    /// * `shared_secret` - The 32-byte shared secret from ECDH key agreement
    /// * `algorithm` - The encryption algorithm, which determines the info string
    ///
    /// # HKDF Process
    ///
    /// The function implements RFC 5869 HKDF with these parameters:
    ///
    /// 1. **Hash Function**: SHA-256 (provides 256-bit output)
    /// 2. **Salt**: `"robonomics-network"` (constant, for domain separation)
    /// 3. **IKM**: The ECDH shared secret (input keying material)
    /// 4. **Info**: Algorithm-specific string (e.g., "robonomics-cps-xchacha20poly1305")
    /// 5. **Length**: 32 bytes (256 bits for encryption key)
    ///
    /// ## Extract Phase
    /// ```text
    /// PRK = HMAC-SHA256(salt, shared_secret)
    /// ```
    /// Produces a pseudorandom key with strong entropy properties.
    ///
    /// ## Expand Phase
    /// ```text
    /// OKM = HMAC-SHA256(PRK, info || 0x01)[0..32]
    /// ```
    /// Produces the final 32-byte encryption key bound to the algorithm.
    ///
    /// # Security Properties
    ///
    /// - **Algorithm Binding**: Different algorithms produce different keys due to
    ///   unique info strings, preventing cross-algorithm attacks.
    /// - **Domain Separation**: Salt binds keys to Robonomics network context.
    /// - **Entropy Extraction**: HKDF extract ensures output has uniform distribution
    ///   even if input has biases.
    /// - **Key Independence**: Each (shared_secret, algorithm) pair produces a
    ///   cryptographically independent key.
    ///
    /// # Returns
    ///
    /// Returns a 32-byte (256-bit) encryption key suitable for the specified
    /// AEAD algorithm.
    ///
    /// # Errors
    ///
    /// Returns an error if the HKDF expand operation fails (which should never
    /// happen with valid parameters).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let shared_secret = [0u8; 32]; // from ECDH
    /// let algorithm = EncryptionAlgorithm::XChaCha20Poly1305;
    /// let key = Cipher::derive_encryption_key_with_algorithm(&shared_secret, &algorithm)?;
    /// // key is now a unique 32-byte encryption key
    /// ```
    fn derive_encryption_key_with_algorithm(
        shared_secret: &[u8; 32],
        algorithm: &EncryptionAlgorithm,
    ) -> Result<[u8; 32]> {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let hkdf = Hkdf::<Sha256>::new(Some(HKDF_SALT), shared_secret);
        let mut okm = [0u8; 32];
        hkdf.expand(algorithm.info_string(), &mut okm)
            .map_err(|e| anyhow!("HKDF expansion failed: {e}"))?;
        Ok(okm)
    }

    /// Get sender's public key.
    ///
    /// Returns the cached public key that was derived in the constructor.
    pub fn public_key(&self) -> [u8; 32] {
        self.public_key
    }

    /// Encrypt data for a specific receiver with inlined AEAD.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The data to encrypt
    /// * `receiver_public` - The recipient's public key (exactly 32 bytes)
    /// * `algorithm` - The encryption algorithm to use
    ///
    /// # Returns
    ///
    /// Returns an EncryptedMessage structure that can be serialized by the caller
    ///
    /// # Errors
    ///
    /// Returns error if the receiver's public key is invalid (not a valid curve point).
    /// This can happen if:
    /// - The public key bytes don't represent a valid Ristretto255 point (SR25519)
    /// - The public key bytes don't represent a valid Edwards curve point (Ed25519)
    /// - The receiver_public parameter contains corrupted or malicious data
    ///
    /// Note: Valid public keys from Substrate accounts will always succeed.
    pub fn encrypt(
        &self,
        plaintext: &[u8],
        receiver_public: &[u8; 32],
        algorithm: EncryptionAlgorithm,
    ) -> Result<EncryptedMessage> {
        debug!(
            "Encrypting {} bytes with {} using {:?} scheme",
            plaintext.len(),
            algorithm,
            self.scheme
        );
        trace!("Receiver public key: {:02x?}...", &receiver_public[..8]);

        // Step 1: Derive shared secret using direct ECDH
        // This can fail if receiver_public is invalid
        trace!("Deriving shared secret via ECDH");
        let shared_secret = self.derive_shared_secret(receiver_public)?;
        trace!("Shared secret derived successfully");

        // Step 2: Derive encryption key using HKDF with salt
        // HKDF expand can only fail if the output length exceeds the hash function's
        // maximum (255 * hash_len for SHA-256 = 8160 bytes), but we only request 32 bytes.
        // We propagate the error for defensive programming rather than panicking.
        trace!("Deriving encryption key with HKDF");
        let encryption_key = Self::derive_encryption_key_with_algorithm(&shared_secret, &algorithm)
            .map_err(|e| anyhow!("HKDF key derivation failed: {e}"))?;
        trace!("Encryption key derived");

        // Step 3: Encrypt with specified algorithm
        trace!("Encrypting plaintext with {:?}", algorithm);
        let (nonce_bytes, ciphertext) = match algorithm {
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                let cipher = XChaCha20Poly1305::new(&encryption_key.into());
                let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
                trace!("Generated XChaCha20 nonce: {} bytes", nonce.len());
                let ct = cipher
                    .encrypt(&nonce, plaintext)
                    .map_err(|e| anyhow!("XChaCha20 encryption failed: {e}"))?;
                (nonce.to_vec(), ct)
            }
            EncryptionAlgorithm::AesGcm256 => {
                let cipher = Aes256Gcm::new(&encryption_key.into());
                let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
                trace!("Generated AES-GCM nonce: {} bytes", nonce.len());
                let ct = cipher
                    .encrypt(&nonce, plaintext)
                    .map_err(|e| anyhow!("AES-GCM encryption failed: {e}"))?;
                (nonce.to_vec(), ct)
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(&encryption_key.into());
                let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
                trace!("Generated ChaCha20 nonce: {} bytes", nonce.len());
                let ct = cipher
                    .encrypt(&nonce, plaintext)
                    .map_err(|e| anyhow!("ChaCha20 encryption failed: {e}"))?;
                (nonce.to_vec(), ct)
            }
        };

        // Step 4: Get sender's public key
        let sender_public = self.public_key();
        trace!("Sender public key: {:02x?}...", &sender_public[..8]);

        // Step 5: Create and return message structure with binary data
        debug!(
            "Encryption complete: {} bytes plaintext -> {} bytes ciphertext (+ {} bytes overhead)",
            plaintext.len(),
            ciphertext.len(),
            ciphertext.len() - plaintext.len()
        );
        Ok(EncryptedMessage::V1 {
            algorithm,
            from: sender_public,
            nonce: nonce_bytes,
            ciphertext,
        })
    }

    /// Decrypt data with inlined AEAD (algorithm auto-detected).
    ///
    /// # Arguments
    ///
    /// * `message` - Encrypted message structure
    /// * `expected_sender` - Optional sender public key for verification
    ///
    /// # Returns
    ///
    /// Returns decrypted plaintext bytes
    ///
    /// # Errors
    ///
    /// Returns error if decryption fails or sender verification fails
    pub fn decrypt(
        &self,
        message: &EncryptedMessage,
        expected_sender: Option<&[u8; 32]>,
    ) -> Result<Vec<u8>> {
        match message {
            EncryptedMessage::V1 {
                algorithm,
                from,
                nonce,
                ciphertext,
            } => {
                debug!(
                    "Decrypting message with {:?} using {:?} scheme",
                    algorithm, self.scheme
                );
                trace!(
                    "Ciphertext: {} bytes, nonce: {} bytes",
                    ciphertext.len(),
                    nonce.len()
                );
                trace!("Sender public key: {:02x?}...", &from[..8]);

                // Step 1: Verify sender if expected
                if let Some(expected_pk) = expected_sender {
                    trace!("Verifying sender public key");
                    if from != expected_pk {
                        return Err(anyhow!(
                            "Sender public key mismatch: message from unexpected sender"
                        ));
                    }
                    trace!("Sender verification passed");
                }

                // Step 2: Derive shared secret using direct ECDH
                trace!("Deriving shared secret via ECDH");
                let shared_secret = self.derive_shared_secret(from)?;
                trace!("Shared secret derived successfully");

                // Step 3: Derive encryption key using HKDF with salt
                trace!("Deriving decryption key with HKDF");
                let encryption_key =
                    Self::derive_encryption_key_with_algorithm(&shared_secret, algorithm)?;
                trace!("Decryption key derived");

                // Step 4: Decrypt with appropriate algorithm
                trace!("Decrypting ciphertext with {:?}", algorithm);
                let plaintext = match algorithm {
                    EncryptionAlgorithm::XChaCha20Poly1305 => {
                        if nonce.len() != 24 {
                            return Err(anyhow!(
                                "Invalid XChaCha20 nonce length: expected 24 bytes"
                            ));
                        }
                        let nonce_array = XNonce::from_slice(nonce);
                        let cipher = XChaCha20Poly1305::new(&encryption_key.into());
                        cipher
                            .decrypt(nonce_array, ciphertext.as_ref())
                            .map_err(|e| anyhow!("XChaCha20 decryption failed: {e}"))
                    }
                    EncryptionAlgorithm::AesGcm256 => {
                        if nonce.len() != 12 {
                            return Err(anyhow!("Invalid AES-GCM nonce length: expected 12 bytes"));
                        }
                        let nonce_array = AesNonce::from_slice(nonce);
                        let cipher = Aes256Gcm::new(&encryption_key.into());
                        cipher
                            .decrypt(nonce_array, ciphertext.as_ref())
                            .map_err(|e| anyhow!("AES-GCM decryption failed: {e}"))
                    }
                    EncryptionAlgorithm::ChaCha20Poly1305 => {
                        if nonce.len() != 12 {
                            return Err(anyhow!(
                                "Invalid ChaCha20 nonce length: expected 12 bytes"
                            ));
                        }
                        let nonce_array = ChachaNonce::from_slice(nonce);
                        let cipher = ChaCha20Poly1305::new(&encryption_key.into());
                        cipher
                            .decrypt(nonce_array, ciphertext.as_ref())
                            .map_err(|e| anyhow!("ChaCha20 decryption failed: {e}"))
                    }
                }?;

                debug!(
                    "Decryption complete: {} bytes ciphertext -> {} bytes plaintext",
                    ciphertext.len(),
                    plaintext.len()
                );
                Ok(plaintext)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algorithm_from_str() {
        assert_eq!(
            EncryptionAlgorithm::from_str("xchacha20").unwrap(),
            EncryptionAlgorithm::XChaCha20Poly1305
        );
        assert_eq!(
            EncryptionAlgorithm::from_str("aesgcm256").unwrap(),
            EncryptionAlgorithm::AesGcm256
        );
        assert_eq!(
            EncryptionAlgorithm::from_str("chacha20").unwrap(),
            EncryptionAlgorithm::ChaCha20Poly1305
        );
    }

    #[test]
    fn test_algorithm_info_strings() {
        assert_eq!(
            EncryptionAlgorithm::XChaCha20Poly1305.info_string(),
            b"robonomics-cps-xchacha20poly1305"
        );
        assert_eq!(
            EncryptionAlgorithm::AesGcm256.info_string(),
            b"robonomics-cps-aesgcm256"
        );
        assert_eq!(
            EncryptionAlgorithm::ChaCha20Poly1305.info_string(),
            b"robonomics-cps-chacha20poly1305"
        );
    }

    #[test]
    fn test_nonce_sizes() {
        assert_eq!(EncryptionAlgorithm::XChaCha20Poly1305.nonce_size(), 24);
        assert_eq!(EncryptionAlgorithm::AesGcm256.nonce_size(), 12);
        assert_eq!(EncryptionAlgorithm::ChaCha20Poly1305.nonce_size(), 12);
    }

    #[test]
    fn test_default() {
        assert_eq!(
            EncryptionAlgorithm::default(),
            EncryptionAlgorithm::XChaCha20Poly1305
        );
    }

    #[test]
    fn test_cipher_creation() {
        let cipher = Cipher::new("//Alice".to_string(), CryptoScheme::Sr25519).unwrap();

        assert_eq!(cipher.scheme(), CryptoScheme::Sr25519);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_sr25519() {
        let cipher = Cipher::new("//Alice".to_string(), CryptoScheme::Sr25519).unwrap();

        // Get Alice's public key for self-encryption
        let public_key = cipher.public_key();

        let plaintext = b"Hello, World!";
        let encrypted = cipher
            .encrypt(
                plaintext,
                &public_key,
                EncryptionAlgorithm::XChaCha20Poly1305,
            )
            .unwrap();
        let decrypted = cipher.decrypt(&encrypted, None).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_ed25519() {
        let cipher = Cipher::new("//Alice".to_string(), CryptoScheme::Ed25519).unwrap();

        // Get Alice's public key for self-encryption
        let public_key = cipher.public_key();

        let plaintext = b"Hello, World!";
        let encrypted = cipher
            .encrypt(plaintext, &public_key, EncryptionAlgorithm::AesGcm256)
            .unwrap();
        let decrypted = cipher.decrypt(&encrypted, None).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_cross_party_encryption_sr25519() {
        let alice = Cipher::new("//Alice".to_string(), CryptoScheme::Sr25519).unwrap();

        let bob = Cipher::new("//Bob".to_string(), CryptoScheme::Sr25519).unwrap();

        let bob_public = bob.public_key();
        let alice_public = alice.public_key();

        let plaintext = b"Secret from Alice to Bob";
        let encrypted = alice
            .encrypt(
                plaintext,
                &bob_public,
                EncryptionAlgorithm::XChaCha20Poly1305,
            )
            .unwrap();
        let decrypted = bob.decrypt(&encrypted, Some(&alice_public)).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_sender_verification_fails() {
        let alice = Cipher::new("//Alice".to_string(), CryptoScheme::Sr25519).unwrap();

        let bob = Cipher::new("//Bob".to_string(), CryptoScheme::Sr25519).unwrap();

        let charlie = Cipher::new("//Charlie".to_string(), CryptoScheme::Sr25519).unwrap();

        let bob_public = bob.public_key();
        let charlie_public = charlie.public_key();

        let plaintext = b"From Alice";
        let encrypted = alice
            .encrypt(
                plaintext,
                &bob_public,
                EncryptionAlgorithm::XChaCha20Poly1305,
            )
            .unwrap();

        // Should fail: expecting message from Charlie, but it's from Alice
        let result = bob.decrypt(&encrypted, Some(&charlie_public));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("mismatch"));
    }

    #[test]
    fn test_derive_shared_secret_sr25519() {
        let alice = Cipher::new("//Alice".to_string(), CryptoScheme::Sr25519).unwrap();

        let bob = Cipher::new("//Bob".to_string(), CryptoScheme::Sr25519).unwrap();

        let bob_public = bob.public_key();
        let alice_public = alice.public_key();

        // Derive shared secrets
        let alice_shared = alice.derive_shared_secret(&bob_public).unwrap();
        let bob_shared = bob.derive_shared_secret(&alice_public).unwrap();

        // Shared secrets should match (Diffie-Hellman property)
        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_derive_shared_secret_ed25519() {
        let alice = Cipher::new("//Alice".to_string(), CryptoScheme::Ed25519).unwrap();

        let bob = Cipher::new("//Bob".to_string(), CryptoScheme::Ed25519).unwrap();

        let bob_public = bob.public_key();
        let alice_public = alice.public_key();

        // Derive shared secrets
        let alice_shared = alice.derive_shared_secret(&bob_public).unwrap();
        let bob_shared = bob.derive_shared_secret(&alice_public).unwrap();

        // Shared secrets should match (Diffie-Hellman property)
        assert_eq!(alice_shared, bob_shared);
    }
}
