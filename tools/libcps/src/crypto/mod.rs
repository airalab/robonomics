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
//! - **Forward Secrecy (with ephemeral ECDH keys)**: When each session uses fresh
//!   ephemeral key pairs for ECDH, compromising one session's keys does not reveal
//!   past sessions
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
//! ```
//!
//! 4. Transmit message:
//!    The encrypted message is a versioned enum serialized with SCALE codec:
//!    
//! ```ignore
//! enum EncryptedMessage {
//!     V1 {
//!         algorithm: EncryptionAlgorithm,  // enum: XChaCha20Poly1305, AesGcm256, ChaCha20Poly1305
//!         from: [u8; 32],                  // sender's public key
//!         nonce: Vec<u8>,                  // 24 bytes for XChaCha20, 12 for AES-GCM/ChaCha20
//!         ciphertext: Vec<u8>,             // encrypted data with auth tag
//!     }
//! }
//! ```
//!    
//!    The versioned format allows future protocol upgrades:
//!    - Enum variants enable backward-compatible format changes
//!    - Currently only V1 variant is supported
//!    - SCALE codec provides efficient binary serialization for blockchain storage
//!    - `algorithm` field enables auto-detection of cipher used
//!    - `from` contains sender's 32-byte public key
//!
//! ```text
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

mod cipher;
mod types;

pub use cipher::Cipher;
pub use types::{CryptoScheme, EncryptedMessage, EncryptionAlgorithm};
