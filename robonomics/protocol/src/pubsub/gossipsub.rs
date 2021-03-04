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
//!
//! This code is fully asynchronous and threadsafe.
//! It implemented using libp2p Gossipsub for effective message delivery.
//!

use futures::{
    channel::{mpsc, oneshot},
    prelude::*,
    Future,
};
use libp2p::core::connection::ListenerId;
use libp2p::gossipsub::{
    protocol::MessageId, Gossipsub, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage,
    MessageAuthenticity, Topic, TopicHash,
};
use libp2p::{Multiaddr, PeerId, Swarm};
use std::{
    collections::hash_map::{DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    ops::DerefMut,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use crate::error::{FutureResult, Result};

enum ToWorkerMsg {
    Listen(Multiaddr, oneshot::Sender<bool>),
    Connect(Multiaddr, oneshot::Sender<bool>),
    Listeners(oneshot::Sender<Vec<Multiaddr>>),
    Subscribe(String, mpsc::UnboundedSender<super::Message>),
    Unsubscribe(String, oneshot::Sender<bool>),
    Publish(String, Vec<u8>),
}

struct PubSubWorker {
    swarm: Swarm<Gossipsub>,
    inbox: HashMap<TopicHash, mpsc::UnboundedSender<super::Message>>,
    from_service: mpsc::UnboundedReceiver<ToWorkerMsg>,
    service: Arc<PubSub>,
}

impl PubSubWorker {
    /// Create new PubSub Worker instance
    pub fn new(heartbeat_interval: Duration) -> Result<Self> {
        // XXX: temporary random local id.
        let local_key = crate::id::random();
        let peer_id = PeerId::from(local_key.public());

        // Set up an encrypted WebSocket compatible Transport over the Mplex and Yamux protocols
        let transport = libp2p::build_tcp_ws_noise_mplex_yamux(local_key.clone())?;

        // Set custom gossipsub
        let gossipsub_config = GossipsubConfigBuilder::new()
            .heartbeat_interval(heartbeat_interval)
            .message_id_fn(|message: &GossipsubMessage| {
                // To content-address message,
                // we can take the hash of message and use it as an ID.
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                MessageId::from(s.finish().to_string())
            })
            .build();

        // Build a gossipsub network behaviour
        let gossipsub = Gossipsub::new(MessageAuthenticity::Signed(local_key), gossipsub_config);

        // Create a Swarm to manage peers and events
        let swarm = Swarm::new(transport, gossipsub, peer_id.clone());

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

    fn connect(&mut self, address: Multiaddr) -> bool {
        log::debug!(target: "robonomics-pubsub", "Connecting to {}", address);

        Swarm::dial_addr(&mut self.swarm, address).is_ok()
    }

    fn subscribe(
        &mut self,
        topic_name: String,
        inbox: mpsc::UnboundedSender<super::Message>,
    ) -> bool {
        let topic = Topic::new(topic_name.clone());
        let subscribed = self.swarm.deref_mut().subscribe(topic.clone());
        if subscribed {
            log::debug!(target: "robonomics-pubsub", "Subscribed to {}", topic_name);
            self.inbox.insert(topic.no_hash(), inbox);
        } else {
            log::warn!(target: "robonomics-pubsub",
                       "Double subscription to {}, ignore", topic_name);
        }
        subscribed
    }

    fn unsubscribe(&mut self, topic_name: String) -> bool {
        let topic = Topic::new(topic_name.clone());
        let unsubscribed = self.swarm.deref_mut().unsubscribe(topic.clone());
        if unsubscribed {
            log::debug!(target: "robonomics-pubsub", "Unsubscribed from {}", topic_name);
            self.inbox.remove(&topic.sha256_hash());
        } else {
            log::warn!(target: "robonomics-pubsub",
                       "Unable to unsubscribe from {}, ignore", topic_name);
        }
        unsubscribed
    }

    fn publish(&mut self, topic_name: String, message: Vec<u8>) {
        log::debug!(target: "robonomics-pubsub", "Publish to {}", topic_name);

        let topic = Topic::new(topic_name);
        let _ = self.swarm.deref_mut().publish(&topic, message);
    }
}

impl Future for PubSubWorker {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            match self.swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(gossip_event)) => match gossip_event {
                    GossipsubEvent::Message(peer_id, id, message) => {
                        log::debug!(
                            target: "robonomics-pubsub",
                            "Received message with id: {} from peer: {}", id, peer_id.to_base58()
                        );

                        // Dispatch handlers by topic name hash
                        for topic in &message.topics {
                            if let Some(inbox) = self.inbox.get_mut(topic) {
                                if let Some(sender) = &message.source {
                                    let _ = inbox.unbounded_send(super::Message {
                                        from: sender.clone(),
                                        data: message.data.clone(),
                                    });
                                }
                            } else {
                                log::warn!(
                                    target: "robonomics-pubsub",
                                    "Topic {} have no associated inbox!", topic
                                );
                            }
                        }
                    }
                    _ => {}
                },
                Poll::Ready(None) | Poll::Pending => break,
            }
        }

        loop {
            match self.from_service.poll_next_unpin(cx) {
                Poll::Ready(Some(request)) => match request {
                    ToWorkerMsg::Listen(addr, result) => {
                        let _ = result.send(self.listen(addr).is_ok());
                    }
                    ToWorkerMsg::Connect(addr, result) => {
                        let _ = result.send(self.connect(addr));
                    }
                    ToWorkerMsg::Listeners(result) => {
                        let _ = result.send(self.listeners());
                    }
                    ToWorkerMsg::Subscribe(topic_name, inbox) => {
                        let _ = self.subscribe(topic_name, inbox);
                    }
                    ToWorkerMsg::Unsubscribe(topic_name, result) => {
                        let _ = result.send(self.unsubscribe(topic_name));
                    }
                    ToWorkerMsg::Publish(topic_name, message) => {
                        self.publish(topic_name, message);
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
    ) -> Result<(Arc<Self>, impl Future<Output = Result<()>>)> {
        PubSubWorker::new(heartbeat_interval).map(|worker| (worker.service.clone(), worker))
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
        receiver.boxed()
    }

    fn unsubscribe<T: ToString>(&self, topic_name: &T) -> FutureResult<bool> {
        let (sender, receiver) = oneshot::channel();
        let _ = self
            .to_worker
            .unbounded_send(ToWorkerMsg::Unsubscribe(topic_name.to_string(), sender));
        receiver.boxed()
    }

    fn publish<T: ToString, M: Into<Vec<u8>>>(&self, topic_name: &T, message: M) {
        let _ = self
            .to_worker
            .unbounded_send(ToWorkerMsg::Publish(topic_name.to_string(), message.into()));
    }
}
