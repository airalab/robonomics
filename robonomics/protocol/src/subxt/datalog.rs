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

use super::{pallet_datalog, pallet_rws, AccountId, Robonomics};
use crate::error::Result;

use futures::future::join_all;
use pallet_datalog::*;
use pallet_rws::*;
use sp_core::crypto::Pair;
use substrate_subxt::{Call, PairSigner};

/// Sign datalog record and send using remote Robonomics node.
pub async fn submit<T: Pair>(
    signer: T,
    remote: String,
    data_record: Vec<u8>,
    rws: bool,
) -> Result<[u8; 32]>
where
    sp_runtime::MultiSigner: From<<T as Pair>::Public>,
    sp_runtime::MultiSignature: From<<T as Pair>::Signature>,
    <T as Pair>::Signature: codec::Codec,
{
    let subxt_signer = PairSigner::new(signer);
    let client = substrate_subxt::ClientBuilder::<Robonomics>::new()
        .skip_type_sizes_check()
        .set_url(remote.as_str())
        .build()
        .await?;

    let xt_hash = if rws {
        client.call(client.record(&subxt_signer, data_record).await?)
    } else {
        client.record(&subxt_signer, data_record).await?
    };

    log::debug!(
        target: "robonomics-datalog",
        "Data record submited in extrinsic with hash {}", xt_hash
    );
    Ok(xt_hash.into())
}

/// Read datalog records from remote Robonomics node.
pub async fn fetch(robot_account: AccountId, remote: String) -> Result<Vec<(u64, Vec<u8>)>> {
    let client = substrate_subxt::ClientBuilder::<Robonomics>::new()
        .skip_type_sizes_check()
        .set_url(remote.as_str())
        .build()
        .await?;

    let metadata = client.metadata().module("Datalog")?;
    let ws_metadata = metadata.constant("WindowSize")?;
    let window_size = ws_metadata.value()?;

    let mut index = client.datalog_index(&robot_account, None).await?;
    let items = join_all(
        index
            .iter(window_size)
            .map(|i| client.datalog_item((&robot_account, i), None))
            .collect::<Vec<_>>(),
    )
    .await;

    let data = items
        .into_iter()
        .filter_map(|item| item.ok())
        .map(|item| (item.0, item.1))
        .collect();
    Ok(data)
}
