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

use libp2p::{
    gossipsub,
    identity::Keypair,
    kad::{record::store::MemoryStore, Kademlia, KademliaEvent},
    mdns,
    request_response,
    swarm::behaviour::toggle::Toggle,
    swarm::NetworkBehaviour, PeerId,
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::Duration,
};

use crate::{
    error,
    reqres,
};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
pub struct RobonomicsNetworkBehaviour {
    pub pubsub: gossipsub::Behaviour,
    pub mdns: Toggle<mdns::tokio::Behaviour>,
    pub kademlia: Toggle<Kademlia<MemoryStore>>,
}

impl RobonomicsNetworkBehaviour {
    pub fn new(
        local_key: Keypair,
        peer_id: PeerId,
        heartbeat_interval: u64,
        disable_mdns: bool,
        disable_kad: bool,
    ) -> error::Result<Self> {
        // Set custom gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_millis(heartbeat_interval))
            .message_id_fn(|message: &gossipsub::Message| {
                // To content-address message,
                // we can take the hash of message and use it as an ID.
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            })
            .build()?;

        // Build a gossipsub network behaviour
        let pubsub = gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Signed(local_key), gossipsub_config)?;

        // Build mDNS network behaviour
        let mdns = if !disable_mdns {
            log::info!("Using mDNS discovery service.");
            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?;
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
    Pubsub(gossipsub::Event),
    Mdns(mdns::Event),
    Kademlia(KademliaEvent),
    RequestResponse(request_response::Event<reqres::Request, reqres::Response>),
}

impl From<gossipsub::Event> for OutEvent {
    fn from(v: gossipsub::Event) -> Self {
        Self::Pubsub(v)
    }
}

impl From<mdns::Event> for OutEvent {
    fn from(v: mdns::Event) -> Self {
        Self::Mdns(v)
    }
}

impl From<KademliaEvent> for OutEvent {
    fn from(v: KademliaEvent) -> Self {
        Self::Kademlia(v)
    }
}

impl From<request_response::Event<reqres::Request, reqres::Response>> for OutEvent {
    fn from(v: request_response::Event<reqres::Request, reqres::Response>) -> Self {
        Self::RequestResponse(v)
    }
}
