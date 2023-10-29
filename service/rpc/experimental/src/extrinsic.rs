///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2023 Robonomics Network <research@robonomics.network>
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
//! Robonomics Network extrinsic API.

use codec::{Decode, Encode, HasCompact};
use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
};
use robonomics_primitives::{AccountId, Block, Index};
use sp_api::{BlockId, Core, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use std::{str::FromStr, sync::Arc};
use substrate_frame_rpc_system::AccountNonceApi;

#[derive(Debug, PartialEq, Encode, Decode)]
struct AsCompact<T: HasCompact>(#[codec(compact)] pub T);

#[derive(Clone)]
pub struct ExtrinsicRpc<C> {
    client: Arc<C>,
}

impl<C> ExtrinsicRpc<C> {
    pub fn new(client: Arc<C>) -> Self
    where
        C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Sync + Send + 'static,
        C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    {
        Self { client }
    }
}

#[rpc(server)]
pub trait ExtrinsicRpc {
    #[method(name = "get_payload")]
    fn get_payload(&self, address: String) -> RpcResult<Vec<String>>;
}

#[async_trait]
impl<C> ExtrinsicRpcServer for ExtrinsicRpc<C>
where
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Sync + Send + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
{
    fn get_payload(&self, address: String) -> RpcResult<Vec<String>> {
        // Address: The address of the sending account.
        let address = AccountId::from_str(&address)
            .map_err(|_| JsonRpseeError::Custom("Unknown address!".to_string()))?;

        // Nonce: The nonce for this transaction.
        let nonce = self
            .client
            .runtime_api()
            .account_nonce(&BlockId::Hash(self.client.info().best_hash), address)
            .map_err(|_| JsonRpseeError::Custom("Internal error!".to_string()))?;

        // Spec Version: The current spec version for the runtime.
        let version = self
            .client
            .runtime_api()
            .version(&BlockId::Hash(self.client.info().best_hash))
            .map_err(|_| JsonRpseeError::Custom("Internal error!".to_string()))?;
        let spec_version = version.spec_version;

        // Tip: Optional, the tip to increase transaction priority.
        let tip = 0 as u64;

        // Era Period: Optional, the number of blocks after the checkpoint
        // for which a transaction is valid. If zero, the transaction is immortal.
        let era = 0 as u64;

        // Transaction Version: The current version for transaction format.
        let tx_version = 1 as u64;

        Ok(vec![
            format!("0x{}", &hex::encode(AsCompact(nonce as u64).encode())),
            format!("0x{}", &hex::encode(spec_version.encode())),
            format!("0x{}", &hex::encode(AsCompact(tip).encode())),
            format!("0x{}", &hex::encode(era.encode())),
            format!("0x{}", &hex::encode(tx_version.encode())),
        ])
    }
}
