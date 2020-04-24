///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life> 
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

use crate::error::Result;
use crate::runtime::Robonomics;
use crate::runtime::pallet_datalog;
use sp_core::crypto::Pair;

/// Main
pub async fn submit<T: Pair>(signer: T, remote: String, record: Vec<u8>) -> Result<()>
    where sp_runtime::MultiSigner: From<<T as Pair>::Public>,
          sp_runtime::MultiSignature: From<<T as Pair>::Signature>,
          <T as Pair>::Signature: codec::Codec,
{
    let xt_hash = substrate_subxt::ClientBuilder::<Robonomics>::new()
        .set_url(remote.as_str())
        .build().await?
        .xt(signer, None).await?
        .submit(pallet_datalog::record::<Robonomics>(record))
        .await?;
    log::info!(
        target: "robonomics-datalog",
        "Data record submited in extrinsic with hash {}", xt_hash
    );
    Ok(())
}
