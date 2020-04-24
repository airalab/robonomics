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
///! Virtual actuators collection.
///
/// This module contains:
/// - Stdout: Standart output stream. 
///

use robonomics_protocol::datalog;
use robonomics_protocol::pubsub::{
    self, Multiaddr, PubSub as PubSubT,
};
use futures::{Stream, StreamExt, Future, future};
use sp_core::{sr25519, crypto::Pair};
use std::io::{self, Write};
use crate::pipe::Consumer;
use crate::error::Result;
use async_std::task;
use std::sync::Arc;

/// Simple standart output.
pub struct Stdout;

impl Stdout {
    pub fn new() -> Self { Self }
}

impl Consumer for Stdout {
    type In = Box<dyn Stream<Item = String> + Unpin>;
    type Out = Box<dyn Future<Output = ()> + Unpin>;

    fn consume(self, input: Self::In) -> Self::Out {
        Box::new(input.for_each(|msg| {
            io::stdout()
                .write_all(msg.as_bytes())
                .expect("unable to write to stdout");
            future::ready(())
        }))
    }
}

/// PubSub publisher.
pub struct PubSub {
    pubsub: Arc<pubsub::Gossipsub>,
    topic_name: String,
}

impl PubSub {
    pub fn new(
        listen: Multiaddr,
        bootnodes: Vec<Multiaddr>,
        topic_name: String,
    ) -> Result<Self> {
        let (pubsub, worker) = pubsub::Gossipsub::new().unwrap();

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

impl Consumer for PubSub {
    type In = Box<dyn Stream<Item = String> + Unpin>;
    type Out = Box<dyn Future<Output = ()> + Unpin>;

    fn consume(self, input: Self::In) -> Self::Out {
        Box::new(input.for_each(move |msg| {
            self.pubsub.publish(&self.topic_name, msg);
            future::ready(())
        }))
    }
}

/// Datalog submitter.
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

impl Consumer for Datalog {
    type In = Box<dyn Stream<Item = String> + Unpin>;
    type Out = Box<dyn Future<Output = ()> + Unpin>;

    fn consume(self, input: Self::In) -> Self::Out {
        Box::new(input.for_each(move |record| {
            let _ = task::block_on(
                datalog::submit(
                    self.pair.clone(),
                    self.remote.as_str(),
                    record.as_bytes().to_vec(),
                )
            );
            future::ready(())
        }))
    }
}
