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
//! Collection of virtual devices (like stdout).

use robonomics_protocol::datalog;
use robonomics_protocol::pubsub::{
    self, Multiaddr, PubSub as PubSubT,
};
use futures::{future, Future, FutureExt, StreamExt};
use sp_core::{sr25519, crypto::Pair};
use crate::error::Result;
use ipfs_api::IpfsClient;
use std::time::Duration;
use std::io::Cursor;
use async_std::task;
use std::sync::Arc;

/// Publish data into PubSub topic. 
pub struct PubSub {
    pubsub: Arc<pubsub::Gossipsub>,
    topic_name: String,
}

impl PubSub {
    pub fn new(
        listen: Multiaddr,
        bootnodes: Vec<Multiaddr>,
        topic_name: String,
        heartbeat: Duration,
    ) -> Result<Self> {
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
        Ok(Self { pubsub, topic_name }) 
    }
}

impl super::AsyncSink<String, ()> for PubSub {
    fn sink(&mut self, input: String) -> super::ImplFuture<()> { 
        self.pubsub.publish(&self.topic_name, input);
        Box::pin(future::ready(()))
    }
}

/// Submit signed data record into blockchain.
pub struct Datalog {
    remote: String,
    pair: sr25519::Pair,
}

impl Datalog {
    pub fn new(remote: String, suri: String) -> Result<Self> {
        let pair = sr25519::Pair::from_string(suri.as_str(), None)?;
        Ok(Self { remote, pair })
    }
}

impl super::AsyncSink<Vec<u8>, ()> for Datalog {
    fn sink(&mut self, input: Vec<u8>) -> super::ImplFuture<()> {
        Box::pin(datalog::submit(
            self.pair.clone(),
            self.remote.clone(),
            input,
        ).map(|_| ()))
    }
}

pub struct Ipfs(IpfsClient);

impl Ipfs {
    pub fn new(uri: &str) -> Self {
        let client = IpfsClient::new_from_uri(uri)
            .expect("IPFS API uri should be valid");
        Ipfs(client)
    }
}

impl super::AsyncSink<Vec<u8>, Result<String>> for Ipfs {
    fn sink(&mut self, input: Vec<u8>) -> super::ImplFuture<Result<String>> {
        self.0
            .add(Cursor::new(input))
            .map(|item| item.map(|value| value.hash).map_err(Into::into))
    }
}
