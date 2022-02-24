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

use crate::error::Error;
use local_runtime::{Runtime, System};
use robonomics_primitives::{AccountId, Index};

#[rpc]
pub trait ESPApi {
    #[rpc(name = "get_payload")]
    fn get_payload(
        &self,
        account_id: String,
    ) -> Result<(
        AccountId, // address
        String,    // block_hash
        u32,       // block_number
        String,    // genesis_hash
        String,    // metadata
        u32,       // nonce
        u32,       // spec_version
        u32,       // tip
        u32,       // era
        u32,       // tx_version
    )>;
}

pub struct ESP;

impl ESPApi for ESP {
    /// ESP asks for transaction params
    fn get_payload(
        &self,
        address: String,
    ) -> Result<(
        AccountId,
        String,
        u32,
        String,
        String,
        u32,
        u32,
        u32,
        u32,
        u32,
    )> {
        // Address: The SS58-encoded address of the sending account.
        let address = AccountId::from_ss58check(address.as_str())
            .map_err(|_| Error::Ss58CodecError)
            .unwrap();
        println!("address: {:?}", address);

        // Block Hash: The hash of the checkpoint block.
        // let block_hash = System::block_hash();
        let block_hash = "block_hash".to_string();
        println!("block_hash: {:?}", block_hash);

        // Block Number: The number of the checkpoint block.
        // let block_number = System::block_number();
        // use sp_arithmetic::traits::SaturatedConversion;
        // let current_block = System::block_number()
        //     .saturated_into::<u64>()
        //     .saturating_sub(1);
        // println!("current_block: {:?}", current_block);
        let block_number = 42;
        println!("block_number: {:?}", block_number);

        // Genesis Hash: The genesis hash of the chain.
        let genesis_hash = "genesis_hash".to_string();
        println!("genesis_hash: {:?}", genesis_hash);

        // Metadata: The SCALE-encoded metadata for the runtime when submitted.
        let metadata = "metadata".to_string();
        println!("metadata: {:?}", metadata);

        // Nonce: The nonce for this transaction.
        // let nonce = System::account_nonce(address);
        // let nonce = frame_system::::account_nonce(&address);
        let nonce = 0;
        println!("nonce: {:?}", nonce);

        // Spec Version: The current spec version for the runtime.
        let spec_version = System::runtime_version().spec_version;
        println!("spec_version: {:?}", spec_version);

        // Tip: Optional, the tip to increase transaction priority.
        let tip = 0;
        println!("tip: {:?}", tip);

        // Era Period: Optional, the number of blocks after the checkpoint for which a transaction is valid. If zero, the transaction is immortal.
        let era = 0;
        println!("era: {:?}", era);

        // Transaction Version: The current version for transaction format.
        let tx_version = 1;
        println!("tx_version: {:?}", tx_version);

        Ok((
            address,
            block_hash,
            block_number,
            genesis_hash,
            metadata,
            nonce,
            spec_version,
            tip,
            era,
            tx_version,
        ))
    }
}
