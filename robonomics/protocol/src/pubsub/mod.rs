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
//! Robonomics Network publisher/subscriber module.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use libp2p::gossipsub::{
    GossipsubConfigBuilder, Topic, Gossipsub,
    GossipsubMessage, protocol::MessageId,
};
use libp2p::Swarm;

use crate::error::Result;

#[cfg(feature = "cli")]
pub mod cli;

/// Gossipsub heartbeat interval
const HEARDBEAT_SECS: u64 = 10;

/// To content-address message,
/// we can take the hash of message and use it as an ID.
fn message_id(message: &GossipsubMessage) -> MessageId {
    let mut s = DefaultHasher::new();
    message.data.hash(&mut s);
    MessageId(s.finish().to_string())
}

/// Create new gossipsub swarm and subscribe to topic argument.
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
