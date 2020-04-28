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
use futures::{future, FutureExt};
use sp_core::{sr25519, crypto::Pair};
use crate::error::Result;
use crate::pipe::{Pipe, PipeFuture, Consumer};
use std::time::Duration;
use async_std::task;
use std::sync::Arc;

/// Standart console output.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Stdout;

impl Stdout {
    pub fn new() -> Self { Self }
}

impl<'a> Pipe<'a, String, ()> for Stdout {
    fn exec(&mut self, input: String) -> PipeFuture<'a, ()> {
        println!("{}", input);
        Box::pin(future::ready(()))
    }
}

impl<'a> Consumer<'a, String> for Stdout {}

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

impl<'a> Pipe<'a, String, ()> for PubSub {
    fn exec(&mut self, input: String) -> PipeFuture<'a, ()> {
        self.pubsub.publish(&self.topic_name, input);
        Box::pin(future::ready(()))
    }
}

impl<'a> Consumer<'a, String> for PubSub {}

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

impl<'a> Pipe<'a, Vec<u8>, ()> for Datalog {
    fn exec(&mut self, input: Vec<u8>) -> PipeFuture<'a, ()> {
        Box::pin(datalog::submit(
            self.pair.clone(),
            self.remote.clone(),
            input,
        ).map(|_| ()))
    }
}

impl<'a> Consumer<'a, Vec<u8>> for Datalog {}
