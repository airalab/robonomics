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
use std::hash::{Hash, Hasher};
use std::time::Duration;

use libp2p::identity::Keypair;
use libp2p::gossipsub::{
    GossipsubConfigBuilder, Topic, Gossipsub,
    GossipsubMessage, protocol::MessageId,
};
use libp2p::{PeerId, Swarm};

pub fn pubsub_at(topic_name: String) -> Result<Swarm<Gossipsub>, String> {
    let local_key = Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("PeerId: {:?}", local_peer_id);

    // Set up an encrypted WebSocket compatible Transport over the Mplex and Yamux protocols
    let transport = libp2p::build_tcp_ws_secio_mplex_yamux(local_key)
        .map_err(|e| format!("transport build error: {}", e))?;

    // Create a Swarm to manage peers and events
    let mut swarm = {
        // To content-address message, we can take the hash of message and use it as an ID.
        let message_id_fn = |message: &GossipsubMessage| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            MessageId(s.finish().to_string())
        };

        // Set custom gossipsub
        let gossipsub_config = GossipsubConfigBuilder::new()
            .heartbeat_interval(Duration::from_secs(10))
            .message_id_fn(message_id_fn)
            // content-address messages. No two messages of the same content will be propagated.
            .build();

        // Build a gossipsub network behaviour
        let mut gossipsub = Gossipsub::new(local_peer_id.clone(), gossipsub_config);

        // Create a Floodsub/Gossipsub topic
        gossipsub.subscribe(Topic::new(topic_name));

        Swarm::new(transport, gossipsub, local_peer_id)
    };

    // Listen on all interfaces and whatever port the OS assigns
    let addr = Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .map_err(|e| format!("Listen error: {}", e))?;
    println!("Listening on {:?}", addr);

    Ok(swarm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::task;
    use futures::prelude::*;
    use std::task::{Context, Poll};
    use libp2p::gossipsub::GossipsubEvent;

    #[test]
    fn listen_on() {
        let mut swarm = pubsub_at("test_pubsub".to_string()).unwrap();
        task::block_on(future::poll_fn(move |cx: &mut Context| {
            loop {
                match swarm.poll_next_unpin(cx) {
                    Poll::Ready(Some(gossip_event)) => match gossip_event {
                        GossipsubEvent::Message(peer_id, id, message) => println!(
                            "Got message: {} with id: {} from peer: {:?}",
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
                println!("External address {:?}", a);
            }

            for addr in libp2p::Swarm::listeners(&swarm) {
                println!("Listening on {:?}", addr);
            }

            Poll::Pending
        }))
    }
}
