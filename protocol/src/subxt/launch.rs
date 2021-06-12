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
//! Launch CPS using Robonomics network.

use super::{pallet_launch::*, pallet_rws::*, AccountId, Robonomics};
use crate::error::{Error, Result};

use codec::Decode;
use sp_core::crypto::{Pair, Ss58Codec};
use substrate_subxt::{EventSubscription, PairSigner};

/// Send launch request using remote Robonomics node.
pub async fn submit<T: Pair>(
    signer: T,
    remote: String,
    robot: String,
    param: bool,
    rws: Option<String>,
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

    let xt_hash = if let Some(subscription) = rws {
        let call = client.encode(LaunchCall {
            robot: robot_account,
            param,
        })?;
        let subscription_account =
            AccountId::from_ss58check(subscription.as_str()).map_err(|_| Error::Ss58CodecError)?;
        client
            .call(&subxt_signer, &subscription_account, &call)
            .await?
    } else {
        client.launch(&subxt_signer, robot_account, param).await?
    };

    log::debug!(
        target: "robonomics-launch",
        "Launch request submited in extrinsic with hash {}", xt_hash
    );
    Ok(xt_hash.into())
}

/// Listen for incoming launch requests.
pub async fn listen(
    remote: String,
    mut callback: impl FnMut(NewLaunchEvent<Robonomics>),
) -> Result<()> {
    let client = substrate_subxt::ClientBuilder::<Robonomics>::new()
        .set_url(remote.as_str())
        .build()
        .await?;

    let sub = client.subscribe_events().await?;
    let mut sub = EventSubscription::<Robonomics>::new(sub, client.events_decoder());
    sub.filter_event::<NewLaunchEvent<_>>();
    while let Some(Ok(raw)) = sub.next().await {
        if let Ok(event) = NewLaunchEvent::<Robonomics>::decode(&mut &raw.data[..]) {
            callback(event)
        } else {
            log::warn!("Unable decode launch event: {:?}", raw);
        }
    }

    Ok(())
}
