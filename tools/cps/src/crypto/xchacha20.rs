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
//! use libcps::crypto::{encrypt_with_algorithm, decrypt, EncryptionAlgorithm};
//! use schnorrkel::SecretKey;
//!
//! # fn example() -> anyhow::Result<()> {
//! let sender_secret = SecretKey::from_bytes(&[0u8; 64])?;
//! let receiver_public = [0u8; 32];
//! let plaintext = b"secret message";
//!
//! // Encrypt with specific algorithm
//! let encrypted = encrypt_with_algorithm(
//!     plaintext,
//!     &sender_secret,
//!     &receiver_public,
//!     EncryptionAlgorithm::AesGcm256
//! )?;
//!
//! // Decrypt (algorithm auto-detected)
//! let receiver_secret = SecretKey::from_bytes(&[0u8; 64])?;
//! let decrypted = decrypt(&encrypted, &receiver_secret)?;
//!
//! assert_eq!(plaintext, &decrypted[..]);
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
    aead::{AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce as ChachaNonce, XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use schnorrkel::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::crypto::KeypairType;

/// Encrypted message format stored on-chain.
///
/// This structure is JSON-serialized for storage and transmission.
///
/// # Fields
///
/// * `version` - Message format version (currently 1)
/// * `keypair_type` - Type of keypair used (sr25519 or ed25519)
/// * `algorithm` - Encryption algorithm used (xchacha20, aesgcm256, or chacha20)
/// * `from` - Sender's public key in base58 encoding
/// * `nonce` - AEAD nonce in base64 encoding (size depends on algorithm)
/// * `ciphertext` - Encrypted data in base64 encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    /// Message format version
    pub version: u8,
    /// Keypair type (for backward compat, defaults to sr25519)
    #[serde(default)]
    pub keypair_type: KeypairType,
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

/// A shared secret computed via ECDH.
///
/// This struct holds the shared secret derived from keypair agreement
/// using either Ristretto255 (SR25519) or X25519 (ED25519) operations.
/// It provides methods for deriving encryption keys using HKDF.
///
/// # Security
///
/// The shared secret should be treated as sensitive cryptographic material
/// and not exposed directly. Use the provided methods to derive keys.
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{SharedSecret, KeypairType};
/// use schnorrkel::{SecretKey, PublicKey};
///
/// # fn example() -> anyhow::Result<()> {
/// let my_secret = SecretKey::from_bytes(&[0u8; 64])?;
/// let their_public = PublicKey::from_bytes(&[0u8; 32])?;
///
/// // Derive shared secret with SR25519
/// let shared = SharedSecret::new_sr25519(&my_secret, &their_public)?;
///
/// // Derive encryption key
/// let key = shared.derive_encryption_key(crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305)?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct SharedSecret {
    secret: [u8; 32],
    keypair_type: KeypairType,
}

impl SharedSecret {
    /// Perform ECDH on Ristretto255 to compute a shared secret (SR25519).
    ///
    /// Since sr25519 uses Ristretto255 group (not directly compatible with X25519),
    /// we use Ristretto255-based key agreement:
    /// 1. Compute scalar multiplication: secret_scalar * public_point
    /// 2. Hash the compressed result for uniform distribution
    ///
    /// # Arguments
    ///
    /// * `secret_key` - Our SR25519 secret key for DH
    /// * `public_key` - Their SR25519 public key to compute shared secret with
    ///
    /// # Returns
    ///
    /// Returns a `SharedSecret` wrapping the 32-byte shared secret
    ///
    /// # Errors
    ///
    /// Returns error if the public key cannot be decompressed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libcps::crypto::SharedSecret;
    /// use schnorrkel::{SecretKey, PublicKey};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let my_secret = SecretKey::from_bytes(&[0u8; 64])?;
    /// let their_public = PublicKey::from_bytes(&[0u8; 32])?;
    ///
    /// let shared = SharedSecret::new_sr25519(&my_secret, &their_public)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_sr25519(secret_key: &SecretKey, public_key: &PublicKey) -> Result<Self> {
        use curve25519_dalek::ristretto::CompressedRistretto;
        use curve25519_dalek::scalar::Scalar;
        use sha2::{Digest, Sha512};
        
        // Get secret scalar (first 32 bytes of secret key)
        let secret_bytes = secret_key.to_bytes();
        let mut scalar_bytes = [0u8; 32];
        scalar_bytes.copy_from_slice(&secret_bytes[..32]);
        
        // Create scalar (schnorrkel uses Ristretto255 internally)
        let scalar = Scalar::from_bytes_mod_order(scalar_bytes);
        
        // Get public key as Ristretto point
        // schnorrkel PublicKey is compressed Ristretto255
        let public_compressed = CompressedRistretto(public_key.to_bytes());
        let public_point = public_compressed
            .decompress()
            .ok_or_else(|| anyhow!("Failed to decompress Ristretto255 public key"))?;
        
        // Perform scalar multiplication on Ristretto255
        let shared_point = scalar * public_point;
        
        // Compress the result and hash it for uniform distribution
        // This ensures both parties get the same result
        let shared_compressed = shared_point.compress();
        
        // Use SHA-512 to derive a uniform 64-byte output, then take first 32 bytes
        // This matches how Substrate typically handles key agreement
        let mut hasher = Sha512::new();
        hasher.update(b"robonomics-cps-ecdh");
        hasher.update(shared_compressed.as_bytes());
        let hash_output = hasher.finalize();
        
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash_output[..32]);
        
        Ok(Self {
            secret: result,
            keypair_type: KeypairType::Sr25519,
        })
    }

    /// Perform X25519 ECDH to compute a shared secret (ED25519).
    ///
    /// This method converts ED25519 keys to Curve25519 (Montgomery form)
    /// using the birationally equivalent Edwards ↔ Montgomery curve conversion,
    /// then performs X25519 ECDH.
    ///
    /// # Arguments
    ///
    /// * `secret_key` - Our ED25519 secret key for DH
    /// * `public_key` - Their ED25519 public key to compute shared secret with
    ///
    /// # Returns
    ///
    /// Returns a `SharedSecret` wrapping the 32-byte shared secret
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libcps::crypto::SharedSecret;
    /// use ed25519_dalek::{SigningKey, VerifyingKey};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let my_secret = SigningKey::from_bytes(&[0u8; 32]);
    /// let their_public_bytes = [0u8; 32];
    /// let their_public = VerifyingKey::from_bytes(&their_public_bytes)?;
    ///
    /// let shared = SharedSecret::new_ed25519(&my_secret, &their_public)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_ed25519(
        secret_key: &ed25519_dalek::SigningKey,
        public_key: &ed25519_dalek::VerifyingKey,
    ) -> Result<Self> {
        // Convert ED25519 secret key to X25519 static secret
        // This uses the properly clamped scalar bytes from ED25519
        let scalar_bytes = secret_key.to_scalar_bytes();
        let my_x25519_secret = x25519_dalek::StaticSecret::from(scalar_bytes);
        
        // Convert ED25519 public key to X25519 public key
        // This uses the Montgomery form of the Edwards point
        let montgomery_point = public_key.to_montgomery();
        let their_x25519_public = x25519_dalek::PublicKey::from(montgomery_point.to_bytes());
        
        // Perform X25519 ECDH
        let shared_secret = my_x25519_secret.diffie_hellman(&their_x25519_public);
        
        Ok(Self {
            secret: *shared_secret.as_bytes(),
            keypair_type: KeypairType::Ed25519,
        })
    }

    /// Create shared secret using the old method signature (for backward compatibility).
    ///
    /// This is deprecated in favor of `new_sr25519()`.
    ///
    /// # Deprecated
    ///
    /// Use `new_sr25519()` instead.
    #[deprecated(since = "0.1.0", note = "Use new_sr25519() instead")]
    pub fn new(secret_key: &SecretKey, public_key: &PublicKey) -> Result<Self> {
        Self::new_sr25519(secret_key, public_key)
    }

    /// Get the keypair type used for this shared secret.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libcps::crypto::{SharedSecret, KeypairType};
    /// use schnorrkel::{SecretKey, PublicKey};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let my_secret = SecretKey::from_bytes(&[0u8; 64])?;
    /// let their_public = PublicKey::from_bytes(&[0u8; 32])?;
    ///
    /// let shared = SharedSecret::new_sr25519(&my_secret, &their_public)?;
    /// assert_eq!(shared.keypair_type(), KeypairType::Sr25519);
    /// # Ok(())
    /// # }
    /// ```
    pub fn keypair_type(&self) -> KeypairType {
        self.keypair_type
    }

    /// Derive an encryption key from the shared secret using HKDF-SHA256.
    ///
    /// Uses HKDF (HMAC-based Key Derivation Function) with SHA256 to derive
    /// a 32-byte encryption key suitable for the specified algorithm.
    /// The keypair type is included in the info string for domain separation.
    ///
    /// # Arguments
    ///
    /// * `algorithm` - The encryption algorithm to derive a key for
    ///
    /// # Returns
    ///
    /// Returns 32-byte encryption key
    ///
    /// # Errors
    ///
    /// Returns error if HKDF expansion fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libcps::crypto::{SharedSecret, EncryptionAlgorithm};
    /// use schnorrkel::{SecretKey, PublicKey};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let my_secret = SecretKey::from_bytes(&[0u8; 64])?;
    /// let their_public = PublicKey::from_bytes(&[0u8; 32])?;
    ///
    /// let shared = SharedSecret::new_sr25519(&my_secret, &their_public)?;
    /// let encryption_key = shared.derive_encryption_key(EncryptionAlgorithm::XChaCha20Poly1305)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn derive_encryption_key(&self, algorithm: crate::crypto::EncryptionAlgorithm) -> Result<[u8; 32]> {
        // Include keypair type in info string for domain separation
        let info = format!(
            "robonomics-cps-{}-{}",
            self.keypair_type.info_suffix(),
            std::str::from_utf8(algorithm.info_string())
                .unwrap_or("unknown")
                .trim_start_matches("robonomics-cps-")
        );
        
        let hkdf = Hkdf::<Sha256>::new(None, &self.secret);
        let mut okm = [0u8; 32];
        hkdf.expand(info.as_bytes(), &mut okm)
            .map_err(|e| anyhow!("HKDF expansion failed: {e}"))?;
        Ok(okm)
    }

    /// Get a reference to the raw shared secret bytes.
    ///
    /// # Security Warning
    ///
    /// The shared secret should be treated as sensitive cryptographic material.
    /// Prefer using `derive_encryption_key()` instead of accessing raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.secret
    }
}

impl AsRef<[u8]> for SharedSecret {
    fn as_ref(&self) -> &[u8] {
        &self.secret
    }
}

impl std::fmt::Debug for SharedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedSecret")
            .field("secret", &"[REDACTED]")
            .field("keypair_type", &self.keypair_type)
            .finish()
    }
}

/// Encrypt data using sr25519 → XChaCha20-Poly1305 scheme.
///
/// # Process
///
/// 1. Derive shared secret from sender's secret key and receiver's public key
/// 2. Use HKDF-SHA256 to derive 32-byte encryption key from shared secret
/// 3. Encrypt plaintext with XChaCha20-Poly1305 using random 24-byte nonce
/// 4. Return JSON-encoded message with version, sender, nonce, and ciphertext
///
/// # Arguments
///
/// * `plaintext` - Data to encrypt
/// * `sender_secret` - Sender's sr25519 secret key
/// * `receiver_public` - Receiver's sr25519 public key (32 bytes)
///
/// # Returns
///
/// JSON-encoded [`EncryptedMessage`] with base64-encoded nonce and ciphertext
///
/// # Errors
///
/// Returns an error if:
/// - Receiver's public key is invalid
/// - HKDF expansion fails
/// - Encryption fails
/// - JSON serialization fails
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::encrypt;
/// use schnorrkel::SecretKey;
///
/// # fn example() -> anyhow::Result<()> {
/// let sender_secret = SecretKey::from_bytes(&[0u8; 64])?;
/// let receiver_public = [0u8; 32];
/// let plaintext = b"secret message";
///
/// let encrypted = encrypt(plaintext, &sender_secret, &receiver_public)?;
/// # Ok(())
/// # }
/// ```
pub fn encrypt(
    plaintext: &[u8],
    sender_secret: &SecretKey,
    receiver_public: &[u8; 32],
) -> Result<Vec<u8>> {
    encrypt_with_algorithm(
        plaintext,
        sender_secret,
        receiver_public,
        crate::crypto::EncryptionAlgorithm::default(),
    )
}

/// Encrypt data using sr25519 → AEAD scheme with specified algorithm.
///
/// # Process
///
/// 1. Derive shared secret from sender's secret key and receiver's public key
/// 2. Use HKDF-SHA256 to derive 32-byte encryption key from shared secret
/// 3. Encrypt plaintext with specified AEAD cipher using random nonce
/// 4. Return JSON-encoded message with version, keypair type, algorithm, sender, nonce, and ciphertext
///
/// # Arguments
///
/// * `plaintext` - Data to encrypt
/// * `sender_secret` - Sender's sr25519 secret key
/// * `receiver_public` - Receiver's sr25519 public key (32 bytes)
/// * `algorithm` - AEAD encryption algorithm to use
///
/// # Returns
///
/// JSON-encoded [`EncryptedMessage`] with base64-encoded nonce and ciphertext
///
/// # Errors
///
/// Returns an error if:
/// - Receiver's public key is invalid
/// - HKDF expansion fails
/// - Encryption fails
/// - JSON serialization fails
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{encrypt_with_algorithm, EncryptionAlgorithm};
/// use schnorrkel::SecretKey;
///
/// # fn example() -> anyhow::Result<()> {
/// let sender_secret = SecretKey::from_bytes(&[0u8; 64])?;
/// let receiver_public = [0u8; 32];
/// let plaintext = b"secret message";
///
/// let encrypted = encrypt_with_algorithm(
///     plaintext,
///     &sender_secret,
///     &receiver_public,
///     EncryptionAlgorithm::AesGcm256
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn encrypt_with_algorithm(
    plaintext: &[u8],
    sender_secret: &SecretKey,
    receiver_public: &[u8; 32],
    algorithm: crate::crypto::EncryptionAlgorithm,
) -> Result<Vec<u8>> {
    use base64::{engine::general_purpose, Engine as _};

    // Step 1: Parse receiver's public key
    let receiver_pubkey = PublicKey::from_bytes(receiver_public)
        .map_err(|e| anyhow!("Invalid receiver public key: {e}"))?;
    
    // Step 2: Derive shared secret using ECDH (SR25519)
    let shared_secret = SharedSecret::new_sr25519(sender_secret, &receiver_pubkey)?;
    
    // Step 3: Derive encryption key using HKDF
    let encryption_key = shared_secret.derive_encryption_key(algorithm)?;

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

    // Step 5: Create message structure
    let sender_public = sender_secret.to_public();
    let algorithm_str = match algorithm {
        crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305 => "xchacha20",
        crate::crypto::EncryptionAlgorithm::AesGcm256 => "aesgcm256",
        crate::crypto::EncryptionAlgorithm::ChaCha20Poly1305 => "chacha20",
    };
    
    let message = EncryptedMessage {
        version: 1,
        keypair_type: shared_secret.keypair_type(),
        algorithm: algorithm_str.to_string(),
        from: bs58::encode(sender_public.to_bytes()).into_string(),
        nonce: general_purpose::STANDARD.encode(&nonce_bytes),
        ciphertext: general_purpose::STANDARD.encode(&ciphertext),
    };

    // Serialize to JSON
    serde_json::to_vec(&message).map_err(|e| anyhow!("JSON serialization failed: {e}"))
}

/// Encrypt data using ed25519 → AEAD scheme with specified algorithm.
///
/// This function uses ED25519 keys with X25519 ECDH for key agreement.
///
/// # Process
///
/// 1. Derive shared secret from sender's ED25519 key and receiver's ED25519 key using X25519
/// 2. Use HKDF-SHA256 to derive 32-byte encryption key from shared secret
/// 3. Encrypt plaintext with specified AEAD cipher using random nonce
/// 4. Return JSON-encoded message with version, keypair type, algorithm, sender, nonce, and ciphertext
///
/// # Arguments
///
/// * `plaintext` - Data to encrypt
/// * `sender_secret` - Sender's ED25519 secret key
/// * `receiver_public` - Receiver's ED25519 public key (32 bytes)
/// * `algorithm` - AEAD encryption algorithm to use
///
/// # Returns
///
/// JSON-encoded [`EncryptedMessage`] with base64-encoded nonce and ciphertext
///
/// # Errors
///
/// Returns an error if:
/// - Receiver's public key is invalid
/// - HKDF expansion fails
/// - Encryption fails
/// - JSON serialization fails
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{encrypt_with_algorithm_ed25519, EncryptionAlgorithm};
/// use ed25519_dalek::SigningKey;
///
/// # fn example() -> anyhow::Result<()> {
/// let sender_secret = SigningKey::from_bytes(&[0u8; 32]);
/// let receiver_public = [0u8; 32];
/// let plaintext = b"secret message";
///
/// let encrypted = encrypt_with_algorithm_ed25519(
///     plaintext,
///     &sender_secret,
///     &receiver_public,
///     EncryptionAlgorithm::XChaCha20Poly1305
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn encrypt_with_algorithm_ed25519(
    plaintext: &[u8],
    sender_secret: &ed25519_dalek::SigningKey,
    receiver_public: &[u8; 32],
    algorithm: crate::crypto::EncryptionAlgorithm,
) -> Result<Vec<u8>> {
    use base64::{engine::general_purpose, Engine as _};

    // Step 1: Parse receiver's public key
    let receiver_pubkey = ed25519_dalek::VerifyingKey::from_bytes(receiver_public)
        .map_err(|e| anyhow!("Invalid receiver ED25519 public key: {e}"))?;
    
    // Step 2: Derive shared secret using X25519 ECDH
    let shared_secret = SharedSecret::new_ed25519(sender_secret, &receiver_pubkey)?;
    
    // Step 3: Derive encryption key using HKDF
    let encryption_key = shared_secret.derive_encryption_key(algorithm)?;

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

    // Step 5: Create message structure
    let sender_public = sender_secret.verifying_key();
    let algorithm_str = match algorithm {
        crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305 => "xchacha20",
        crate::crypto::EncryptionAlgorithm::AesGcm256 => "aesgcm256",
        crate::crypto::EncryptionAlgorithm::ChaCha20Poly1305 => "chacha20",
    };
    
    let message = EncryptedMessage {
        version: 1,
        keypair_type: shared_secret.keypair_type(),
        algorithm: algorithm_str.to_string(),
        from: bs58::encode(sender_public.to_bytes()).into_string(),
        nonce: general_purpose::STANDARD.encode(&nonce_bytes),
        ciphertext: general_purpose::STANDARD.encode(&ciphertext),
    };

    // Serialize to JSON
    serde_json::to_vec(&message).map_err(|e| anyhow!("JSON serialization failed: {e}"))
}

/// Decrypt data using sr25519 → AEAD scheme (algorithm auto-detected).
///
/// # Process
///
/// 1. Parse JSON-encoded encrypted message
/// 2. Detect encryption algorithm from message
/// 3. Derive shared secret from receiver's secret key and sender's public key
/// 4. Use HKDF-SHA256 to derive encryption key with algorithm-specific info
/// 5. Decrypt ciphertext with appropriate AEAD cipher
///
/// # Arguments
///
/// * `encrypted_data` - JSON-encoded [`EncryptedMessage`]
/// * `receiver_secret` - Receiver's sr25519 secret key
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
/// - HKDF expansion fails
/// - Cannot decode base64 nonce or ciphertext
/// - Decryption fails (wrong key or corrupted data)
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{encrypt, decrypt};
/// use schnorrkel::SecretKey;
///
/// # fn example() -> anyhow::Result<()> {
/// let sender_secret = SecretKey::from_bytes(&[0u8; 64])?;
/// let receiver_secret = SecretKey::from_bytes(&[1u8; 64])?;
/// let receiver_public = receiver_secret.to_public().to_bytes();
/// let plaintext = b"secret message";
///
/// let encrypted = encrypt(plaintext, &sender_secret, &receiver_public)?;
/// let decrypted = decrypt(&encrypted, &receiver_secret)?;
///
/// assert_eq!(plaintext, &decrypted[..]);
/// # Ok(())
/// # }
/// ```
pub fn decrypt(encrypted_data: &[u8], receiver_secret: &SecretKey) -> Result<Vec<u8>> {
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

    // Step 3: Decode sender's public key
    let sender_public_bytes = bs58::decode(&message.from)
        .into_vec()
        .map_err(|e| anyhow!("Failed to decode sender public key: {e}"))?;
    
    if sender_public_bytes.len() != 32 {
        return Err(anyhow!("Invalid sender public key length"));
    }
    
    let mut sender_pk_array = [0u8; 32];
    sender_pk_array.copy_from_slice(&sender_public_bytes);
    
    // Step 4: Derive shared secret based on keypair type
    let shared_secret = match message.keypair_type {
        KeypairType::Sr25519 => {
            let sender_public = PublicKey::from_bytes(&sender_pk_array)
                .map_err(|e| anyhow!("Invalid sender SR25519 public key: {e}"))?;
            SharedSecret::new_sr25519(receiver_secret, &sender_public)?
        }
        KeypairType::Ed25519 => {
            return Err(anyhow!(
                "Cannot decrypt ED25519 message with SR25519 key. Use decrypt_ed25519() instead."
            ));
        }
    };
    
    // Step 5: Derive encryption key using HKDF
    let encryption_key = shared_secret.derive_encryption_key(algorithm)?;

    // Step 6: Decode nonce and ciphertext
    let nonce_bytes = general_purpose::STANDARD
        .decode(&message.nonce)
        .map_err(|e| anyhow!("Failed to decode nonce: {e}"))?;

    let ciphertext = general_purpose::STANDARD
        .decode(&message.ciphertext)
        .map_err(|e| anyhow!("Failed to decode ciphertext: {e}"))?;

    // Step 7: Decrypt with appropriate algorithm
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

/// Decrypt data using ed25519 → AEAD scheme (algorithm auto-detected).
///
/// # Process
///
/// 1. Parse JSON-encoded encrypted message
/// 2. Detect encryption algorithm from message
/// 3. Verify message was encrypted with ED25519
/// 4. Derive shared secret from receiver's ED25519 key and sender's ED25519 key
/// 5. Use HKDF-SHA256 to derive encryption key with algorithm-specific info
/// 6. Decrypt ciphertext with appropriate AEAD cipher
///
/// # Arguments
///
/// * `encrypted_data` - JSON-encoded [`EncryptedMessage`]
/// * `receiver_secret` - Receiver's ED25519 secret key
///
/// # Returns
///
/// Decrypted plaintext bytes
///
/// # Errors
///
/// Returns an error if:
/// - Cannot parse JSON message
/// - Message was not encrypted with ED25519
/// - Unsupported message version or algorithm
/// - Invalid sender public key
/// - HKDF expansion fails
/// - Cannot decode base64 nonce or ciphertext
/// - Decryption fails (wrong key or corrupted data)
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{encrypt_with_algorithm_ed25519, decrypt_ed25519, EncryptionAlgorithm};
/// use ed25519_dalek::SigningKey;
///
/// # fn example() -> anyhow::Result<()> {
/// let sender_secret = SigningKey::from_bytes(&[0u8; 32]);
/// let receiver_secret = SigningKey::from_bytes(&[1u8; 32]);
/// let receiver_public = receiver_secret.verifying_key().to_bytes();
/// let plaintext = b"secret message";
///
/// let encrypted = encrypt_with_algorithm_ed25519(
///     plaintext,
///     &sender_secret,
///     &receiver_public,
///     EncryptionAlgorithm::XChaCha20Poly1305
/// )?;
/// let decrypted = decrypt_ed25519(&encrypted, &receiver_secret)?;
///
/// assert_eq!(plaintext, &decrypted[..]);
/// # Ok(())
/// # }
/// ```
pub fn decrypt_ed25519(
    encrypted_data: &[u8],
    receiver_secret: &ed25519_dalek::SigningKey,
) -> Result<Vec<u8>> {
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

    // Step 3: Decode sender's public key
    let sender_public_bytes = bs58::decode(&message.from)
        .into_vec()
        .map_err(|e| anyhow!("Failed to decode sender public key: {e}"))?;
    
    if sender_public_bytes.len() != 32 {
        return Err(anyhow!("Invalid sender public key length"));
    }
    
    let mut sender_pk_array = [0u8; 32];
    sender_pk_array.copy_from_slice(&sender_public_bytes);
    
    // Step 4: Derive shared secret based on keypair type
    let shared_secret = match message.keypair_type {
        KeypairType::Ed25519 => {
            let sender_public = ed25519_dalek::VerifyingKey::from_bytes(&sender_pk_array)
                .map_err(|e| anyhow!("Invalid sender ED25519 public key: {e}"))?;
            SharedSecret::new_ed25519(receiver_secret, &sender_public)?
        }
        KeypairType::Sr25519 => {
            return Err(anyhow!(
                "Cannot decrypt SR25519 message with ED25519 key. Use decrypt() instead."
            ));
        }
    };
    
    // Step 5: Derive encryption key using HKDF
    let encryption_key = shared_secret.derive_encryption_key(algorithm)?;

    // Step 6: Decode nonce and ciphertext
    let nonce_bytes = general_purpose::STANDARD
        .decode(&message.nonce)
        .map_err(|e| anyhow!("Failed to decode nonce: {e}"))?;

    let ciphertext = general_purpose::STANDARD
        .decode(&message.ciphertext)
        .map_err(|e| anyhow!("Failed to decode ciphertext: {e}"))?;

    // Step 7: Decrypt with appropriate algorithm
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
        ).unwrap();
        
        // Encrypted data should not be empty
        assert!(!encrypted.is_empty());
        
        // Encrypted data should be valid JSON
        let _: EncryptedMessage = serde_json::from_slice(&encrypted).unwrap();
        
        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret).unwrap();
        
        // Decrypted should match original plaintext
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Same message";
        
        // Encrypt same message twice
        let encrypted1 = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        let encrypted2 = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Should produce different ciphertexts due to random nonces
        assert_ne!(encrypted1, encrypted2);
        
        // But both should decrypt to the same plaintext
        let decrypted1 = decrypt(&encrypted1, &receiver.secret).unwrap();
        let decrypted2 = decrypt(&encrypted2, &receiver.secret).unwrap();
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
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Try to decrypt with wrong key
        let result = decrypt(&encrypted, &wrong_receiver.secret);
        
        // Should fail
        assert!(result.is_err());
    }
    
    #[test]
    fn test_decrypt_with_corrupted_data_fails() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test message";
        
        // Encrypt
        let mut encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Corrupt the data
        if encrypted.len() > 10 {
            encrypted[10] ^= 0xFF;
        }
        
        // Try to decrypt corrupted data
        let result = decrypt(&encrypted, &receiver.secret);
        
        // Should fail (either parse error or authentication failure)
        assert!(result.is_err());
    }
    
    #[test]
    fn test_encrypt_empty_message() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"";
        
        // Encrypt empty message
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret).unwrap();
        
        // Should get empty message back
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_encrypt_large_message() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = vec![42u8; 10000]; // 10KB message
        
        // Encrypt
        let encrypted = encrypt(&plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret).unwrap();
        
        // Should match
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_encrypted_message_structure() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test";
        
        // Encrypt
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
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
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Parse and modify version
        let mut message: EncryptedMessage = serde_json::from_slice(&encrypted).unwrap();
        message.version = 2;
        let modified = serde_json::to_vec(&message).unwrap();
        
        // Try to decrypt
        let result = decrypt(&modified, &receiver.secret);
        
        // Should fail due to unsupported version
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported encryption version"));
    }

    // ========================================================================
    // ED25519 Tests
    // ========================================================================
    
    /// Generate a test ED25519 keypair from a seed
    fn test_ed25519_keypair(seed: u8) -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
    }
    
    #[test]
    fn test_ed25519_shared_secret_creation() {
        // Create two ED25519 keypairs
        let alice = test_ed25519_keypair(1);
        let bob = test_ed25519_keypair(2);
        
        // Derive shared secrets from both sides
        let shared_alice_bob = SharedSecret::new_ed25519(&alice, &bob.verifying_key()).unwrap();
        let shared_bob_alice = SharedSecret::new_ed25519(&bob, &alice.verifying_key()).unwrap();
        
        // Shared secrets should be identical
        assert_eq!(shared_alice_bob.as_bytes(), shared_bob_alice.as_bytes());
        
        // Shared secret should be 32 bytes
        assert_eq!(shared_alice_bob.as_bytes().len(), 32);
        
        // Shared secret should not be all zeros
        assert_ne!(shared_alice_bob.as_bytes(), &[0u8; 32]);
        
        // Verify keypair type
        assert_eq!(shared_alice_bob.keypair_type(), KeypairType::Ed25519);
    }
    
    #[test]
    fn test_ed25519_shared_secret_different_pairs() {
        // Create three ED25519 keypairs
        let alice = test_ed25519_keypair(1);
        let bob = test_ed25519_keypair(2);
        let charlie = test_ed25519_keypair(3);
        
        // Derive different shared secrets
        let shared_alice_bob = SharedSecret::new_ed25519(&alice, &bob.verifying_key()).unwrap();
        let shared_alice_charlie = SharedSecret::new_ed25519(&alice, &charlie.verifying_key()).unwrap();
        
        // Different pairs should produce different shared secrets
        assert_ne!(shared_alice_bob.as_bytes(), shared_alice_charlie.as_bytes());
    }
    
    #[test]
    fn test_ed25519_derive_encryption_key() {
        let alice = test_ed25519_keypair(1);
        let bob = test_ed25519_keypair(2);
        let shared_secret = SharedSecret::new_ed25519(&alice, &bob.verifying_key()).unwrap();
        
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
    fn test_ed25519_encrypt_decrypt_roundtrip() {
        // Create sender and receiver ED25519 keypairs
        let sender = test_ed25519_keypair(1);
        let receiver = test_ed25519_keypair(2);
        
        let plaintext = b"Hello, Robonomics CPS with ED25519!";
        
        // Encrypt with ED25519
        let encrypted = encrypt_with_algorithm_ed25519(
            plaintext,
            &sender,
            &receiver.verifying_key().to_bytes(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
        ).unwrap();
        
        // Encrypted data should not be empty
        assert!(!encrypted.is_empty());
        
        // Encrypted data should be valid JSON
        let message: EncryptedMessage = serde_json::from_slice(&encrypted).unwrap();
        assert_eq!(message.keypair_type, KeypairType::Ed25519);
        assert_eq!(message.algorithm, "xchacha20");
        
        // Decrypt with ED25519
        let decrypted = decrypt_ed25519(&encrypted, &receiver).unwrap();
        
        // Decrypted should match original plaintext
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_ed25519_all_algorithms() {
        let sender = test_ed25519_keypair(1);
        let receiver = test_ed25519_keypair(2);
        let plaintext = b"Test all algorithms with ED25519";
        
        // Test XChaCha20-Poly1305
        let encrypted_xchacha = encrypt_with_algorithm_ed25519(
            plaintext,
            &sender,
            &receiver.verifying_key().to_bytes(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
        ).unwrap();
        let decrypted_xchacha = decrypt_ed25519(&encrypted_xchacha, &receiver).unwrap();
        assert_eq!(decrypted_xchacha, plaintext);
        
        // Test AES-GCM-256
        let encrypted_aes = encrypt_with_algorithm_ed25519(
            plaintext,
            &sender,
            &receiver.verifying_key().to_bytes(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
        ).unwrap();
        let decrypted_aes = decrypt_ed25519(&encrypted_aes, &receiver).unwrap();
        assert_eq!(decrypted_aes, plaintext);
        
        // Test ChaCha20-Poly1305
        let encrypted_chacha = encrypt_with_algorithm_ed25519(
            plaintext,
            &sender,
            &receiver.verifying_key().to_bytes(),
            crate::crypto::EncryptionAlgorithm::ChaCha20Poly1305,
        ).unwrap();
        let decrypted_chacha = decrypt_ed25519(&encrypted_chacha, &receiver).unwrap();
        assert_eq!(decrypted_chacha, plaintext);
    }
    
    #[test]
    fn test_ed25519_decrypt_with_wrong_key_fails() {
        let sender = test_ed25519_keypair(1);
        let receiver = test_ed25519_keypair(2);
        let wrong_receiver = test_ed25519_keypair(3);
        
        let plaintext = b"Secret message for ED25519";
        
        // Encrypt for receiver
        let encrypted = encrypt_with_algorithm_ed25519(
            plaintext,
            &sender,
            &receiver.verifying_key().to_bytes(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
        ).unwrap();
        
        // Try to decrypt with wrong key
        let result = decrypt_ed25519(&encrypted, &wrong_receiver);
        
        // Should fail
        assert!(result.is_err());
    }
    
    #[test]
    fn test_ed25519_empty_message() {
        let sender = test_ed25519_keypair(1);
        let receiver = test_ed25519_keypair(2);
        let plaintext = b"";
        
        // Encrypt empty message
        let encrypted = encrypt_with_algorithm_ed25519(
            plaintext,
            &sender,
            &receiver.verifying_key().to_bytes(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
        ).unwrap();
        
        // Decrypt
        let decrypted = decrypt_ed25519(&encrypted, &receiver).unwrap();
        
        // Should get empty message back
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_ed25519_large_message() {
        let sender = test_ed25519_keypair(1);
        let receiver = test_ed25519_keypair(2);
        let plaintext = vec![42u8; 10000]; // 10KB message
        
        // Encrypt
        let encrypted = encrypt_with_algorithm_ed25519(
            &plaintext,
            &sender,
            &receiver.verifying_key().to_bytes(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
        ).unwrap();
        
        // Decrypt
        let decrypted = decrypt_ed25519(&encrypted, &receiver).unwrap();
        
        // Should match
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_mixed_keypair_types_fail() {
        // SR25519 keypair
        let sr_sender = test_keypair(1);
        let sr_receiver = test_keypair(2);
        
        // ED25519 keypair
        let ed_receiver = test_ed25519_keypair(2);
        
        let plaintext = b"Test mixed keypair types";
        
        // Encrypt with SR25519
        let encrypted_sr = encrypt(
            plaintext,
            &sr_sender.secret,
            &sr_receiver.public.to_bytes(),
        ).unwrap();
        
        // Try to decrypt SR25519 message with ED25519 key - should fail
        let result = decrypt_ed25519(&encrypted_sr, &ed_receiver);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot decrypt SR25519 message with ED25519 key"));
    }
    
    #[test]
    fn test_keypair_type_in_message() {
        // Test SR25519
        let sr_sender = test_keypair(1);
        let sr_receiver = test_keypair(2);
        let plaintext = b"Test keypair type";
        
        let encrypted_sr = encrypt(
            plaintext,
            &sr_sender.secret,
            &sr_receiver.public.to_bytes(),
        ).unwrap();
        
        let message_sr: EncryptedMessage = serde_json::from_slice(&encrypted_sr).unwrap();
        assert_eq!(message_sr.keypair_type, KeypairType::Sr25519);
        
        // Test ED25519
        let ed_sender = test_ed25519_keypair(1);
        let ed_receiver = test_ed25519_keypair(2);
        
        let encrypted_ed = encrypt_with_algorithm_ed25519(
            plaintext,
            &ed_sender,
            &ed_receiver.verifying_key().to_bytes(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
        ).unwrap();
        
        let message_ed: EncryptedMessage = serde_json::from_slice(&encrypted_ed).unwrap();
        assert_eq!(message_ed.keypair_type, KeypairType::Ed25519);
    }
    
    #[test]
    fn test_ed25519_sr25519_different_shared_secrets() {
        // This test verifies that SR25519 and ED25519 produce different shared secrets
        // even with the same seed, as expected
        let sr_alice = test_keypair(1);
        let sr_bob = test_keypair(2);
        
        let ed_alice = test_ed25519_keypair(1);
        let ed_bob = test_ed25519_keypair(2);
        
        let shared_sr = SharedSecret::new_sr25519(&sr_alice.secret, &sr_bob.public).unwrap();
        let shared_ed = SharedSecret::new_ed25519(&ed_alice, &ed_bob.verifying_key()).unwrap();
        
        // Different curve operations should produce different shared secrets
        assert_ne!(shared_sr.as_bytes(), shared_ed.as_bytes());
        
        // But both should produce valid 32-byte secrets
        assert_eq!(shared_sr.as_bytes().len(), 32);
        assert_eq!(shared_ed.as_bytes().len(), 32);
    }
}
