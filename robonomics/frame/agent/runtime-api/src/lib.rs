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
//! Runtime API definition required by Robonomics Agent RPC extensions.
//!
//! This API should be imported and implemented by the runtime,
//! of a node that wants to use the Robonomics Agent RPC.

#![cfg_attr(not(feature = "std"), no_std)]

use pallet_robonomics_liability::{
    TechnicalParam, EconomicalParam, TechnicalReport, LiabilityIndex,
};
use pallet_robonomics_provider::RobonomicsMessage;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait RobonomicsAgentApi<T> where
        T: frame_system::Trait
    {
        /// Get Robonomics Agent offchain worker account.
        fn account() -> T::AccountId;
    }

    #[api_version(1)]
    pub trait RobonomicsLiabilityApi<T> where
        T: pallet_robonomics_liability::Trait
    {
        /// Check inbox for a new Robonomics messages.
        fn recv() -> Vec<RobonomicsMessage<T>>;

        /// Send demand message from agent account.
        fn send_demand(
            technics: TechnicalParam<T>,
            economics: EconomicalParam<T>
        ) -> Result<(), ()>;

        /// Send offer message from agent account.
        fn send_offer(
            technics: TechnicalParam<T>,
            economics: EconomicalParam<T>,
        ) -> Result<(), ()>;

        /// Send report message from agent account.
        fn send_report(
            index: LiabilityIndex<T>,
            report: TechnicalReport<T>,
        ) -> Result<(), ()>;

        /// Pull execution task for a liability player.
        fn pull() -> Option<Vec<u8>>;
    }

    #[api_version(1)]
    pub trait RobonomicsBlockchainApi<T> where
        T: pallet_robonomics_storage::Trait
    {
        /// Send data record to robonomics blockchain storage.
        fn send_record(record: T::Record) -> Result<(), ()>;
    }
}
