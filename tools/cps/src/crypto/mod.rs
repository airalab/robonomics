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
//! with sr25519 key agreement and HKDF key derivation.

pub mod cipher;
pub mod encryption;
pub mod keypair_type;
pub mod shared_secret;

pub use cipher::EncryptionAlgorithm;
pub use encryption::{decrypt, encrypt, EncryptedMessage};
pub use keypair_type::KeypairType;
pub use shared_secret::{DeriveSharedSecret, SharedSecret};
