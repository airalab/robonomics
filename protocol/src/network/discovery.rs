///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2023 Robonomics Network <research@robonomics.network>
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
//! Robonomics protocol node discovery.

use super::behaviour::RobonomicsNetworkBehaviour;
use libp2p::{Multiaddr, PeerId, Swarm};
use std::collections::HashMap;

pub fn add_explicit_peers(
    swarm: &mut Swarm<RobonomicsNetworkBehaviour>,
    peers: &mut HashMap<PeerId, Multiaddr>,
    bootnodes: Vec<String>,
    disable_kad: bool,
) {
    for node in bootnodes {
        if let Ok(addr) = node.parse::<Multiaddr>() {
            if let Some(peer) = PeerId::try_from_multiaddr(&addr) {
                peers.insert(peer, addr.clone());

                // Add node to PubSub
                swarm.behaviour_mut().pubsub.add_explicit_peer(&peer);

                // Add node to DHT
                if !disable_kad {
                    if let Some(kademlia) = swarm.behaviour_mut().kademlia.as_mut() {
                        kademlia.add_address(&peer, addr);
                    };
                }
            }
        }
    }
}
