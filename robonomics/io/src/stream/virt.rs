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

use robonomics_protocol::pubsub::{
    self, Multiaddr, PubSub as PubSubT,
};
use futures::{Stream, StreamExt};
use crate::error::Result;
use async_std::task;
use std::task::{Context, Poll};
use std::time::Duration;
use std::pin::Pin;

/// Subscribe for data from PubSub topic.
pub struct PubSub(Pin<Box<dyn Stream<Item = pubsub::Message> + Send>>);

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
        Ok(Self(pubsub.subscribe(&topic_name))) 
    }
}

impl Stream for PubSub {
    type Item = pubsub::Message;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_next_unpin(cx)
    }
}
