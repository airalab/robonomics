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
use libp2p::{
    identity::Keypair,
    kad::KademliaEvent,
    request_response::{RequestResponseEvent, RequestResponseMessage},
    swarm::{SwarmBuilder, SwarmEvent},
    Multiaddr, PeerId, Swarm,
};
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::{
    error::{FutureResult, Result},
    network::{
        behaviour::{OutEvent, RobonomicsNetworkBehaviour},
        worker::NetworkWorker,
    },
    pubsub::{Inbox, PubSub, Pubsub},
    reqres::Response,
};

pub mod behaviour;
pub mod discovery;
pub mod worker;

pub struct Network {
    swarm: Swarm<RobonomicsNetworkBehaviour>,
}

impl Network {
    pub fn new(
        local_key: Keypair,
        heartbeat_interval: u64,
        bootnodes: Vec<String>,
        disable_mdns: bool,
        disable_kad: bool,
    ) -> Result<Self> {
        let peer_id = PeerId::from(local_key.public());
        let transport = libp2p::tokio_development_transport(local_key.clone())?;
        let behaviour = RobonomicsNetworkBehaviour::new(
            local_key,
            peer_id,
            heartbeat_interval,
            disable_mdns,
            disable_kad,
        )?;
        let mut swarm = SwarmBuilder::new(transport, behaviour, peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

        Swarm::listen_on(&mut swarm, "/ip4/127.0.0.1/tcp/30400".parse().unwrap())?;
        discovery::add_peers(&mut swarm, bootnodes);

        Ok(Self { swarm })
    }
}

impl Future for Network {
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
                        // if let Some(kademlia) = self.swarm.behaviour_mut().kademlia.as_mut() {
                        //     if let Err(e) = kademlia.bootstrap() {
                        //         log::debug!("Bootstrap error: {:?}", e);
                        //     };
                        // }
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

// impl PubSub for Network {
//     fn peer_id(&self) -> PeerId {
//         self.swarm.behaviour_mut().pubsub.peer_id()
//     }
//
//     fn listen(&self, address: Multiaddr) -> FutureResult<bool> {
//         self.swarm.behaviour_mut().pubsub.listen(address)
//     }
//
//     fn listeners(&self) -> FutureResult<Vec<Multiaddr>> {
//         self.swarm.behaviour_mut().pubsub.listeners()
//     }
//
//     fn connect(&self, address: Multiaddr) -> FutureResult<bool> {
//         self.swarm.behaviour_mut().pubsub.connect(address)
//     }
//
//     fn subscribe<T: ToString>(&self, topic_name: &T) -> Inbox {
//         self.swarm
//             .behaviour_mut()
//             .pubsub
//             .subscribe(&topic_name.to_string())
//     }
//
//     fn unsubscribe<T: ToString>(&self, topic_name: &T) -> FutureResult<bool> {
//         self.swarm
//             .behaviour_mut()
//             .pubsub
//             .unsubscribe(&topic_name.to_string())
//     }
//
//     fn publish<T: ToString, M: Into<Vec<u8>>>(
//         &self,
//         topic_name: &T,
//         message: M,
//     ) -> FutureResult<bool> {
//         self.swarm
//             .behaviour_mut()
//             .pubsub
//             .publish(&topic_name.to_string(), message.into())
//     }
// }

//------------------------------------------------

pub struct RobonomicsNetwork {
    pub pubsub: Arc<Pubsub>,
}

impl RobonomicsNetwork {
    pub fn new(
        local_key: Keypair,
        pubsub: Arc<Pubsub>,
        heartbeat_interval: u64,
        _bootnodes: Vec<String>,
        disable_mdns: bool,
        disable_kad: bool,
    ) -> Result<(Arc<Self>, impl Future<Output = ()>)> {
        let network_worker =
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
