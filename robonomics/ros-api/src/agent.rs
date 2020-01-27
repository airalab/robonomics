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
//! This module exports Robonomics API into ROS namespace.


use std::sync::Arc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{traits, generic::BlockId};
use pallet_robonomics_agent_runtime_api::RobonomicsAgentApi;

#[cfg(not(feature = "agent"))]
pub fn print_account<C>(_client: C) {}

#[cfg(feature = "agent")]
pub fn print_account<B: traits::Block, C, T>(client: Arc<C>) where
    C: ProvideRuntimeApi<B> + HeaderBackend<B>,
    C::Api: RobonomicsAgentApi<B, T>,
    T: frame_system::Trait,
{
    let best_hash = client.info().best_hash;
    let block_id = BlockId::Hash(best_hash);
    let account = client
            .runtime_api()
            .account(&block_id)
            .expect("Runtime communication error");
    log::info!("Robonomics Agent: {}", account);
}
