///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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
//! PubSub implementation using libp2p Gossipsub.

use futures::{
    channel::{mpsc, oneshot},
    executor::block_on,
    prelude::*,
    Future,
};
use libp2p::{
    core::transport::ListenerId,
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage, MessageAuthenticity,
        MessageId, Sha256Topic as Topic, TopicHash,
    },
    kad::KademliaEvent,
    request_response::{RequestResponseEvent, RequestResponseMessage},
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};
use std::{
    collections::hash_map::{DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use crate::{
    error::{FutureResult, Result},
    network::{OutEvent, RobonomicsNetworkBehaviour},
    pubsub::discovery,
    reqres::Response,
};

enum ToWorkerMsg {
    Listen(Multiaddr, oneshot::Sender<bool>),
    Connect(Multiaddr, oneshot::Sender<bool>),
    Listeners(oneshot::Sender<Vec<Multiaddr>>),
    Subscribe(String, mpsc::UnboundedSender<super::Message>),
    Unsubscribe(String, oneshot::Sender<bool>),
    Publish(String, Vec<u8>, oneshot::Sender<bool>),
}

struct PubSubWorker {
    swarm: Swarm<RobonomicsNetworkBehaviour>,
    inbox: HashMap<TopicHash, mpsc::UnboundedSender<super::Message>>,
    from_service: mpsc::UnboundedReceiver<ToWorkerMsg>,
    service: Arc<PubSub>,
}

impl PubSubWorker {
    /// Create new PubSub Worker instance
    pub fn new(
        heartbeat_interval: Duration,
        bootnodes: Vec<String>,
        disable_mdns: bool,
        disable_kad: bool,
    ) -> Result<Self> {
        // XXX: temporary random local id.
        let local_key = crate::id::random();
        let peer_id = PeerId::from(local_key.public());
        log::info!("Robonomics peer id: {:?}", peer_id);

        // Set up an encrypted WebSocket compatible Transport over the Mplex and Yamux protocols
        let transport = block_on(libp2p::development_transport(local_key.clone()))?;

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
            .build()
            .expect("Valid gossipsub config");

        // Build a gossipsub network behaviour
        let pubsub = Gossipsub::new(MessageAuthenticity::Signed(local_key), gossipsub_config)
            .expect("Correct configuration");

        // Build a combined network behaviour
        let behaviour =
            RobonomicsNetworkBehaviour::new(pubsub, disable_mdns, disable_kad, peer_id)?;

        // Create a Swarm to manage peers and events
        let mut swarm = Swarm::new(transport, behaviour, peer_id.clone());

        // Reach out to another nodes if specified
        discovery::add_explicit_peers(&mut swarm, bootnodes, disable_kad);

        // Create worker communication channel
        let (to_worker, from_service) = mpsc::unbounded();

        // Create PubSub service
        let service = Arc::new(PubSub { to_worker, peer_id });

        // Create worker instance with empty subscribers
        Ok(PubSubWorker {
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
        inbox: mpsc::UnboundedSender<super::Message>,
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

impl Future for PubSubWorker {
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
                                let _ = inbox.unbounded_send(super::Message {
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
                        ..
                    })) => {
                        log::info!("Received kademlia peer: {:?}", peer);
                        if let Some(kademlia) = self.swarm.behaviour_mut().kademlia.as_mut() {
                            if let Err(e) = kademlia.bootstrap() {
                                log::debug!("Bootstrap error: {:?}", e);
                            };
                        }
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
                        let _ = result.send(self.listen(addr).is_ok());
                    }
                    ToWorkerMsg::Listeners(result) => {
                        let _ = result.send(self.listeners());
                    }
                    ToWorkerMsg::Connect(addr, result) => {
                        let _ = result.send(self.connect(addr).is_ok());
                    }
                    ToWorkerMsg::Subscribe(topic_name, inbox) => {
                        let _ = self.subscribe(topic_name, inbox);
                    }
                    ToWorkerMsg::Unsubscribe(topic_name, result) => {
                        let _ = result.send(self.unsubscribe(topic_name).is_ok());
                    }
                    ToWorkerMsg::Publish(topic_name, message, result) => {
                        let _ = result.send(self.publish(topic_name, message).is_ok());
                    }
                },
                Poll::Ready(None) | Poll::Pending => break,
            }
        }
        Poll::Pending
    }
}

/// LibP2P Gossipsub based publisher/subscriber service.
pub struct PubSub {
    peer_id: PeerId,
    to_worker: mpsc::UnboundedSender<ToWorkerMsg>,
}

impl PubSub {
    /// Create Gossipsub based PubSub service and worker.
    pub fn new(
        heartbeat_interval: Duration,
        bootnodes: Vec<String>,
        disable_mdns: bool,
        disable_kad: bool,
    ) -> Result<(Arc<Self>, impl Future<Output = ()>)> {
        PubSubWorker::new(heartbeat_interval, bootnodes, disable_mdns, disable_kad)
            .map(|worker| (worker.service.clone(), worker))
    }
}

impl super::PubSub for PubSub {
    fn peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }

    fn listen(&self, address: Multiaddr) -> FutureResult<bool> {
        let (sender, receiver) = oneshot::channel();
        let _ = self
            .to_worker
            .unbounded_send(ToWorkerMsg::Listen(address, sender));
        receiver.boxed()
    }

    fn listeners(&self) -> FutureResult<Vec<Multiaddr>> {
        let (sender, receiver) = oneshot::channel();
        let _ = self
            .to_worker
            .unbounded_send(ToWorkerMsg::Listeners(sender));
        receiver.boxed()
    }

    fn connect(&self, address: Multiaddr) -> FutureResult<bool> {
        let (sender, receiver) = oneshot::channel();
        let _ = self
            .to_worker
            .unbounded_send(ToWorkerMsg::Connect(address, sender));
        receiver.boxed()
    }

    fn subscribe<T: ToString>(&self, topic_name: &T) -> super::Inbox {
        let (sender, receiver) = mpsc::unbounded();
        let _ = self
            .to_worker
            .unbounded_send(ToWorkerMsg::Subscribe(topic_name.to_string(), sender));

        receiver
    }

    fn unsubscribe<T: ToString>(&self, topic_name: &T) -> FutureResult<bool> {
        let (sender, receiver) = oneshot::channel();
        let _ = self
            .to_worker
            .unbounded_send(ToWorkerMsg::Unsubscribe(topic_name.to_string(), sender));
        receiver.boxed()
    }

    fn publish<T: ToString, M: Into<Vec<u8>>>(
        &self,
        topic_name: &T,
        message: M,
    ) -> FutureResult<bool> {
        let (sender, receiver) = oneshot::channel();
        let _ = self.to_worker.unbounded_send(ToWorkerMsg::Publish(
            topic_name.to_string(),
            message.into(),
            sender,
        ));
        receiver.boxed()
    }
}
