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
//! Robonomics Network broadcasting layer.

use futures::{
    channel::{mpsc, oneshot},
    prelude::*,
};
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use std::fmt;

use crate::error::FutureResult;

/// Robonomics PubSub message.
#[derive(PartialEq, Eq, Clone, Debug, Serialize)]
pub struct Message {
    pub from: Vec<u8>,
    pub data: Vec<u8>,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PeerId: {:?}, Data: {:?}", self.from, self.data)
    }
}

/// Stream of incoming messages.
pub type Inbox = mpsc::UnboundedReceiver<Message>;

pub enum ToWorkerMsg {
    Listen(Multiaddr, oneshot::Sender<bool>),
    Connect(Multiaddr, oneshot::Sender<bool>),
    Listeners(oneshot::Sender<Vec<Multiaddr>>),
    Subscribe(String, mpsc::UnboundedSender<Message>),
    Unsubscribe(String, oneshot::Sender<bool>),
    Publish(String, Vec<u8>, oneshot::Sender<bool>),
}

/// Robonomics Publisher/Subscriber.
pub trait PubSub {
    /// Returns local peer ID.
    fn peer_id(&self) -> PeerId;

    /// Listen address for incoming connections.
    ///
    /// Returns true when successful bind and false in case of error.
    fn listen(&self, address: Multiaddr) -> FutureResult<bool>;

    /// Returns a list of node addresses.
    fn listeners(&self) -> FutureResult<Vec<Multiaddr>>;

    /// Connect to peer and add it into swarm.
    ///
    /// Returns true when connected and false in case of error.
    fn connect(&self, address: Multiaddr) -> FutureResult<bool>;

    /// Subscribe for a topic with given name.
    ///
    /// Returns stream of incoming messages.
    fn subscribe<T: ToString>(&self, topic_name: &T) -> Inbox;

    /// Unsubscribe for incoming messages from topic.
    ///
    /// Returns true when success.
    fn unsubscribe<T: ToString>(&self, topic_name: &T) -> FutureResult<bool>;

    /// Publish message into the topic by name.
    fn publish<T: ToString, M: Into<Vec<u8>>>(
        &self,
        topic_name: &T,
        message: M,
    ) -> FutureResult<bool>;
}

/// LibP2P Gossipsub based publisher/subscriber service.
pub struct Pubsub {
    peer_id: PeerId,
    to_worker: mpsc::UnboundedSender<ToWorkerMsg>,
}

impl Pubsub {
    pub fn new(peer_id: PeerId, to_worker: mpsc::UnboundedSender<ToWorkerMsg>) -> Self {
        Self { peer_id, to_worker }
    }
}

impl PubSub for Pubsub {
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

    fn subscribe<T: ToString>(&self, topic_name: &T) -> Inbox {
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
