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
//! Robonomics ROS services implementation. 

use std::sync::Arc;
use base58::FromBase58;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{traits, generic::BlockId};
use pallet_robonomics_storage as storage;
use pallet_robonomics_agent_runtime_api::RobonomicsBlockchainApi;

use msgs::robonomics_msgs::{
    SendRecord, SendRecordRes,
};

#[cfg(not(feature = "blockchain"))]
pub fn send_record<C>(_client: C) -> Result<(), String> { Ok(()) }

/// Send data record into account blockchain storage using agent runtime api.
#[cfg(feature = "blockchain")]
pub fn send_record<B: traits::Block, C, T>(
    client: Arc<C>,
) -> Result<rosrust::Service, String> where
    C: ProvideRuntimeApi<B> + HeaderBackend<B> + 'static,
    C::Api: RobonomicsBlockchainApi<B, T>,
    <T as storage::Trait>::Record: From<Vec<u8>>,
    T: storage::Trait,
{
    rosrust::service::<SendRecord, _>("blockchain/record", move |req| {
        let record = req.record
            .from_base58()
            .map_err(|e| format!("Base58 decode error: {:?}", e))?;
        let block_id = BlockId::Hash(client.info().best_hash);
        client
            .runtime_api()
            .send_record(&block_id,  record.into())
            .expect("Runtime communication error")
            .map_err(|_| "Error in Robonomics Runtime Agent")?;
        let mut res = SendRecordRes::default(); 
        Ok(res)
    }).map_err(|e| format!("ROS service error: {}", e))
}
