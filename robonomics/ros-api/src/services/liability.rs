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
//! Robonomics ROS liability services implementation. 

use std::sync::Arc;
use base58::FromBase58;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{traits, generic::BlockId};
use pallet_robonomics_liability::traits::Technical;
use pallet_robonomics_liability::traits::Economical;
use pallet_robonomics_liability as liability;
use pallet_robonomics_agent_runtime_api::RobonomicsLiabilityApi;

use msgs::robonomics_msgs::{
    SendOrder, SendOrderRes,
    SendReport, SendReportRes,
};

#[cfg(not(feature = "liability"))]
pub fn send_demand<C>(_client: C) -> Result<(), String> { Ok(()) }

/// Send liability demand using agent runtime api.
#[cfg(feature = "liability")]
pub fn send_demand<B: traits::Block, C, T>(
    client: Arc<C>,
) -> Result<rosrust::Service, String> where
    C: ProvideRuntimeApi<B> + HeaderBackend<B> + 'static,
    C::Api: RobonomicsLiabilityApi<B, T>,
    T: liability::Trait,
    <<T as liability::Trait>::Technics as Technical>::Parameter: AsRef<[u8]> + From<Vec<u8>>,
    <<T as liability::Trait>::Economics as Economical>::Parameter: Default, 
{
    rosrust::service::<SendOrder, _>("liability/demand", move |req| {
        let technics = req.technics
            .from_base58()
            .map_err(|e| format!("Base58 decode error: {:?}", e))?;
        let block_id = BlockId::Hash(client.info().best_hash);
        client
            .runtime_api()
            .send_demand(&block_id, technics.into(), Default::default())
            .expect("Runtime communication error")
            .map_err(|_| "Error in Robonomics Runtime Agent")?;
        let mut res = SendOrderRes::default(); 
        Ok(res)
    }).map_err(|e| format!("ROS service error: {}", e))
}

#[cfg(not(feature = "liability"))]
pub fn send_offer<C>(_client: C) -> Result<(), String> { Ok(()) }

/// Send liability offer using agent runtime api.
#[cfg(feature = "liability")]
pub fn send_offer<B: traits::Block, C, T>(
    client: Arc<C>,
) -> Result<rosrust::Service, String> where
    C: ProvideRuntimeApi<B> + HeaderBackend<B> + 'static,
    C::Api: RobonomicsLiabilityApi<B, T>,
    T: liability::Trait,
    <<T as liability::Trait>::Technics as Technical>::Parameter: AsRef<[u8]> + From<Vec<u8>>,
    <<T as liability::Trait>::Economics as Economical>::Parameter: Default, 
{
    rosrust::service::<SendOrder, _>("liability/offer", move |req| {
        let technics = req.technics
            .from_base58()
            .map_err(|e| format!("Base58 decode error: {:?}", e))?;
        let block_id = BlockId::Hash(client.info().best_hash);
        client
            .runtime_api()
            .send_offer(&block_id, technics.into(), Default::default())
            .expect("Runtime communication error")
            .map_err(|_| "Error in Robonomics Runtime Agent")?;
        let mut res = SendOrderRes::default(); 
        Ok(res)
    }).map_err(|e| format!("ROS service error: {}", e))
}

#[cfg(not(feature = "liability"))]
pub fn send_report<C>(_client: C) -> Result<(), String> { Ok(()) }

/// Send liability report using agent runtime api.
#[cfg(feature = "liability")]
pub fn send_report<B: traits::Block, C, T>(
    client: Arc<C>,
) -> Result<rosrust::Service, String> where
    C: ProvideRuntimeApi<B> + HeaderBackend<B> + 'static,
    C::Api: RobonomicsLiabilityApi<B, T>,
    T: liability::Trait,
{
    rosrust::service::<SendReport, _>("liability/report", move |req| {
        let mut res = SendReportRes::default(); 
        // TODO
        Ok(res)
    }).map_err(|e| format!("ROS service error: {}", e))
}
