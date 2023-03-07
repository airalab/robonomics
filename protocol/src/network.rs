///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2022 Robonomics Network <research@robonomics.network>
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
//! Robonomics network layer.

use futures::prelude::*;
use libp2p::{identity::Keypair, Multiaddr, PeerId};
use std::sync::Arc;

use crate::{
    error::{FutureResult, Result},
    network::worker::NetworkWorker,
    pubsub::{Inbox, PubSub, Pubsub},
};

pub mod behaviour;
pub mod discovery;
pub mod worker;

pub struct RobonomicsNetwork {
    pub pubsub: Arc<Pubsub>,
}

impl RobonomicsNetwork {
    pub fn new(
        local_key: Keypair,
        pubsub: Arc<Pubsub>,
        heartbeat_interval: u64,
        bootnodes: Vec<String>,
        disable_mdns: bool,
        disable_kad: bool,
    ) -> Result<(Arc<Self>, impl Future<Output = ()>)> {
        let mut network_worker =
            NetworkWorker::new(local_key, heartbeat_interval, disable_mdns, disable_kad)?;

        // Reach out to another nodes if specified
        // discovery::add_peers(&mut network_worker.swarm, bootnodes);

        Ok((Arc::new(Self { pubsub }), network_worker))
    }
}

impl PubSub for RobonomicsNetwork {
    fn peer_id(&self) -> PeerId {
        self.pubsub.peer_id()
    }

    fn listen(&self, address: Multiaddr) -> FutureResult<bool> {
        self.pubsub.listen(address)
    }

    fn listeners(&self) -> FutureResult<Vec<Multiaddr>> {
        self.pubsub.listeners()
    }

    fn connect(&self, address: Multiaddr) -> FutureResult<bool> {
        self.pubsub.connect(address)
    }

    fn subscribe<T: ToString>(&self, topic_name: &T) -> Inbox {
        self.pubsub.subscribe(&topic_name.to_string())
    }

    fn unsubscribe<T: ToString>(&self, topic_name: &T) -> FutureResult<bool> {
        self.pubsub.unsubscribe(&topic_name.to_string())
    }

    fn publish<T: ToString, M: Into<Vec<u8>>>(
        &self,
        topic_name: &T,
        message: M,
    ) -> FutureResult<bool> {
        self.pubsub.publish(&topic_name.to_string(), message.into())
    }
}
