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

use log::info;
use std::sync::Arc;
use sp_runtime::traits;
use futures::future::Future;
use sp_api::ProvideRuntimeApi;
use sc_client::BlockchainEvents;
use node_primitives::{AccountId, Index};
use sp_core::traits::BareCryptoStorePtr;
use sp_transaction_pool::TransactionPool;
use sp_core::crypto::{Pair, Ss58Codec};
use frame_system_rpc_runtime_api::AccountNonceApi;
use pallet_robonomics_runtime_api::SystemEventsApi;
use node_runtime::Event;

pub mod error;
mod events;
mod crypto;
mod services;

use error::Result;
use crypto::RobotCrypto;
use services::{
    send_demand,
    send_offer,
    send_report,
};

/// ROS API main routine.
pub fn start<C, P>(
    client: Arc<C>,
    pool: Arc<P>,
	keystore: BareCryptoStorePtr,
) -> Result<(impl Future<Output=()>, Vec<rosrust::Service>)> where
    C: ProvideRuntimeApi<P::Block> + BlockchainEvents<P::Block> + 'static,
    P: TransactionPool,
    C::Api: AccountNonceApi<P::Block, AccountId, Index> + SystemEventsApi<P::Block, Event>,
    P::Block: traits::Block,
{
    let robot = RobotCrypto::new(client.clone(), pool, keystore)?;
    info!("Robot account loaded: {}", robot.key.public().to_ss58check());

    let task = events::finalized_event_stream(client)
        .map_err(error::Error::RosError)?;
    let services = vec![
        send_demand(robot.clone())?,
        send_offer(robot.clone())?,
        send_report(robot.clone())?,
    ];

    Ok((task, services))
}
