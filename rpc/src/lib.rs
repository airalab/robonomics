///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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
//! A collection of node-specific RPC methods.

use std::sync::Arc;

use robonomics_primitives::{AccountId, Balance, Block, Index};
use robonomics_protocol::pubsub::PubSub;

use jsonrpsee::RpcModule;
use sc_client_api::AuxStore;
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};

// TODO: fix rpc servers
pub mod extrinsic;
pub mod pubsub;
//pub mod reqres;

use extrinsic::{ExtrinsicRpc, ExtrinsicRpcServer};
use pubsub::{PubSubRpc, PubSubRpcServer};
//use reqres::{ReqRespRpc, ReqRespRpcServer};

/// Full client dependencies.
pub struct FullDeps<C, P, T> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls.
    pub deny_unsafe: DenyUnsafe,
    // PubSub worker.
    pub pubsub: Arc<T>,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, T>(
    deps: FullDeps<C, P, T>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + AuxStore
        + HeaderMetadata<Block, Error = BlockChainError>
        + Sync
        + Send
        + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>
        + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + BlockBuilder<Block>,
    P: TransactionPool + Sync + Send + 'static,
    T: PubSub + Sync + Send + 'static,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};

    let mut io = RpcModule::new(());
    let FullDeps {
        client,
        pool,
        deny_unsafe,
        pubsub,
    } = deps;

    io.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
    io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
    io.merge(PubSubRpc::new(pubsub).into_rpc())?;
    io.merge(ExtrinsicRpc::new(client.clone()).into_rpc())?;
    //io.merge(ReqRespRpc::new().into_rpc())?;

    Ok(io)
}
