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
//! # Robonomics Claim Pallet
//!
//! A pallet for claiming tokens using Ethereum signatures.
//!
//! ## Overview
//!
//! This pallet enables users who hold Ethereum addresses to claim tokens on the Robonomics
//! parachain by proving ownership of their Ethereum address through ECDSA signatures.
//! This is particularly useful for token migrations from Ethereum to Substrate-based chains.
//!
//! ## Features
//!
//! - **Unsigned Claims**: Users can submit unsigned transactions with Ethereum signatures
//! - **Signature Verification**: Validates ECDSA signatures using Ethereum's personal_sign format
//! - **One-time Claims**: Each Ethereum address can only claim once
//! - **Root Management**: Claims can be added by root/governance
//!
//! ## Usage
//!
//! ### For Users
//!
//! To claim tokens, users need:
//! 1. An Ethereum address with an associated claim
//! 2. Control of the private key for that Ethereum address
//! 3. A destination account on the parachain
//!
//! The user signs their destination account ID with their Ethereum private key and submits
//! the signature via the `claim` extrinsic.
//!
//! ### For Governance
//!
//! New claims can be added via the `add_claim` extrinsic (requires root origin):
//! ```ignore
//! // Add a claim for an Ethereum address
//! Claims::add_claim(
//!     RuntimeOrigin::root(),
//!     ethereum_address,
//!     amount_to_claim,
//! )?;
//! ```
//!
//! ## Implementation Details
//!
//! The pallet stores claims in a map from Ethereum addresses to token amounts.
//! When a valid claim is processed:
//! 1. The signature is verified against the destination account
//! 2. Tokens are transferred from the pallet account to the destination
//! 3. The claim is removed from storage
//! 4. A `Claimed` event is emitted

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{format, string::String};
#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
use frame_support::{
    ensure,
    traits::{Currency, ExistenceRequirement, Get},
    weights::Weight,
    DefaultNoBound,
};
pub use pallet::*;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
use sp_io::{crypto::secp256k1_ecdsa_recover, hashing::keccak_256};
use sp_runtime::{
    traits::AccountIdConversion,
    transaction_validity::{
        InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
    },
    RuntimeDebug,
};

/// Balance type alias for easier reference throughout the pallet.
type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Weight information for pallet extrinsics.
///
/// Provides benchmark-derived weights for each extrinsic in the pallet.
pub trait WeightInfo {
    /// Weight for the `claim` extrinsic.
    fn claim() -> Weight;
    /// Weight for the `add_claim` extrinsic.
    fn add_claim() -> Weight;
}

/// Test weight implementation that returns zero weight for all operations.
///
/// Used in testing environments where actual weight calculations are not needed.
pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
    fn claim() -> Weight {
        Weight::zero()
    }
    fn add_claim() -> Weight {
        Weight::zero()
    }
}

/// An Ethereum address (i.e. 20 bytes, used to represent an Ethereum account).
///
/// This gets serialized to the 0x-prefixed hex representation.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Default,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct EthereumAddress(pub [u8; 20]);

impl Serialize for EthereumAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex: String = rustc_hex::ToHex::to_hex(&self.0[..]);
        serializer.serialize_str(&format!("0x{}", hex))
    }
}

impl<'de> Deserialize<'de> for EthereumAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let base_string = String::deserialize(deserializer)?;
        let offset = if base_string.starts_with("0x") { 2 } else { 0 };
        let s = &base_string[offset..];
        if s.len() != 40 {
            Err(serde::de::Error::custom(
                "Bad length of Ethereum address (should be 42 including '0x')",
            ))?;
        }
        let raw: Vec<u8> = rustc_hex::FromHex::from_hex(s)
            .map_err(|e| serde::de::Error::custom(format!("{:?}", e)))?;
        let mut r = Self::default();
        r.0.copy_from_slice(&raw);
        Ok(r)
    }
}

impl AsRef<[u8]> for EthereumAddress {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

/// ECDSA signature from Ethereum.
///
/// Contains 64 bytes for the signature (r, s) and 1 byte for the recovery ID (v).
/// This matches the signature format produced by Ethereum's `personal_sign` and `eth_sign` RPC methods.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, TypeInfo, MaxEncodedLen)]
pub struct EcdsaSignature(pub [u8; 65]);

impl PartialEq for EcdsaSignature {
    fn eq(&self, other: &Self) -> bool {
        &self.0[..] == &other.0[..]
    }
}

impl core::fmt::Debug for EcdsaSignature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "EcdsaSignature({:?})", &self.0[..])
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_support::{traits::Currency, PalletId};
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Configuration trait for the Claim pallet.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Currency type for handling token transfers.
        type Currency: Currency<Self::AccountId>;
        
        /// Prefix string prepended to signed messages.
        ///
        /// This is used in the Ethereum signed message format to prevent signature replay attacks
        /// across different contexts. Example: b"Pay RWS to the Robonomics account:"
        #[pallet::constant]
        type Prefix: Get<&'static [u8]>;
        
        /// Pallet ID for deriving the pallet's account.
        ///
        /// The pallet account holds the claimable tokens before they are claimed.
        #[pallet::constant]
        type PalletId: Get<PalletId>;
        
        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Someone claimed token.
        Claimed {
            who: T::AccountId,
            ethereum_address: EthereumAddress,
            amount: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Invalid Ethereum signature.
        InvalidEthereumSignature,
        /// Ethereum address has no claim.
        SignerHasNoClaim,
    }

    /// Mapping of Ethereum addresses to their claimable token amounts.
    ///
    /// Once a claim is processed, the entry is removed from this map.
    #[pallet::storage]
    pub type Claims<T: Config> = StorageMap<_, Identity, EthereumAddress, BalanceOf<T>>;

    /// Genesis configuration for the Claim pallet.
    ///
    /// Allows setting up initial claims at chain genesis.
    #[pallet::genesis_config]
    #[derive(DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// List of initial claims as (Ethereum address, claimable amount) pairs.
        pub claims: Vec<(EthereumAddress, BalanceOf<T>)>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            self.claims
                .iter()
                .map(|(a, b)| (*a, *b))
                .for_each(|(a, b)| {
                    Claims::<T>::insert(a, b);
                });
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Make a claim to collect your token.
        ///
        /// The dispatch origin for this call must be _None_.
        ///
        /// Unsigned Validation:
        /// A call to claim is deemed valid if the signature provided matches
        /// the expected signed message of:
        ///
        /// > Ethereum Signed Message:
        /// > (configured prefix string)(address)
        ///
        /// and `address` matches the `dest` account.
        ///
        /// Parameters:
        /// - `dest`: The destination account to payout the claim.
        /// - `ethereum_signature`: The signature of an ethereum signed message matching the format
        ///   described above.
        ///
        /// <weight>
        /// The weight of this call is invariant over the input parameters.
        /// Weight includes logic to validate unsigned `claim` call.
        ///
        /// Total Complexity: O(1)
        /// </weight>
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::claim())]
        pub fn claim(
            origin: OriginFor<T>,
            dest: T::AccountId,
            ethereum_signature: EcdsaSignature,
        ) -> DispatchResult {
            ensure_none(origin)?;

            let data = dest.using_encoded(to_ascii_hex);
            let signer = Self::eth_recover(&ethereum_signature, &data)
                .ok_or(Error::<T>::InvalidEthereumSignature)?;

            Self::process_claim(signer, dest)?;
            Ok(())
        }

        /// Create a new claim to collect token.
        ///
        /// The dispatch origin for this call must be _Root_.
        ///
        /// Parameters:
        /// - `who`: The Ethereum address allowed to collect this claim.
        /// - `value`: The number of token that will be claimed.
        ///
        /// <weight>
        /// The weight of this call is invariant over the input parameters.
        ///
        /// Total Complexity: O(1)
        /// </weight>
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::add_claim())]
        pub fn add_claim(
            origin: OriginFor<T>,
            who: EthereumAddress,
            value: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            Claims::<T>::insert(who, value);
            Ok(())
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            const PRIORITY: u64 = 100;

            let maybe_signer = match call {
                // <weight>
                // The weight of this logic is included in the `claim` dispatchable.
                // </weight>
                Call::claim {
                    dest: account,
                    ethereum_signature,
                } => {
                    let data = account.using_encoded(to_ascii_hex);
                    Self::eth_recover(&ethereum_signature, &data)
                }
                _ => return Err(InvalidTransaction::Call.into()),
            };

            let signer = maybe_signer.ok_or(InvalidTransaction::Custom(0))?;

            let e = InvalidTransaction::Custom(1);
            ensure!(Claims::<T>::contains_key(&signer), e);

            Ok(ValidTransaction {
                priority: PRIORITY,
                requires: vec![],
                provides: vec![("claims", signer).encode()],
                longevity: TransactionLongevity::max_value(),
                propagate: true,
            })
        }
    }
}

/// Converts the given binary data into ASCII-encoded hex. It will be twice the length.
///
/// # Arguments
/// * `data` - The binary data to convert to hex ASCII
///
/// # Returns
/// A vector containing the ASCII hex representation (e.g., [0xAB] becomes [b'a', b'b'])
fn to_ascii_hex(data: &[u8]) -> Vec<u8> {
    let mut r = Vec::with_capacity(data.len() * 2);
    let mut push_nibble = |n| r.push(if n < 10 { b'0' + n } else { b'a' - 10 + n });
    for &b in data.iter() {
        push_nibble(b / 16);
        push_nibble(b % 16);
    }
    r
}

impl<T: Config> Pallet<T> {
    /// Constructs the message that Ethereum RPC's `personal_sign` and `eth_sign` would sign.
    ///
    /// The format matches Ethereum's standard:
    /// `\x19Ethereum Signed Message:\n{length}{prefix}{what}`
    ///
    /// # Arguments
    /// * `what` - The actual message content to sign
    ///
    /// # Returns
    /// The full message that should be hashed and signed
    fn ethereum_signable_message(what: &[u8]) -> Vec<u8> {
        let prefix = T::Prefix::get();
        let mut l = prefix.len() + what.len();
        let mut rev = Vec::new();
        while l > 0 {
            rev.push(b'0' + (l % 10) as u8);
            l /= 10;
        }
        let mut v = b"\x19Ethereum Signed Message:\n".to_vec();
        v.extend(rev.into_iter().rev());
        v.extend_from_slice(prefix);
        v.extend_from_slice(what);
        v
    }

    /// Attempts to recover the Ethereum address from a message signature.
    ///
    /// Uses ECDSA signature recovery with the secp256k1 curve.
    ///
    /// # Arguments
    /// * `s` - The ECDSA signature (65 bytes: r, s, v)
    /// * `what` - The message that was signed
    ///
    /// # Returns
    /// The recovered Ethereum address, or `None` if recovery fails
    fn eth_recover(s: &EcdsaSignature, what: &[u8]) -> Option<EthereumAddress> {
        let msg = keccak_256(&Self::ethereum_signable_message(what));
        let mut res = EthereumAddress::default();
        res.0
            .copy_from_slice(&keccak_256(&secp256k1_ecdsa_recover(&s.0, &msg).ok()?[..])[12..]);
        Some(res)
    }

    /// Processes a claim by transferring tokens and removing the claim from storage.
    ///
    /// # Arguments
    /// * `signer` - The Ethereum address that signed the claim
    /// * `dest` - The destination account to receive the claimed tokens
    ///
    /// # Errors
    /// * `SignerHasNoClaim` - If the Ethereum address has no associated claim
    /// * Transfer errors from the currency pallet
    fn process_claim(signer: EthereumAddress, dest: T::AccountId) -> sp_runtime::DispatchResult {
        let balance_due = Claims::<T>::get(&signer).ok_or(Error::<T>::SignerHasNoClaim)?;

        let pallet_account = T::PalletId::get().into_account_truncating();
        let _ = T::Currency::transfer(
            &pallet_account,
            &dest,
            balance_due,
            ExistenceRequirement::AllowDeath,
        )?;

        Claims::<T>::remove(&signer);

        // Let's deposit an event to let the outside world know this happened.
        Self::deposit_event(Event::<T>::Claimed {
            who: dest,
            ethereum_address: signer,
            amount: balance_due,
        });

        Ok(())
    }
}

#[cfg(any(test, feature = "runtime-benchmarks"))]
mod secp_utils {
    //! Utilities for working with secp256k1 keys and Ethereum addresses in tests and benchmarks.
    use super::*;

    /// Derives the public key from a secret key.
    pub fn public(secret: &libsecp256k1::SecretKey) -> libsecp256k1::PublicKey {
        libsecp256k1::PublicKey::from_secret_key(secret)
    }
    
    /// Derives an Ethereum address from a secret key.
    ///
    /// Computes the Keccak-256 hash of the public key and takes the last 20 bytes.
    pub fn eth(secret: &libsecp256k1::SecretKey) -> EthereumAddress {
        let mut res = EthereumAddress::default();
        res.0
            .copy_from_slice(&keccak_256(&public(secret).serialize()[1..65])[12..]);
        res
    }
    
    /// Signs a message with a secret key in the format expected by the pallet.
    ///
    /// # Arguments
    /// * `secret` - The secret key to sign with
    /// * `what` - The message to sign (will be hex-encoded and wrapped in Ethereum format)
    ///
    /// # Returns
    /// An ECDSA signature in the format expected by the claim pallet
    pub fn sig<T: Config>(secret: &libsecp256k1::SecretKey, what: &[u8]) -> EcdsaSignature {
        let msg = keccak_256(&super::Pallet::<T>::ethereum_signable_message(
            &to_ascii_hex(what)[..],
        ));
        let (sig, recovery_id) = libsecp256k1::sign(&libsecp256k1::Message::parse(&msg), secret);
        let mut r = [0u8; 65];
        r[0..64].copy_from_slice(&sig.serialize()[..]);
        r[64] = recovery_id.serialize();
        EcdsaSignature(r)
    }
}

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
