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

use crate::error::Error;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use robonomics_primitives::{AccountId, Block, Index};
use sp_api::{BlockId, Core, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_core::crypto::Ss58Codec;
use std::sync::Arc;
use substrate_frame_rpc_system::AccountNonceApi;

type GenesisHash = String;
type Nonce = u32;
type SpecVersion = u32;
type Tip = u32;
type Era = u32;
type TxVersion = u32;

#[rpc]
pub trait ExtrinsicT {
    #[rpc(name = "get_payload")]
    fn get_payload(
        &self,
        account_id: String,
    ) -> Result<(GenesisHash, Nonce, SpecVersion, Tip, Era, TxVersion)>;
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
    fn get_payload(
        &self,
        address: String,
    ) -> Result<(GenesisHash, Nonce, SpecVersion, Tip, Era, TxVersion)> {
        // Address: The SS58-encoded address of the sending account.
        let address = AccountId::from_ss58check(address.as_str())
            .map_err(|_| Error::Ss58CodecError)
            .unwrap();

        // Genesis Hash: The genesis hash of the chain.
        let genesis_hash = self.client.info().genesis_hash.to_string();

        // Nonce: The nonce for this transaction.
        let nonce = self
            .client
            .runtime_api()
            .account_nonce(&BlockId::Hash(self.client.info().best_hash), address)
            .expect("Fetching account nonce works");

        // Spec Version: The current spec version for the runtime.
        let version = self
            .client
            .runtime_api()
            .version(&BlockId::Hash(self.client.info().best_hash))
            .expect("There should be runtime version at 0");
        let spec_version = version.spec_version;

        // Tip: Optional, the tip to increase transaction priority.
        let tip = 0;

        // Era Period: Optional, the number of blocks after the checkpoint
        // for which a transaction is valid. If zero, the transaction is immortal.
        let era = 0;

        // Transaction Version: The current version for transaction format.
        let tx_version = 1;

        Ok((genesis_hash, nonce, spec_version, tip, era, tx_version))
    }
}
