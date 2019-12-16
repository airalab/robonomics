///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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
//! Runtime module cryptographic primitives.

pub mod sr25519 {
	mod app_sr25519 {
		use app_crypto::{app_crypto, sr25519};
        use crate::KEY_TYPE;
		app_crypto!(sr25519, KEY_TYPE);
	}

	/// An i'm online keypair using sr25519 as its crypto.
	#[cfg(feature = "std")]
	pub type ProviderPair = app_sr25519::Pair;

	/// An i'm online signature using sr25519 as its crypto.
	pub type ProviderSignature = app_sr25519::Signature;

	/// An i'm online identifier using sr25519 as its crypto.
	pub type ProviderId = app_sr25519::Public;
}

pub mod ed25519 {
	mod app_ed25519 {
		use app_crypto::{app_crypto, ed25519};
        use crate::KEY_TYPE;
		app_crypto!(ed25519, KEY_TYPE);
	}

	/// An i'm online keypair using ed25519 as its crypto.
	#[cfg(feature = "std")]
	pub type ProviderPair = app_ed25519::Pair;

	/// An i'm online signature using ed25519 as its crypto.
	pub type ProviderSignature = app_ed25519::Signature;

	/// An i'm online identifier using ed25519 as its crypto.
	pub type ProviderId = app_ed25519::Public;
}
