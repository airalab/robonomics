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

use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use sp_core::crypto::Ss58Codec;
use std::sync::Arc;
use substrate_frame_rpc_system::AccountNonceApi;

use crate::error::Error;
use robonomics_primitives::{AccountId, Balance, Block, BlockNumber, Index};
use sc_client_api::AuxStore;
use sc_transaction_pool_api::TransactionPool;
use sp_api::BlockId;
use sp_api::Core;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};

type BlockHash = String;
type GenesisHash = String;
type Metadata = String;
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
    ) -> Result<(
        AccountId,
        BlockHash,
        BlockNumber,
        GenesisHash,
        Metadata,
        Nonce,
        SpecVersion,
        Tip,
        Era,
        TxVersion,
    )>;
}

pub struct ExtrinsicApi<C> {
    client: Arc<C>,
}

impl<C> ExtrinsicApi<C> {
    pub fn new(client: Arc<C>) -> ExtrinsicApi<C>
    where
        // C: std::marker::Send + std::marker::Sync,
        C: ProvideRuntimeApi<Block>
            + HeaderBackend<Block>
            + AuxStore
            + HeaderMetadata<Block, Error = BlockChainError>
            + Sync
            + Send
            + 'static,
        C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
        // C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
        C::Api: BlockBuilder<Block>,
        // P: TransactionPool + 'static,
    {
        ExtrinsicApi { client }
    }
}

impl<C> ExtrinsicT for ExtrinsicApi<C>
where
    // C: 'static + std::marker::Send + std::marker::Sync,
    C: ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + AuxStore
        + HeaderMetadata<Block, Error = BlockChainError>
        + Sync
        + Send
        + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    // C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: BlockBuilder<Block>,
    // P: TransactionPool + 'static,
{
    fn get_payload(
        &self,
        address: String,
    ) -> Result<(
        AccountId,
        BlockHash,
        BlockNumber,
        GenesisHash,
        Metadata,
        Nonce,
        SpecVersion,
        Tip,
        Era,
        TxVersion,
    )> {
        // Address: The SS58-encoded address of the sending account.
        let address = AccountId::from_ss58check(address.as_str())
            .map_err(|_| Error::Ss58CodecError)
            .unwrap();
        println!("address: {:?}", address);

        // Block Hash: The hash of the checkpoint block.
        let block_hash = self.client.info().best_hash;
        println!("block_hash: {:?}", block_hash);

        // Block Number: The number of the checkpoint block.
        let block_number = self.client.info().best_number;
        println!("block_number: {:?}", block_number);

        // Genesis Hash: The genesis hash of the chain.
        let genesis_hash = self.client.info().genesis_hash;
        println!("genesis_hash: {:?}", genesis_hash);

        // ------------------------------------------------
        // Metadata: The SCALE-encoded metadata for the runtime when submitted.
        let metadata = "metadata".to_string();
        println!("metadata: {:?}", metadata);
        // ------------------------------------------------

        // Nonce: The nonce for this transaction.
        let nonce = self
            .client
            .runtime_api()
            .account_nonce(&BlockId::Hash(block_hash), address.clone())
            .expect("Fetching account nonce works");
        println!("nonce: {:?}", nonce);

        // Spec Version: The current spec version for the runtime.
        let version = self
            .client
            .runtime_api()
            .version(&BlockId::Hash(block_hash))
            .expect("There should be runtime version at 0");
        let spec_version = version.spec_version;
        println!("spec_version: {:?}", spec_version);

        // Tip: Optional, the tip to increase transaction priority.
        let tip = 0;
        println!("tip: {:?}", tip);

        // Era Period: Optional, the number of blocks after the checkpoint
        // for which a transaction is valid. If zero, the transaction is immortal.
        let era = 0;
        println!("era: {:?}", era);

        // Transaction Version: The current version for transaction format.
        let tx_version = 1;
        println!("tx_version: {:?}", tx_version);

        Ok((
            address,
            block_hash.to_string(),
            block_number,
            genesis_hash.to_string(),
            metadata,
            nonce,
            spec_version,
            tip,
            era,
            tx_version,
        ))
    }
}
