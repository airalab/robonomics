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
//! Runtime API definition required by Robonomics Agent RPC extensions.
//!
//! This API should be imported and implemented by the runtime,
//! of a node that wants to use the Robonomics Agent RPC.

#![cfg_attr(not(feature = "std"), no_std)]

use pallet_robonomics_provider::RobonomicsMessage;
use pallet_robonomics_liability::{
    TechnicalParam, EconomicalParam, TechnicalReport, LiabilityIndex,
};
use sp_runtime::RuntimeDebug;
use codec::{Encode, Decode};
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    #[api_version(1)]
	pub trait RobonomicsAgentApi<AccountId> where
        AccountId: codec::Codec,
    {
        /// Get Robonomics Agent offchain worker account.
        fn account() -> Option<AccountId>;
	}

    #[api_version(1)]
    pub trait RobonomicsLiabilityApi<Runtime> where
        Runtime: pallet_robonomics_provider::Trait,
    {
        /// Pull execution task for a liability player.
        fn pull() -> Option<Vec<u8>>;

        /// Check inbox for a new Robonomics messages.
        fn recv() -> Vec<RobonomicsMessage<Runtime>>;

        /// Send demand message from agent account.
        fn send_demand(
            technics: TechnicalParam<Runtime>,
            economics: EconomicalParam<Runtime>
        ) -> Result<(), ()>;

        /// Send offer message from agent account.
        fn send_offer(
            technics: TechnicalParam<Runtime>,
            economics: EconomicalParam<Runtime>,
        ) -> Result<(), ()>;

        /// Send report message from agent account.
        fn send_report(
            index: LiabilityIndex,
            report: TechnicalReport<Runtime>,
        ) -> Result<(), ()>;
    }

    #[api_version(1)]
    pub trait RobonomicsBlockchainApi {
        /// Send some data to store it in blockchain.
        fn send_data(data: Vec<u8>) -> Result<(), ()>;
    }
}
