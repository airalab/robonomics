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
//! Launch CPS using Robonomics network.

use crate::error::{Error, Result};
use crate::runtime::pallet_launch;
use crate::runtime::{AccountId, Robonomics};

use pallet_launch::*;
use sp_core::crypto::{Pair, Ss58Codec};
use substrate_subxt::{EventSubscription, EventsDecoder, PairSigner};

/// Send launch request using remote Robonomics node.
pub async fn submit<T: Pair>(
    signer: T,
    remote: String,
    robot: String,
    param: bool,
) -> Result<[u8; 32]>
where
    sp_runtime::MultiSigner: From<<T as Pair>::Public>,
    sp_runtime::MultiSignature: From<<T as Pair>::Signature>,
    <T as Pair>::Signature: codec::Codec,
{
    let subxt_signer = PairSigner::new(signer);
    let robot_account =
        AccountId::from_ss58check(robot.as_str()).map_err(|_| Error::Ss58CodecError)?;
    let client = substrate_subxt::ClientBuilder::<Robonomics>::new()
        .set_url(remote.as_str())
        .build()
        .await?;
    let xt_hash = client.launch(&subxt_signer, robot_account, param).await?;
    log::debug!(
        target: "robonomics-launch",
        "Launch request submited in extrinsic with hash {}", xt_hash
    );
    Ok(xt_hash.into())
}

/// Listen for incoming launch requests.
pub async fn listen(remote: String) -> Result<EventSubscription<Robonomics>> {
    let client = substrate_subxt::ClientBuilder::<Robonomics>::new()
        .set_url(remote.as_str())
        .build()
        .await?;

    let sub = client.subscribe_events().await?;
    let decoder = EventsDecoder::<Robonomics>::new(client.metadata().clone());
    let mut sub = EventSubscription::<Robonomics>::new(sub, decoder);
    sub.filter_event::<NewLaunchEvent<_>>();

    Ok(sub)
}
