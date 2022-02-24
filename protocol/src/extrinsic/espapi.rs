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
use sp_core::{sr25519, Pair};

use local_runtime::{
    BlockHashCount, CheckedExtrinsic, Runtime, SessionKeys, SignedExtra, System, UncheckedExtrinsic,
};
use robonomics_primitives::{AccountId, Balance, Index};
use sp_runtime::generic::Era;
use sp_runtime::traits::Zero;

#[rpc]
pub trait ESPApi {
    #[rpc(name = "get_payload")]
    fn get_payload(&self, account_id: String) -> Result<(String, String)>;
}

pub struct ESP;

impl ESPApi for ESP {
    /// ESP asks for transaction params
    fn get_payload(&self, account: String) -> Result<(String, String)> {
        let pair = sr25519::Pair::from_string(account.as_str(), None).unwrap();
        println!("account: {}", account);

        // Подготовка метаданных (время жизни, nonce, хеш генезис-блока и тп)
        // -----------------------------------------------------------------------
        // Address: The SS58-encoded address of the sending account.
        // Block Hash: The hash of the checkpoint block.
        // Block Number: The number of the checkpoint block.
        // Genesis Hash: The genesis hash of the chain.
        // Metadata: The SCALE-encoded metadata for the runtime when submitted.
        // Nonce: The nonce for this transaction.*
        // Spec Version: The current spec version for the runtime.
        // Transaction Version: The current version for transaction format.
        // Tip: Optional, the tip to increase transaction priority.
        // Era Period: Optional, the number of blocks after the checkpoint for which a transaction is valid. If zero, the transaction is immortal.
        // -----------------------------------------------------------------------

        use futures::{executor, StreamExt};
        use robonomics_primitives::BlockNumber;
        // let h1 = frame_system::Module::<T>::block_hash(T::BlockNumber::zero());
        // let a = frame_system::runtime_version();
        // let a = System::block_number();

        executor::block_on(async {
            let a = System::block_number();
            println!("a: {:?}", a);
        });

        // let h2 = <system::Module<T>>::block_hash(T::BlockNumber::from(0));
        // println!("a: {:?}", a);

        // let period = BlockHashCount::get() as u64;
        // let current_block = System::block_number()
        //     .saturated_into::<u64>()
        //     .saturating_sub(1);
        // let genesis_hash = client
        //     .block_hash(Zero::zero())
        //     .expect("Database error?")
        //     .expect("Genesis block always exists; qed")
        //     .into();
        //         frame_system::CheckTxVersion::new(),
        //         frame_system::CheckGenesis::new(),
        //         frame_system::CheckEra::from(Era::mortal(256, 0)),
        //         frame_system::CheckNonce::from(nonce),
        //         frame_system::CheckWeight::new(),

        // println!("genesis: {:?}", genesis);

        let payload = "payload".to_string();
        let extrinsic = "extrinsic".to_string();

        // payload - данные которые нужно подписать
        // extrinsic - данные к которым нужно добавить подпись перед отправкой
        //
        // в итоге только данные нужны
        Ok((payload, extrinsic))
    }
}
