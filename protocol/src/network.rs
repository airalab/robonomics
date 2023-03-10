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

use futures::{
    channel::{mpsc, oneshot},
    prelude::*,
};
use libp2p::{
    core::transport::ListenerId,
    gossipsub::{GossipsubEvent, IdentTopic as Topic, TopicHash},
    identity::Keypair,
    kad::KademliaEvent,
    request_response::{RequestResponseEvent, RequestResponseMessage},
    swarm::{SwarmBuilder, SwarmEvent},
    Multiaddr, PeerId, Swarm,
};
use std::{
    collections::hash_map::HashMap,
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
    pubsub::{Inbox, Message, PubSub, Pubsub, ToWorkerMsg},
    reqres::Response,
};

pub mod behaviour;
pub mod discovery;
pub mod worker;

pub struct RNetwork {
    swarm: Swarm<RobonomicsNetworkBehaviour>,
    inbox: HashMap<TopicHash, mpsc::UnboundedSender<Message>>,
    from_service: mpsc::UnboundedReceiver<ToWorkerMsg>,
    pub service: Arc<Pubsub>,
}

impl RNetwork {
    pub fn new(
        local_key: Keypair,
        heartbeat_interval: u64,
        bootnodes: Vec<String>,
        disable_mdns: bool,
        disable_kad: bool,
    ) -> Result<Self> {
        let peer_id = PeerId::from(local_key.public());
        let transport = libp2p::tokio_development_transport(local_key.clone())?;
        let mut behaviour = RobonomicsNetworkBehaviour::new(
            local_key,
            peer_id,
            heartbeat_interval,
            disable_mdns,
            disable_kad,
        )?;

        // XXX
        let topic = libp2p::gossipsub::IdentTopic::new("ROS");
        behaviour.pubsub.subscribe(&topic)?;

        let mut swarm = SwarmBuilder::new(transport, behaviour, peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

        Swarm::listen_on(&mut swarm, "/ip4/127.0.0.1/tcp/30400".parse().unwrap())?;
        // Swarm::listen_on(&mut swarm, "/ip4/127.0.0.1/tcp/30401".parse().unwrap())?;
        discovery::add_peers(&mut swarm, bootnodes);

        // Create worker communication channel
        let (to_worker, from_service) = mpsc::unbounded::<ToWorkerMsg>();

        // Create PubSub service
        let service = Arc::new(Pubsub::create(to_worker, peer_id));
        // let service = Arc::new(Pubsub { to_worker, peer_id });

        Ok(Self {
            swarm,
            inbox: HashMap::new(),
            from_service,
            service,
        })
    }

    fn listen(&mut self, address: Multiaddr) -> Result<ListenerId> {
        let listener = Swarm::listen_on(&mut self.swarm, address.clone())?;
        log::debug!(
            target: "robonomics-pubsub",
            "Listener for address {} created: {:?}", address, listener
        );

        Ok(listener)
    }

    fn listeners(&self) -> Vec<Multiaddr> {
        let listeners = Swarm::listeners(&self.swarm).cloned().collect();
        log::debug!(target: "robonomics-pubsub", "Listeners: {:?}", listeners);

        listeners
    }

    fn connect(&mut self, address: Multiaddr) -> Result<()> {
        Swarm::dial(&mut self.swarm, address.clone())?;
        log::debug!(target: "robonomics-pubsub", "Connected to {}", address);

        Ok(())
    }

    fn subscribe(
        &mut self,
        topic_name: String,
        inbox: mpsc::UnboundedSender<Message>,
    ) -> Result<bool> {
        let topic = Topic::new(topic_name.clone());
        self.swarm.behaviour_mut().pubsub.subscribe(&topic)?;
        self.inbox.insert(topic.hash(), inbox);
        log::debug!(target: "robonomics-pubsub", "Subscribed to {}", topic_name);

        Ok(true)
    }

    fn unsubscribe(&mut self, topic_name: String) -> Result<bool> {
        let topic = Topic::new(topic_name.clone());
        self.swarm.behaviour_mut().pubsub.unsubscribe(&topic)?;
        self.inbox.remove(&topic.hash());
        log::debug!(target: "robonomics-pubsub", "Unsubscribed from {}", topic_name);

        Ok(true)
    }

    fn publish(&mut self, topic_name: String, message: Vec<u8>) -> Result<bool> {
        let topic = Topic::new(topic_name.clone());
        self.swarm
            .behaviour_mut()
            .pubsub
            .publish(topic.clone(), message)?;
        log::debug!(target: "robonomics-pubsub", "Publish to {}", topic_name);

        Ok(true)
    }
}

impl Future for RNetwork {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            match self.swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(swarm_event)) => match swarm_event {
                    SwarmEvent::Behaviour(OutEvent::Pubsub(GossipsubEvent::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    })) => {
                        log::debug!(
                            target: "robonomics-pubsub",
                            "Received message with id: {} from peer: {}", id, peer_id.to_base58()
                        );

                        // Dispatch handlers by topic name hash
                        if let Some(inbox) = self.inbox.get_mut(&message.topic) {
                            if let Some(sender) = &message.source {
                                let _ = inbox.unbounded_send(Message {
                                    from: sender.clone().to_bytes(),
                                    data: message.data.clone(),
                                });
                            }
                        } else {
                            log::warn!(
                                target: "robonomics-pubsub",
                                "Topic {} have no associated inbox!", message.topic
                            );
                        }
                    }
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

        loop {
            match self.from_service.poll_next_unpin(cx) {
                Poll::Ready(Some(request)) => match request {
                    ToWorkerMsg::Listen(addr, result) => {
                        println!("---------------------------- listen");
                        let _ = result.send(self.listen(addr).is_ok());
                    }
                    ToWorkerMsg::Listeners(result) => {
                        println!("---------------------------- listeners");
                        let _ = result.send(self.listeners());
                    }
                    ToWorkerMsg::Connect(addr, result) => {
                        println!("---------------------------- connect");
                        let _ = result.send(self.connect(addr).is_ok());
                    }
                    ToWorkerMsg::Subscribe(topic_name, inbox) => {
                        println!("---------------------------- subscribe");
                        let _ = self.subscribe(topic_name, inbox);
                    }
                    ToWorkerMsg::Unsubscribe(topic_name, result) => {
                        println!("---------------------------- unsubscribe");
                        let _ = result.send(self.unsubscribe(topic_name).is_ok());
                    }
                    ToWorkerMsg::Publish(topic_name, message, result) => {
                        println!("---------------------------- publish");
                        let _ = result.send(self.publish(topic_name, message).is_ok());
                    }
                },
                Poll::Ready(None) | Poll::Pending => break,
            }
        }

        Poll::Pending
    }
}

// impl PubSub for RNetwork {
//     fn peer_id(&self) -> PeerId {
//         self.service.peer_id()
//     }
//
//     fn listen(&self, address: Multiaddr) -> FutureResult<bool> {
//         self.service.listen(address)
//     }
//
//     fn listeners(&self) -> FutureResult<Vec<Multiaddr>> {
//         self.service.listeners()
//     }
//
//     fn connect(&self, address: Multiaddr) -> FutureResult<bool> {
//         self.service.connect(address)
//     }
//
//     fn subscribe<T: ToString>(&self, topic_name: &T) -> Inbox {
//         self.service.subscribe(&topic_name.to_string())
//     }
//
//     fn unsubscribe<T: ToString>(&self, topic_name: &T) -> FutureResult<bool> {
//         self.service.unsubscribe(&topic_name.to_string())
//     }
//
//     fn publish<T: ToString, M: Into<Vec<u8>>>(
//         &self,
//         topic_name: &T,
//         message: M,
//     ) -> FutureResult<bool> {
//         self.service
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
