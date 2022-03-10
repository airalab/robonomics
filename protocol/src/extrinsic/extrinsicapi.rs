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
//! Robonomics Network protocol.

use jsonrpc_core::{Error, Result};
use jsonrpc_derive::rpc;
use robonomics_primitives::{AccountId, Block, Index};
use sp_api::{BlockId, Core, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_runtime::generic::Era;
use std::{str::FromStr, sync::Arc};
use substrate_frame_rpc_system::AccountNonceApi;

type Nonce = Index;
type SpecVersion = u32;
type Tip = u32;
type TxVersion = u32;

#[rpc]
pub trait ExtrinsicT {
    #[rpc(name = "get_payload")]
    fn get_payload(&self, address: String) -> Result<(Nonce, SpecVersion, Tip, Era, TxVersion)>;
}

pub struct ExtrinsicApi<C> {
    client: Arc<C>,
}

impl<C> ExtrinsicApi<C> {
    pub fn new(client: Arc<C>) -> ExtrinsicApi<C>
    where
        C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Sync + Send + 'static,
        C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    {
        ExtrinsicApi { client }
    }
}

impl<C> ExtrinsicT for ExtrinsicApi<C>
where
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Sync + Send + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
{
    fn get_payload(&self, address: String) -> Result<(Nonce, SpecVersion, Tip, Era, TxVersion)> {
        // Address: The address of the sending account.
        let address =
            AccountId::from_str(&address).map_err(|_| Error::invalid_params("Invalid account"))?;

        // Nonce: The nonce for this transaction.
        let nonce = self
            .client
            .runtime_api()
            .account_nonce(&BlockId::Hash(self.client.info().best_hash), address)
            .map_err(|_| Error::internal_error())?;

        // Spec Version: The current spec version for the runtime.
        let version = self
            .client
            .runtime_api()
            .version(&BlockId::Hash(self.client.info().best_hash))
            .map_err(|_| Error::internal_error())?;
        let spec_version = version.spec_version;

        // Tip: Optional, the tip to increase transaction priority.
        let tip = 0;

        // Era Period: Optional, the number of blocks after the checkpoint
        // for which a transaction is valid. If zero, the transaction is immortal.
        let era = Era::immortal();

        // Transaction Version: The current version for transaction format.
        let tx_version = 1;

        Ok((nonce, spec_version, tip, era, tx_version))
    }
}
