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
//! Encryption and key derivation utilities.
//!
//! This module provides encryption functions using multiple AEAD ciphers
//! with Elliptic Curve Diffie-Hellman (ECDH) key agreement and HKDF key derivation.
//!
//! # Cryptographic Architecture
//!
//! The encryption scheme follows a hybrid approach combining asymmetric and symmetric cryptography:
//!
//! 1. **Key Agreement**: ECDH (Elliptic Curve Diffie-Hellman) using SR25519 or ED25519
//! 2. **Key Derivation**: HKDF-SHA256 (HMAC-based Key Derivation Function)
//! 3. **Encryption**: AEAD ciphers (XChaCha20-Poly1305, AES-256-GCM, or ChaCha20-Poly1305)
//!
//! ## HKDF Key Derivation
//!
//! The HKDF key derivation function (RFC 5869) is used to derive encryption keys from
//! the shared secret produced by ECDH. HKDF consists of two stages:
//!
//! ### 1. Extract Phase
//! ```text
//! PRK = HKDF-Extract(salt, IKM)
//! ```
//! Where:
//! - `salt`: A constant value `"robonomics-network"` for domain separation
//! - `IKM` (Input Keying Material): The shared secret from ECDH
//! - `PRK` (Pseudorandom Key): The extracted key material
//!
//! ### 2. Expand Phase
//! ```text
//! OKM = HKDF-Expand(PRK, info, L)
//! ```
//! Where:
//! - `PRK`: The pseudorandom key from extract phase
//! - `info`: Algorithm-specific context (e.g., "robonomics-cps-xchacha20poly1305")
//! - `L`: Desired output length (32 bytes for 256-bit keys)
//! - `OKM` (Output Keying Material): The final encryption key
//!
//! ## Security Properties
//!
//! ### Salt Purpose
//! The constant salt `"robonomics-network"` provides:
//! - **Domain Separation**: Keys derived for Robonomics network are distinct from other systems
//! - **Additional Structure**: Adds a fixed input to the key derivation process independent of the shared secret
//! - **Defense in Depth**: Provides security even if the shared secret has low entropy
//!
//! Note: The salt doesn't need to be secret or random. A constant application-specific
//! value is appropriate here since the public keys are already incorporated in the
//! ECDH shared secret derivation, making each key pair unique.
//!
//! ### Info String Purpose
//! The algorithm-specific info string provides:
//! - **Algorithm Binding**: Prevents key reuse across different encryption algorithms
//! - **Context Separation**: Keys for XChaCha20-Poly1305 are independent from AES-GCM keys
//! - **Protocol Flexibility**: Allows safe algorithm upgrades without key conflicts
//!
//! ## Security Guarantees
//!
//! This scheme provides:
//! - **Forward Secrecy**: Compromising one session doesn't compromise past sessions
//! - **Algorithm Agility**: Multiple AEAD algorithms supported without security loss
//! - **Domain Separation**: Keys are bound to the Robonomics network context
//! - **Key Independence**: Each algorithm and key pair combination produces unique keys
//!
//! # Example Flow
//!
//! ```text
//! Sender (Alice)                               Receiver (Bob)
//! ==============                               ===============
//! 
//! 1. ECDH Key Agreement:
//!    alice_secret + bob_public    ───────────────────>
//!    shared_secret = ECDH(alice_secret, bob_public)
//! 
//! 2. HKDF Key Derivation:
//!    salt = "robonomics-network"
//!    info = "robonomics-cps-xchacha20poly1305"
//!    encryption_key = HKDF(salt, shared_secret, info, 32)
//! 
//! 3. AEAD Encryption:
//!    nonce = random(24 bytes)
//!    ciphertext = XChaCha20Poly1305(encryption_key, nonce, plaintext)
//! 
//! 4. Transmit message:
//!    {from: alice_public, algorithm: "xchacha20poly1305",
//!     nonce: base64(nonce), ciphertext: base64(ciphertext)}
//!                           ───────────────────>
//! 
//! 5. Receiver verifies and derives same key:
//!    shared_secret = ECDH(bob_secret, alice_public)
//!    encryption_key = HKDF(salt, shared_secret, info, 32)
//! 
//! 6. AEAD Decryption:
//!    plaintext = XChaCha20Poly1305_Decrypt(encryption_key, nonce, ciphertext)
//! ```
//!
//! # References
//!
//! - RFC 5869: HMAC-based Extract-and-Expand Key Derivation Function (HKDF)
//! - RFC 7539: ChaCha20 and Poly1305 for IETF Protocols
//! - RFC 8439: ChaCha20-Poly1305 AEAD
//! - draft-irtf-cfrg-xchacha: XChaCha: eXtended-nonce ChaCha and AEAD_XChaCha20_Poly1305

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
use std::fmt;
use std::str::FromStr;

pub mod scheme;

pub use scheme::CryptoScheme;

/// HKDF salt for key derivation.
///
/// A constant salt value used in the HKDF extract phase when deriving encryption keys
/// from ECDH shared secrets. This value is `"robonomics-network"`.
///
/// # Purpose
///
/// The salt serves multiple security purposes:
///
/// 1. **Domain Separation**: Binds derived keys to the Robonomics network context,
///    ensuring keys cannot be reused or confused with other systems or protocols.
///
/// 2. **Structured Extraction**: Provides a fixed, protocol-specific input to HKDF's
///    extract phase, helping to condition the shared secret into a uniform pseudorandom
///    key without claiming to add independent entropy.
///
/// 3. **Defense in Depth**: Increases robustness against potential weaknesses in the
///    shared secret by strengthening the key derivation process and reducing the risk
///    of key material being misused across different contexts.
///
/// # Why a Constant Salt?
///
/// Unlike some cryptographic applications where a unique random salt per operation
/// is required, HKDF with a constant salt is appropriate here because:
///
/// - The ECDH shared secret is already unique per key pair combination
/// - The public keys (which vary) are used in the ECDH computation
/// - The info parameter provides per-algorithm domain separation
/// - A constant salt enables deterministic key derivation (same inputs → same key)
/// - Deterministic derivation allows both parties to independently compute the same key
///
/// # Security Note
///
/// The salt value is not secret and does not need to be transmitted. Both parties
/// use the same constant salt value when deriving encryption keys. The security
/// of the system relies on the secrecy of the private keys and the strength of
/// the ECDH key agreement, not on the salt's secrecy.
///
/// # References
///
/// RFC 5869 Section 3.1: "The 'salt' value is a non-secret random value; if not
/// provided, it is set to a string of HashLen zeros."
const HKDF_SALT: &[u8] = b"robonomics-network";

/// Supported AEAD encryption algorithms.
///
/// Each algorithm provides authenticated encryption with associated data (AEAD),
/// ensuring both confidentiality and integrity of encrypted data.
///
/// # Algorithms
///
/// - **XChaCha20Poly1305**: Extended nonce ChaCha20-Poly1305 (24-byte nonce)
///   - Best for: General purpose, portable, large nonce space
///   - Performance: ~680 MB/s (software)
///   - Nonce: 192 bits (collision-resistant)
///
/// - **AesGcm256**: AES-256 in Galois/Counter Mode (12-byte nonce)
///   - Best for: Hardware acceleration (AES-NI), high throughput
///   - Performance: ~2-3 GB/s (with AES-NI)
///   - Nonce: 96 bits (requires careful management)
///
/// - **ChaCha20Poly1305**: Standard ChaCha20-Poly1305 (12-byte nonce)
///   - Best for: Portable performance without hardware acceleration
///   - Performance: ~600 MB/s (software)
///   - Nonce: 96 bits (requires careful management)
///
/// # Examples
///
/// ```
/// use libcps::crypto::EncryptionAlgorithm;
/// use std::str::FromStr;
///
/// let algo = EncryptionAlgorithm::XChaCha20Poly1305;
/// assert_eq!(algo.info_string(), b"robonomics-cps-xchacha20poly1305");
///
/// let from_str = EncryptionAlgorithm::from_str("aesgcm256").unwrap();
/// assert_eq!(from_str, EncryptionAlgorithm::AesGcm256);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// XChaCha20-Poly1305 AEAD (24-byte nonce)
    XChaCha20Poly1305,
    /// AES-256-GCM AEAD (12-byte nonce)
    AesGcm256,
    /// ChaCha20-Poly1305 AEAD (12-byte nonce)
    ChaCha20Poly1305,
}

impl EncryptionAlgorithm {
    /// Get the HKDF info string for this algorithm.
    ///
    /// The info string is used for domain separation in HKDF key derivation,
    /// ensuring keys derived for different algorithms are independent.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::EncryptionAlgorithm;
    ///
    /// let algo = EncryptionAlgorithm::XChaCha20Poly1305;
    /// assert_eq!(algo.info_string(), b"robonomics-cps-xchacha20poly1305");
    /// ```
    pub fn info_string(&self) -> &'static [u8] {
        match self {
            Self::XChaCha20Poly1305 => b"robonomics-cps-xchacha20poly1305",
            Self::AesGcm256 => b"robonomics-cps-aesgcm256",
            Self::ChaCha20Poly1305 => b"robonomics-cps-chacha20poly1305",
        }
    }

    /// Get the nonce size in bytes for this algorithm.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::EncryptionAlgorithm;
    ///
    /// assert_eq!(EncryptionAlgorithm::XChaCha20Poly1305.nonce_size(), 24);
    /// assert_eq!(EncryptionAlgorithm::AesGcm256.nonce_size(), 12);
    /// assert_eq!(EncryptionAlgorithm::ChaCha20Poly1305.nonce_size(), 12);
    /// ```
    pub fn nonce_size(&self) -> usize {
        match self {
            Self::XChaCha20Poly1305 => 24,
            Self::AesGcm256 => 12,
            Self::ChaCha20Poly1305 => 12,
        }
    }

    /// Get the key size in bytes for this algorithm.
    ///
    /// All supported algorithms use 256-bit (32-byte) keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::EncryptionAlgorithm;
    ///
    /// assert_eq!(EncryptionAlgorithm::XChaCha20Poly1305.key_size(), 32);
    /// ```
    pub fn key_size(&self) -> usize {
        32 // All algorithms use 256-bit keys
    }

    /// Get a human-readable name for this algorithm.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::EncryptionAlgorithm;
    ///
    /// assert_eq!(EncryptionAlgorithm::XChaCha20Poly1305.name(), "XChaCha20-Poly1305");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            Self::XChaCha20Poly1305 => "XChaCha20-Poly1305",
            Self::AesGcm256 => "AES-256-GCM",
            Self::ChaCha20Poly1305 => "ChaCha20-Poly1305",
        }
    }
}

impl Default for EncryptionAlgorithm {
    fn default() -> Self {
        Self::XChaCha20Poly1305
    }
}

impl fmt::Display for EncryptionAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for EncryptionAlgorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "xchacha20" | "xchacha20poly1305" | "xchacha20-poly1305" => Ok(Self::XChaCha20Poly1305),
            "aesgcm256" | "aes-256-gcm" | "aes256gcm" | "aesgcm" => Ok(Self::AesGcm256),
            "chacha20" | "chacha20poly1305" | "chacha20-poly1305" => Ok(Self::ChaCha20Poly1305),
            _ => Err(format!(
                "Unknown encryption algorithm: '{s}'. Supported: xchacha20, aesgcm256, chacha20"
            )),
        }
    }
}

/// Encrypted message format stored on-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
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
///     EncryptionAlgorithm::XChaCha20Poly1305,
///     CryptoScheme::Sr25519,
/// ).unwrap();
///
/// let plaintext = b"secret message";
/// let receiver_public = &[0u8; 32]; // receiver's public key
/// let encrypted = cipher.encrypt(plaintext, receiver_public).unwrap();
/// let decrypted = cipher.decrypt(&encrypted, None).unwrap();
/// ```
pub struct Cipher {
    /// 32-byte secret key
    secret: [u8; 32],
    /// Cached public key (derived once in constructor)
    public_key: [u8; 32],
    /// Encryption algorithm
    algorithm: EncryptionAlgorithm,
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
    /// * `algorithm` - Encryption algorithm to use
    /// * `scheme` - Cryptographic scheme to use
    ///
    /// # Returns
    ///
    /// Returns a Cipher instance with the secret key
    ///
    /// # Errors
    ///
    /// Returns error if keypair parsing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libcps::crypto::{Cipher, EncryptionAlgorithm, CryptoScheme};
    ///
    /// let cipher = Cipher::new(
    ///     "//Alice".to_string(),
    ///     EncryptionAlgorithm::XChaCha20Poly1305,
    ///     CryptoScheme::Sr25519,
    /// ).unwrap();
    /// ```
    pub fn new(suri: String, algorithm: EncryptionAlgorithm, scheme: CryptoScheme) -> Result<Self> {
        let (secret, public_key) = match scheme {
            CryptoScheme::Sr25519 => {
                let pair = sp_core::sr25519::Pair::from_string(&suri, None)
                    .map_err(|e| anyhow!("Failed to parse SR25519 keypair: {:?}", e))?;
                let secret_bytes = pair.to_raw_vec();
                let mut secret = [0u8; 32];
                secret.copy_from_slice(&secret_bytes[..32]);
                // Derive public key using Pair interface
                let public_key = pair.public().0;
                (secret, public_key)
            }
            CryptoScheme::Ed25519 => {
                let pair = sp_core::ed25519::Pair::from_string(&suri, None)
                    .map_err(|e| anyhow!("Failed to parse ED25519 keypair: {:?}", e))?;
                let secret_bytes = pair.to_raw_vec();
                let mut secret = [0u8; 32];
                secret.copy_from_slice(&secret_bytes[..32]);
                // Derive public key using Pair interface
                let public_key = pair.public().0;
                (secret, public_key)
            }
        };
        Ok(Cipher {
            secret,
            public_key,
            algorithm,
            scheme,
        })
    }

    /// Get the encryption algorithm.
    pub fn algorithm(&self) -> EncryptionAlgorithm {
        self.algorithm
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
    /// Returns error if ECDH fails
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

    /// Derive encryption key from shared secret using HKDF-SHA256.
    ///
    /// This is a convenience wrapper around `derive_encryption_key_with_algorithm`
    /// that uses the cipher's configured algorithm.
    ///
    /// # Arguments
    ///
    /// * `shared_secret` - The 32-byte shared secret from ECDH key agreement
    ///
    /// # Returns
    ///
    /// Returns a 32-byte encryption key derived using HKDF with the cipher's
    /// configured algorithm.
    ///
    /// # See Also
    ///
    /// - [`derive_encryption_key_with_algorithm`] for detailed HKDF documentation
    fn derive_encryption_key(&self, shared_secret: &[u8; 32]) -> Result<[u8; 32]> {
        Self::derive_encryption_key_with_algorithm(shared_secret, &self.algorithm)
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

        // Step 2: Derive encryption key using HKDF with salt
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
        let sender_public = self.public_key();

        // Step 5: Create message structure
        let algorithm_str = match self.algorithm {
            EncryptionAlgorithm::XChaCha20Poly1305 => "xchacha20",
            EncryptionAlgorithm::AesGcm256 => "aesgcm256",
            EncryptionAlgorithm::ChaCha20Poly1305 => "chacha20",
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
    pub fn decrypt(
        &self,
        ciphertext: &[u8],
        expected_sender: Option<&[u8; 32]>,
    ) -> Result<Vec<u8>> {
        use base64::{engine::general_purpose, Engine as _};

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

        // Step 6: Derive encryption key using HKDF with salt
        let encryption_key =
            Self::derive_encryption_key_with_algorithm(&shared_secret, &algorithm)?;

        // Step 7: Decode nonce and ciphertext
        let nonce_bytes = general_purpose::STANDARD
            .decode(&message.nonce)
            .map_err(|e| anyhow!("Failed to decode nonce: {e}"))?;

        let ciphertext_bytes = general_purpose::STANDARD
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
                    .decrypt(nonce, ciphertext_bytes.as_ref())
                    .map_err(|e| anyhow!("XChaCha20 decryption failed: {e}"))
            }
            EncryptionAlgorithm::AesGcm256 => {
                if nonce_bytes.len() != 12 {
                    return Err(anyhow!("Invalid AES-GCM nonce length: expected 12 bytes"));
                }
                let nonce = AesNonce::from_slice(&nonce_bytes);
                let cipher = Aes256Gcm::new(&encryption_key.into());
                cipher
                    .decrypt(nonce, ciphertext_bytes.as_ref())
                    .map_err(|e| anyhow!("AES-GCM decryption failed: {e}"))
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
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
        let cipher = Cipher::new(
            "//Alice".to_string(),
            EncryptionAlgorithm::XChaCha20Poly1305,
            CryptoScheme::Sr25519,
        )
        .unwrap();

        assert_eq!(cipher.algorithm(), EncryptionAlgorithm::XChaCha20Poly1305);
        assert_eq!(cipher.scheme(), CryptoScheme::Sr25519);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_sr25519() {
        let cipher = Cipher::new(
            "//Alice".to_string(),
            EncryptionAlgorithm::XChaCha20Poly1305,
            CryptoScheme::Sr25519,
        )
        .unwrap();

        // Get Alice's public key for self-encryption
        let public_key = cipher.public_key();

        let plaintext = b"Hello, World!";
        let encrypted = cipher.encrypt(plaintext, &public_key).unwrap();
        let decrypted = cipher.decrypt(&encrypted, None).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_ed25519() {
        let cipher = Cipher::new(
            "//Alice".to_string(),
            EncryptionAlgorithm::AesGcm256,
            CryptoScheme::Ed25519,
        )
        .unwrap();

        // Get Alice's public key for self-encryption
        let public_key = cipher.public_key();

        let plaintext = b"Hello, World!";
        let encrypted = cipher.encrypt(plaintext, &public_key).unwrap();
        let decrypted = cipher.decrypt(&encrypted, None).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_cross_party_encryption_sr25519() {
        let alice = Cipher::new(
            "//Alice".to_string(),
            EncryptionAlgorithm::XChaCha20Poly1305,
            CryptoScheme::Sr25519,
        )
        .unwrap();

        let bob = Cipher::new(
            "//Bob".to_string(),
            EncryptionAlgorithm::XChaCha20Poly1305,
            CryptoScheme::Sr25519,
        )
        .unwrap();

        let bob_public = bob.public_key();
        let alice_public = alice.public_key();

        let plaintext = b"Secret from Alice to Bob";
        let encrypted = alice.encrypt(plaintext, &bob_public).unwrap();
        let decrypted = bob.decrypt(&encrypted, Some(&alice_public)).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_sender_verification_fails() {
        let alice = Cipher::new(
            "//Alice".to_string(),
            EncryptionAlgorithm::XChaCha20Poly1305,
            CryptoScheme::Sr25519,
        )
        .unwrap();

        let bob = Cipher::new(
            "//Bob".to_string(),
            EncryptionAlgorithm::XChaCha20Poly1305,
            CryptoScheme::Sr25519,
        )
        .unwrap();

        let charlie = Cipher::new(
            "//Charlie".to_string(),
            EncryptionAlgorithm::XChaCha20Poly1305,
            CryptoScheme::Sr25519,
        )
        .unwrap();

        let bob_public = bob.public_key();
        let charlie_public = charlie.public_key();

        let plaintext = b"From Alice";
        let encrypted = alice.encrypt(plaintext, &bob_public).unwrap();

        // Should fail: expecting message from Charlie, but it's from Alice
        let result = bob.decrypt(&encrypted, Some(&charlie_public));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("mismatch"));
    }

    #[test]
    fn test_derive_shared_secret_sr25519() {
        let alice = Cipher::new(
            "//Alice".to_string(),
            EncryptionAlgorithm::XChaCha20Poly1305,
            CryptoScheme::Sr25519,
        )
        .unwrap();

        let bob = Cipher::new(
            "//Bob".to_string(),
            EncryptionAlgorithm::XChaCha20Poly1305,
            CryptoScheme::Sr25519,
        )
        .unwrap();

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
        let alice = Cipher::new(
            "//Alice".to_string(),
            EncryptionAlgorithm::AesGcm256,
            CryptoScheme::Ed25519,
        )
        .unwrap();

        let bob = Cipher::new(
            "//Bob".to_string(),
            EncryptionAlgorithm::AesGcm256,
            CryptoScheme::Ed25519,
        )
        .unwrap();

        let bob_public = bob.public_key();
        let alice_public = alice.public_key();

        // Derive shared secrets
        let alice_shared = alice.derive_shared_secret(&bob_public).unwrap();
        let bob_shared = bob.derive_shared_secret(&alice_public).unwrap();

        // Shared secrets should match (Diffie-Hellman property)
        assert_eq!(alice_shared, bob_shared);
    }
}
