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
//! Robot cryptographic actions support module.

use std::sync::Arc;
use sp_runtime::traits;
use sp_api::ProvideRuntimeApi;
use node_primitives::{AccountId, Index};
use frame_system_rpc_runtime_api::AccountNonceApi;
use sp_transaction_pool::{TransactionPool, TxHash, error::Error as PoolError};
use sp_core::{
    sr25519,
    crypto::{KeyTypeId, Pair},
    traits::BareCryptoStorePtr,
};

use node_runtime::Call;
use crate::error::Error;

/// ROS cryptographic key type for using in CPS.
///
/// For security reasons worker doesn't have direct access to the keys but only
/// to app-specific subkeys, which are defined and grouped by their `KeyTypeId`.
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"cpsc");

/// Support for cryptographic actions for CPS.
pub struct RobotCrypto<C, P> {
    pub client: Arc<C>,
    pub pool: Arc<P>,
    pub key: sr25519::Pair,
}

impl<C, P> Clone for RobotCrypto<C, P> {
    fn clone(&self) -> Self {
        RobotCrypto {
            client: self.client.clone(),
            pool: self.pool.clone(),
            key: self.key.clone(),
        }
    }
}

impl<C, P> RobotCrypto<C, P> {
    pub fn new(
        client: Arc<C>,
        pool: Arc<P>,
        keystore_ptr: BareCryptoStorePtr
    ) -> Result<Self, Error> {
        let mut keystore = keystore_ptr.write();

        let keys = keystore.sr25519_public_keys(KEY_TYPE);
        let pubkey = if keys.len() == 0 {
            keystore.sr25519_generate_new(KEY_TYPE, None).map_err(Error::KeystoreError)?
        } else {
            keys[0]
        };

        if let Some(key) = keystore.sr25519_key_pair(KEY_TYPE, &pubkey) {
            Ok(RobotCrypto { client, pool, key: key.into() })
        } else {
            Err(Error::KeystoreError("Key load error".to_string()))
        }
    }
}

pub trait ExtrinsicSender {
    type Pool: TransactionPool;

    fn submit(&self, call: Call) -> Result<TxHash<Self::Pool>, PoolError>;
}

impl<C, P> ExtrinsicSender for RobotCrypto<C, P> where
    C: ProvideRuntimeApi<P::Block>,
    P: TransactionPool,
    P::Block: traits::Block,
    C::Api: AccountNonceApi<P::Block, AccountId, Index>,
{
    type Pool = P;

    fn submit(&self, call: Call) -> Result<TxHash<Self::Pool>, PoolError> {
        unimplemented!("send")
        /*
        let api = client.runtime_api();
        let block_id = BlockId::hash(client.chain_info().best_hash);
        // TODO: also check transaction pool for pending txs
        let nonce = api.account_nonce(&block_id, key.public())?; 
        let check_version = system::CheckVersion::new();
        let check_genesis = system::CheckGenesis::new();
        let check_era = system::CheckEra::from(Era::Immortal);
        let check_nonce = system::CheckNonce::from(nonce);
        let check_weight = system::CheckWeight::new();
        let take_fees = balances::TakeFees::from(0);

        let extra = (check_version, check_genesis, check_era, check_nonce, check_weight, take_fees); 
        let raw_payload = (Call::Robonomics(call), extra.clone(), client.info().genesis_hash);

        let signature = raw_payload.using_encoded(|payload| if payload.len() > 256 {
            key.sign(&blake2_256(payload)[..])
        } else {
            key.sign(payload)
        });

        let extrinsic = UncheckedExtrinsic::new_signed(
            raw_payload.0,
            key.public().into(),
            signature.into(),
            extra,
        ).encode();
        let xt: ExtrinsicFor<P> = Decode::decode(&mut extrinsic.as_slice())?;

        let res = block_on(pool.submit_one(&block_id, xt));
        */
    }
}
