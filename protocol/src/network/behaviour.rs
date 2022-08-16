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
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage, MessageAuthenticity,
        MessageId,
    },
    identity::Keypair,
    kad::{record::store::MemoryStore, Kademlia, KademliaEvent},
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    request_response::{
        ProtocolSupport, RequestResponse, RequestResponseConfig, RequestResponseEvent,
    },
    swarm::behaviour::toggle::Toggle,
    NetworkBehaviour, PeerId,
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::Duration,
};

use crate::{
    error::Result,
    reqres::{Request, Response, RobonomicsCodec, RobonomicsProtocol},
};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
pub struct RobonomicsNetworkBehaviour {
    pub pubsub: Gossipsub,
    pub mdns: Toggle<Mdns>,
    pub kademlia: Toggle<Kademlia<MemoryStore>>,
    pub request_response: RequestResponse<RobonomicsCodec>,
}

impl RobonomicsNetworkBehaviour {
    pub fn new(
        local_key: Keypair,
        peer_id: PeerId,
        heartbeat_interval: Duration,
        disable_mdns: bool,
        disable_kad: bool,
    ) -> Result<Self> {
        // Set custom gossipsub
        let gossipsub_config = GossipsubConfigBuilder::default()
            .heartbeat_interval(heartbeat_interval)
            .message_id_fn(|message: &GossipsubMessage| {
                // To content-address message,
                // we can take the hash of message and use it as an ID.
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                MessageId::from(s.finish().to_string())
            })
            .build()?;

        // Build a gossipsub network behaviour
        let pubsub = Gossipsub::new(MessageAuthenticity::Signed(local_key), gossipsub_config)?;

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

        // Build request-response network behaviour
        let protocols = std::iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
        let config = RequestResponseConfig::default();
        let request_response =
            RequestResponse::new(RobonomicsCodec { is_ping: false }, protocols, config);

        // Combined network behaviour
        Ok(RobonomicsNetworkBehaviour {
            pubsub,
            mdns,
            kademlia,
            request_response,
        })
    }
}

#[derive(Debug)]
pub enum OutEvent {
    Pubsub(GossipsubEvent),
    Mdns(MdnsEvent),
    Kademlia(KademliaEvent),
    RequestResponse(RequestResponseEvent<Request, Response>),
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

impl From<RequestResponseEvent<Request, Response>> for OutEvent {
    fn from(v: RequestResponseEvent<Request, Response>) -> Self {
        Self::RequestResponse(v)
    }
}
