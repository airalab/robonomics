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
//! Robonomics network behaviour.

use futures::executor::block_on;
use libp2p::{
    gossipsub::{Gossipsub, GossipsubEvent},
    kad::{record::store::MemoryStore, Kademlia, KademliaEvent},
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    swarm::behaviour::toggle::Toggle,
    NetworkBehaviour, PeerId,
};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
pub struct RobonomicsNetworkBehaviour {
    pub pubsub: Gossipsub,
    pub mdns: Toggle<Mdns>,
    pub kademlia: Toggle<Kademlia<MemoryStore>>,
}

impl RobonomicsNetworkBehaviour {
    pub fn new(
        pubsub: Gossipsub,
        disable_mdns: bool,
        disable_kad: bool,
        peer_id: PeerId,
    ) -> Result<Self, crate::error::Error> {
        // Build mDNS network behaviour
        let mdns = if !disable_mdns {
            log::info!("Using mDNS discovery service.");
            let mdns = block_on(Mdns::new(MdnsConfig::default()))?;
            Toggle::from(Some(mdns))
        } else {
            Toggle::from(None)
        };

        // Build kademlia network behaviour
        let kademlia = if !disable_kad {
            log::info!("Using DHT discovery service.");
            let store = MemoryStore::new(peer_id);
            let kademlia = Kademlia::new(peer_id.clone(), store);
            Toggle::from(Some(kademlia))
        } else {
            Toggle::from(None)
        };

        // Combined network behaviour
        Ok(RobonomicsNetworkBehaviour {
            pubsub,
            mdns,
            kademlia,
        })
    }
}

#[derive(Debug)]
pub enum OutEvent {
    Pubsub(GossipsubEvent),
    Mdns(MdnsEvent),
    Kademlia(KademliaEvent),
}

impl From<GossipsubEvent> for OutEvent {
    fn from(v: GossipsubEvent) -> Self {
        Self::Pubsub(v)
    }
}

impl From<MdnsEvent> for OutEvent {
    fn from(v: MdnsEvent) -> Self {
        Self::Mdns(v)
    }
}

impl From<KademliaEvent> for OutEvent {
    fn from(v: KademliaEvent) -> Self {
        Self::Kademlia(v)
    }
}
