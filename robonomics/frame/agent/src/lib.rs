///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life> 
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
//! Robonomics Network Agent module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use sp_std::{
    prelude::*,
    fmt::Debug,
};
use sp_core::offchain::StorageKind;
use sp_runtime::{
    RuntimeDebug,
    app_crypto::{KeyTypeId, AppKey, AppPublic, AppSignature, RuntimeAppPublic},
    traits::{
        Member, CheckEqual, MaybeSerializeDeserialize, Hash,
        MaybeDisplay, SimpleBitOps, IdentifyAccount,
    },
};
use frame_support::{
    decl_module, decl_event, decl_storage, decl_error,
    debug, StorageValue, weights::SimpleDispatchInfo, dispatch::Parameter,
};
use frame_system::{
    self as system, ensure_signed,
    offchain::{SubmitSignedTransaction, SubmitUnsignedTransaction, Signer}
};
use pallet_robonomics_provider::RobonomicsMessage;
use pallet_robonomics_liability::{
    TechnicalParam, EconomicalParam, ProofParam, AccountId, LiabilityIndex,
    TechnicalReport, traits::ProofBuilder, signed::AppProofSigner,
};

/// Agent crypto primitives.
pub mod crypto;

/// Our local KeyType.
///
/// For security reasons the offchain worker doesn't have direct access to the keys
/// but only to app-specific subkeys, which are defined and grouped by their `KeyTypeId`.
/// We define it here as `xrta` (XRT Agent).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"xrta");

/// The local storage database key under which the worker progress status
/// is tracked.
const DB_KEY: &[u8] = b"airalab/robonomics-agent-worker";

/// The module's main configuration trait.
pub trait Trait: system::Trait {
    /// Runtime application crypto type.
    type AgentKey: AppPublic + RuntimeAppPublic + From<<Self::AgentKey as AppPublic>::Generic>;

    /// The regular events type.
    type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

#[derive(Encode, Decode, Clone, RuntimeDebug)]
pub struct WorkerState<T: Trait> {
    last_update: T::BlockNumber,
}

impl<T: Trait> Default for WorkerState<T> {
    fn default() -> WorkerState<T> {
        WorkerState {
            last_update: T::BlockNumber::default(),
        } 
    }
}

decl_event!(
    pub enum Event {
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Agent {
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Initializing events
        fn deposit_event() = default;

        /// Clean the state on initialisation of a block
        fn on_initialize(_now: T::BlockNumber) {
            // At the beginning of each block execution, system triggers all
            // `on_initialize` functions, which allows us to set up some temporary state or - like
            // in this case - clean up other states
            //
            //<OcRequests<T>>::kill();
        }

        // Runs after every block within the context and current state of said block.
        fn offchain_worker(_now: T::BlockNumber) {
            debug::RuntimeLogger::init();
            Self::offchain();
        }
    }
}

impl<T: Trait> Module<T> {
    /// The main entry point.
    fn offchain() {
    }

    /// Get default worker generic account.
    pub fn account() -> <T::AgentKey as AppPublic>::Generic {
        Self::key().into()
    }

    /// Get default worker signer key.
    pub fn key() -> T::AgentKey {
        let keys = <T::AgentKey as RuntimeAppPublic>::all();
        if keys.is_empty() {
            <T::AgentKey as RuntimeAppPublic>::generate_pair(None)
        } else {
            keys[0].clone()
        }.into()
    }
}

impl<T: Trait + pallet_robonomics_storage::Agent> Module<T> {
    /// Send data record using agent interface.
    pub fn send_record(record: T::Record) -> Result<(), ()> {
        let call = pallet_robonomics_storage::Call::<T>::record(record);
        T::SubmitTransaction::submit_signed(call)[0].1
    }
}

impl<T: Trait + pallet_robonomics_provider::Agent> Module<T> where
    <T::AgentKey as RuntimeAppPublic>::Signature: AppSignature +
        From<<<T::AgentKey as RuntimeAppPublic>::Signature as AppSignature>::Generic>,
    <<T::AgentKey as RuntimeAppPublic>::Signature as AppSignature>::Generic: Into<ProofParam<T>>,
{
    /// Get execution task for liability player.
    pub fn pull() -> Option<Vec<u8>> {
        // TODO
        None
    }

    /// Get list of incoming robonomics messages.
    pub fn recv() -> Vec<RobonomicsMessage<T>> {
        // TODO
        vec![]
    }

    /// Send demand request to robonomics provider.
    pub fn send_demand(
        technics: TechnicalParam<T>,
        economics: EconomicalParam<T>,
    ) -> Result<(), ()> {
        let proof: <<T::AgentKey as RuntimeAppPublic>::Signature as AppSignature>::Generic
            = Self::sign_params(&technics, &economics).into();
        let call = pallet_robonomics_provider::Call::<T>::demand(
            technics,
            economics,
            proof.into(),
        );
        <T as pallet_robonomics_provider::Agent>::SubmitTransaction::submit_signed(call)[0].1
    }

    /// Send demand request to robonomics provider.
    pub fn send_offer(
        technics: TechnicalParam<T>,
        economics: EconomicalParam<T>,
    ) -> Result<(), ()> {
        let proof: <<T::AgentKey as RuntimeAppPublic>::Signature as AppSignature>::Generic
            = Self::sign_params(&technics, &economics).into();
        let call = pallet_robonomics_provider::Call::<T>::offer(
            technics,
            economics,
            proof.into(),
        );
        <T as pallet_robonomics_provider::Agent>::SubmitTransaction::submit_signed(call)[0].1
    }

    /// Send report of liability.
    pub fn send_report(
        index: LiabilityIndex<T>,
        report: TechnicalReport<T>,
    ) -> Result<(), ()> {
        let proof: <<T::AgentKey as RuntimeAppPublic>::Signature as AppSignature>::Generic
            = Self::sign_report(&index, &report).into();
        let call = pallet_robonomics_liability::Call::<T>::finalize(
            index,
            report,
            proof.into(),
        );
        <T as pallet_robonomics_provider::Trait>::SubmitTransaction::submit_unsigned(call)
    }

    fn sign_params(
        technics: &TechnicalParam<T>,
        economics: &EconomicalParam<T>,
    ) -> <T::AgentKey as RuntimeAppPublic>::Signature {
        <AppProofSigner<T::AgentKey> as ProofBuilder<
            <T as pallet_robonomics_liability::Trait>::Technics,
            <T as pallet_robonomics_liability::Trait>::Economics,
            LiabilityIndex<T>,
            _,
            _,
            >>::proof_params(technics, economics, Self::key())
    }

    fn sign_report(
        index: &LiabilityIndex<T>,
        report: &TechnicalReport<T>,
    ) -> <T::AgentKey as RuntimeAppPublic>::Signature {
        <AppProofSigner<T::AgentKey> as ProofBuilder<
            <T as pallet_robonomics_liability::Trait>::Technics,
            <T as pallet_robonomics_liability::Trait>::Economics,
            LiabilityIndex<T>,
            _,
            _,
            >>::proof_report(index, report, Self::key())
    }
}
