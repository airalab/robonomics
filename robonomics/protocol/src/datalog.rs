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
//! Robonomics data blockchainization.

use crate::error::Result;
use crate::runtime::{pallet_datalog, Robonomics};

use pallet_datalog::*;
use sp_core::crypto::Pair;
use substrate_subxt::{PairSigner, Signer};

/// Sign datalog record and send using remote Robonomics node.
pub async fn submit<T: Pair>(signer: T, remote: String, data_record: Vec<u8>) -> Result<[u8; 32]>
where
    sp_runtime::MultiSigner: From<<T as Pair>::Public>,
    sp_runtime::MultiSignature: From<<T as Pair>::Signature>,
    <T as Pair>::Signature: codec::Codec,
{
    let subxt_signer = PairSigner::new(signer);
    let client = substrate_subxt::ClientBuilder::<Robonomics>::new()
        .set_url(remote.as_str())
        .build()
        .await?;
    let xt_hash = client.record(&subxt_signer, data_record).await?;
    log::debug!(
        target: "robonomics-datalog",
        "Data record submited in extrinsic with hash {}", xt_hash
    );
    Ok(xt_hash.into())
}

/// Read datalog records from remote Robonomics node.
pub async fn fetch<T: Pair>(signer: T, remote: String) -> Result<Vec<(u64, Vec<u8>)>>
where
    sp_runtime::MultiSigner: From<<T as Pair>::Public>,
    sp_runtime::MultiSignature: From<<T as Pair>::Signature>,
    <T as Pair>::Signature: codec::Codec,
{
    let signer = PairSigner::new(signer);
    let client = substrate_subxt::ClientBuilder::<Robonomics>::new()
        .set_url(remote.as_str())
        .build()
        .await?;

    let data = client.datalog(&signer.account_id(), None).await?;
    Ok(data.into())
}
