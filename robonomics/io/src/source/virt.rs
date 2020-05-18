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
//! Virtual sensors collection.

use robonomics_protocol::pubsub::{self, Multiaddr, PubSub as PubSubT};
use ipfs_api::{IpfsClient, TryFromUri};
use futures::channel::mpsc;
use async_std::{io, task};
use futures::prelude::*;
use std::time::Duration;

use crate::error::{Result, Error};

/// Read line from standard console input.
pub fn stdin() -> impl Stream<Item = Result<String>> {
    let lines = io::BufReader::new(io::stdin()).lines();
    lines.map(|r| r.map_err(Into::into))
}

/// Subscribe for data from PubSub topic.
pub fn pubsub(
    listen: Multiaddr,
    bootnodes: Vec<Multiaddr>,
    topic_name: String,
    heartbeat: Duration,
) -> Result<impl Stream<Item = Result<pubsub::Message>>> {
    let (pubsub, worker) = pubsub::Gossipsub::new(heartbeat)?;

    // Listen address
    let _ = pubsub.listen(listen);

    // Connect to bootnodes
    for addr in bootnodes {
        let _ = pubsub.connect(addr);
    }

    // Spawn peer discovery
    task::spawn(pubsub::discovery::start(pubsub.clone()));

    // Spawn network worker
    task::spawn(worker);

    // Subscribe to given topic
    Ok(pubsub.subscribe(&topic_name).map(|v| Ok(v)))
}

/// Download some data from IPFS network.
///
/// Returns IPFS data objects.
pub fn ipfs(
    uri: &str,
) -> Result<(impl Sink<String, Error = Error>, impl Stream<Item = Result<Vec<u8>>>)> {
    let client = IpfsClient::from_str(uri).expect("unvalid uri");
    let mut runtime = tokio::runtime::Runtime::new().expect("unable to start runtime");

    let (sender, receiver) = mpsc::unbounded();
    let datas = receiver.map(move |msg: String|
        runtime.block_on(client.cat(msg.as_str()).map_ok(|c| c.to_vec()).try_concat())
            .map_err(Into::into)
    );
    Ok((sender.sink_err_into(), datas))
}
