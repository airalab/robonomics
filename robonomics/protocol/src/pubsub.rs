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
//! Robonomics Network agent functionality.

use std::collections::hash_map::DefaultHasher;
use std::task::{Context, Poll};
use std::hash::{Hash, Hasher};
use std::time::Duration;

use futures::prelude::*;
use async_std::task;

use libp2p::gossipsub::{
    GossipsubConfigBuilder, Topic, Gossipsub,
    GossipsubMessage, GossipsubEvent,
    protocol::MessageId,
};
use libp2p::{Swarm, Multiaddr};

use crate::error::Result;

/// Gossipsub heartbeat interval
const HEARDBEAT_SECS: u64 = 10;

/// The PubSub command for pubsub router mode.
#[derive(Debug, structopt::StructOpt, Clone)]
pub struct PubSubCmd {
    #[structopt(short, long)]
    pub topic: String,
    #[structopt(short, long)]
    pub listen: Multiaddr,
    #[structopt(short, long, use_delimiter = true)]
    pub bootnodes: Vec<Multiaddr>,
}

impl PubSubCmd {
    pub fn run(&self) -> Result<()> {
        let mut swarm = new_pubsub(self.topic.clone())?;

        let listener = Swarm::listen_on(&mut swarm, self.listen.clone())?;
        log::info!(target: "robonomics-pubsub",
                   "listener for address {} created: {:?}", self.listen, listener);

        for addr in self.bootnodes.clone() {
            Swarm::dial_addr(&mut swarm, addr.clone())?;
        }

        task::block_on(future::poll_fn(move |cx: &mut Context| {
            loop {
                match swarm.poll_next_unpin(cx) {
                    Poll::Ready(Some(gossip_event)) => match gossip_event {
                        GossipsubEvent::Message(peer_id, id, message) => log::info!(
                            target: "robonomics-pubsub",
                            "got message: {} with id: {} from peer: {:?}",
                            String::from_utf8_lossy(&message.data),
                            id,
                            peer_id
                        ),
                        _ => {}
                    },
                    Poll::Ready(None) | Poll::Pending => break,
                }
            }

            for a in Swarm::external_addresses(&swarm) {
                log::info!(target: "robonomics-pubsub",
                           "external address {}", a);
            }

            for addr in libp2p::Swarm::listeners(&swarm) {
                log::info!(target: "robonomics-pubsub",
                           "listening on {:?}", addr);
            }

            Poll::Pending
        }))
    }
}

/// To content-address message,
/// we can take the hash of message and use it as an ID.
fn message_id(message: &GossipsubMessage) -> MessageId {
    let mut s = DefaultHasher::new();
    message.data.hash(&mut s);
    MessageId(s.finish().to_string())
}

pub fn new_pubsub(
    topic_name: String,
) -> Result<Swarm<Gossipsub>> {
    let (local_key, peer_id) = crate::crypto::random_id();

    // Set up an encrypted WebSocket compatible Transport over the Mplex and Yamux protocols
    let transport = libp2p::build_tcp_ws_secio_mplex_yamux(local_key)?;

    // Set custom gossipsub
    let gossipsub_config = GossipsubConfigBuilder::new()
        .heartbeat_interval(Duration::from_secs(HEARDBEAT_SECS))
        .message_id_fn(message_id)
        .build();

    // Build a gossipsub network behaviour
    let mut gossipsub = Gossipsub::new(peer_id.clone(), gossipsub_config);

    // Subscribe to topic
    gossipsub.subscribe(Topic::new(topic_name));

    // Create a Swarm to manage peers and events
    Ok(Swarm::new(transport, gossipsub, peer_id))
}
