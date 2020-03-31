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
///! Robonomics Gossipsub protocol.
///
/// This implementation use libp2p Gossipsub for effective message delivery.
/// It also implements automatic peer discovery mechanism to build sustainable
/// and high available pubsub swarm.
///

use std::collections::hash_map::{HashMap, DefaultHasher};
use std::hash::{Hash, Hasher};
use std::time::Duration;
use std::ops::DerefMut;
use serde::{Serialize, Deserialize};
use libp2p::{Swarm, PeerId, Multiaddr};
use libp2p::core::nodes::ListenerId;
//use libp2p::futures::prelude::*;
use libp2p::gossipsub::{
    GossipsubConfigBuilder, Gossipsub,
    GossipsubMessage, GossipsubEvent,
    Topic, TopicHash, protocol::MessageId,
};

use crate::error::Result;

/// Peer information service message.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct DiscoveryMessage {
    peer_id: String,
    listeners: Vec<Multiaddr>,
}

/// Peer discovery topic name.
pub const DISCOVERY_TOPIC_NAME: &str = "_robonomics_pubsub_peer_discovery";

/// Gossipsub heartbeat interval
const HEARTBEAT_SECS: u64 = 10;

/// LibP2P Gossipsub based publisher/subscriber service.
pub struct PubSub {
    swarm: Swarm<Gossipsub>,
    subs:  HashMap<TopicHash, Box<dyn FnMut(PeerId, Vec<u8>) + 'static>>,
}

impl PubSub {
    /// Create new Robonomics PubSub instance
    pub fn new() -> Result<Self> {
        // XXX: temporary random local id.
        let local_key = crate::id::random();
        let peer_id = PeerId::from(local_key.public());

        // Set up an encrypted WebSocket compatible Transport over the Mplex and Yamux protocols
        let transport = libp2p::build_tcp_ws_secio_mplex_yamux(local_key)?;

        // Set custom gossipsub
        let gossipsub_config = GossipsubConfigBuilder::new()
            .heartbeat_interval(Duration::from_secs(HEARTBEAT_SECS))
            .message_id_fn(|message: &GossipsubMessage| {
                // To content-address message,
                // we can take the hash of message and use it as an ID.
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                MessageId(s.finish().to_string())
            })
            .build();

        // Build a gossipsub network behaviour
        let mut gossipsub = Gossipsub::new(peer_id.clone(), gossipsub_config);

        // Subscribe to discovery topic
        gossipsub.subscribe(Topic::new(DISCOVERY_TOPIC_NAME.to_string()));

        // Create a Swarm to manage peers and events
        let swarm = Swarm::new(transport, gossipsub, peer_id);

        // Create pubsub instance with empty subscribers
        Ok(PubSub { swarm, subs: HashMap::new() })
    }

    pub async fn start(&mut self) {
        let topic = Topic::new(DISCOVERY_TOPIC_NAME.to_string());
        loop {
            let message = DiscoveryMessage {
                peer_id: Swarm::local_peer_id(&mut self.swarm).to_base58(),
                listeners: Swarm::listeners(&mut self.swarm).cloned().collect(),
            };

            self.swarm.deref_mut().publish(&topic, bincode::serialize(&message).unwrap());
            log::info!(
                target: "robonomics-pubsub",
                "discovery message sended: {:?}", message
            );

            match self.swarm.next().await {
                GossipsubEvent::Message(peer_id, id, message) => {
                    log::info!(
                        target: "robonomics-pubsub",
                        "got message with id: {} from peer: {}", id, peer_id.to_base58()
                    );

                    // Dispatch handlers by topic name hash
                    for topic in message.topics {
                        match self.subs.get_mut(&topic) {
                            None => {
                                let decoded: bincode::Result<DiscoveryMessage>
                                    = bincode::deserialize(&message.data[..]);
                                match decoded {
                                    Ok(message) => for addr in message.listeners {
                                        let _ = Swarm::dial_addr(&mut self.swarm, addr);
                                    },
                                    Err(e) => log::error!(
                                        target: "robonomics-pubsub",
                                        "Unable to decode message from {}: {}", peer_id.to_base58(), e
                                    ),
                                }
                            },
                            Some(handler) => handler(
                                message.source.clone(),
                                message.data.clone(),
                            ),
                        }
                    }
                },
                _ => {}
            }
        }
    }
}

impl super::PubSub for PubSub {
    fn peer_id(&self) -> PeerId {
        Swarm::local_peer_id(&self.swarm).clone()
    }

    fn listen(&mut self, address: &Multiaddr) -> Result<ListenerId> {
        let listener = Swarm::listen_on(&mut self.swarm, address.clone())?;
        log::debug!(
            target: "robonomics-pubsub",
            "Listener for address {} created: {:?}", address, listener
        );
        Ok(listener)
    }

    fn listeners(&self) -> Vec<Multiaddr> {
        Swarm::listeners(&self.swarm).cloned().collect()
    }

    fn connect(&mut self, address: &Multiaddr) -> Result<()> {
        Swarm::dial_addr(&mut self.swarm, address.clone())?;
        log::debug!(
            target: "robonomics-pubsub",
            "Connecting to {}", address
        );
        Ok(())
    }

    fn subscribe<T, F>(&mut self, topic_name: T, callback: F) -> bool
        where T: ToString, F: FnMut(PeerId, Vec<u8>) + 'static
    {
        log::debug!(target: "robonomics-pubsub", "Subscribed to {}", topic_name.to_string());

        let topic = Topic::new(topic_name.to_string());
        self.subs.insert(topic.sha256_hash(), Box::new(callback));
        self.swarm.subscribe(topic)
    }

    fn unsubscribe<T: ToString>(&mut self, topic_name: T) -> bool {
        log::debug!(target: "robonomics-pubsub", "Unsubscribed from {}", topic_name.to_string());

        let topic = Topic::new(topic_name.to_string());
        self.swarm.deref_mut().unsubscribe(topic)
    }

    fn publish<T: ToString, M: Into<Vec<u8>>>(&mut self, topic_name: T, message: M) {
        log::debug!(target: "robonomics-pubsub", "Publish to {}", topic_name.to_string());

        let topic = Topic::new(topic_name.to_string());
        self.swarm.deref_mut().publish(&topic, message)
    }
}
