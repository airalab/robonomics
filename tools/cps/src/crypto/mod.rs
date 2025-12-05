//! Encryption and key derivation utilities.
//!
//! This module provides encryption functions using the XChaCha20-Poly1305 AEAD cipher
//! with sr25519 key agreement and HKDF key derivation.

pub mod xchacha20;

pub use xchacha20::{decrypt, encrypt, EncryptedMessage};
