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
//! Robonomics network worker.

use futures::{prelude::*, Future};
use libp2p::{
    identity::Keypair,
    kad::KademliaEvent,
    request_response::{RequestResponseEvent, RequestResponseMessage},
    swarm::{SwarmBuilder, SwarmEvent},
    Multiaddr, PeerId, Swarm,
};
use std::{
    pin::Pin,
    // sync::Arc,
    task::{Context, Poll},
};

use crate::{
    error::Result,
    network::behaviour::{OutEvent, RobonomicsNetworkBehaviour},
    // pubsub::{PubSub, Pubsub},
    reqres::Response,
};

pub struct NetworkWorker {
    pub swarm: Swarm<RobonomicsNetworkBehaviour>,
}

impl NetworkWorker {
    /// Create new network worker instance
    pub fn new(
        local_key: Keypair,
        heartbeat_interval: u64,
        disable_mdns: bool,
        disable_kad: bool,
    ) -> Result<Self> {
        let peer_id = PeerId::from(local_key.public());

        // Set up an encrypted WebSocket compatible Transport
        let transport = libp2p::tokio_development_transport(local_key.clone())?;

        // Build a combined network behaviour
        let behaviour = RobonomicsNetworkBehaviour::new(
            local_key,
            peer_id,
            heartbeat_interval,
            disable_mdns,
            disable_kad,
        )?;

        // ???
        // use libp2p::gossipsub::IdentTopic;
        // behaviour.pubsub.subscribe(&IdentTopic::new("ROS"))?;

        // Create a Swarm to manage peers and events
        let mut swarm = SwarmBuilder::new(transport, behaviour, peer_id.clone())
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

        // Listen RNB pubsub port
        let listen_address: Multiaddr = "/ip4/127.0.0.1/tcp/30400".parse().unwrap();
        Swarm::listen_on(&mut swarm, listen_address)?;

        Ok(Self { swarm })
    }
}

impl Future for NetworkWorker {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            match self.swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(swarm_event)) => match swarm_event {
                    SwarmEvent::Behaviour(OutEvent::Kademlia(KademliaEvent::RoutingUpdated {
                        peer,
                        // addresses,
                        ..
                    })) => {
                        log::info!("Received kademlia peer: {:?}", peer);
                        if let Some(kademlia) = self.swarm.behaviour_mut().kademlia.as_mut() {
                            if let Err(e) = kademlia.bootstrap() {
                                log::debug!("Bootstrap error: {:?}", e);
                            };
                        }
                        // for address in addresses.iter() {
                        //     let _ = self.pubsub.connect(address.clone());
                        // }
                    }
                    SwarmEvent::Behaviour(OutEvent::RequestResponse(
                        RequestResponseEvent::Message {
                            peer,
                            message:
                                RequestResponseMessage::Response {
                                    request_id,
                                    response,
                                },
                        },
                    )) => match response {
                        Response::Pong => {
                            log::debug!(
                                " peer2 Resp{} {:?} from {:?}",
                                request_id,
                                &response,
                                peer
                            );
                            break;
                        }
                        Response::Data(data) => {
                            let decoded: Vec<u8> = bincode::deserialize(&data.to_vec()).unwrap();
                            log::debug!(
                                " peer2 Resp: Data '{}' from {:?}",
                                String::from_utf8_lossy(&decoded[..]),
                                peer // ???
                            );
                            log::debug!("{}", String::from_utf8_lossy(&decoded[..]));
                            break;
                        }
                    },
                    _ => {}
                },
                Poll::Ready(None) | Poll::Pending => {
                    break;
                }
            }
        }

        Poll::Pending
    }
}
