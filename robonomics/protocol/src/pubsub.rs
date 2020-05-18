///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life>
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

use crate::error::FutureResult;
use futures::Stream;
use std::pin::Pin;

pub use libp2p::{Multiaddr, PeerId};

pub mod discovery;
pub mod gossipsub;

pub use gossipsub::PubSub as Gossipsub;

/// Robonomics PubSub message.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Message {
    pub from: PeerId,
    pub data: Vec<u8>,
}

/// Stream of incoming messages.
pub type Inbox = Pin<Box<dyn Stream<Item = Message> + Send>>;

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
    fn publish<T: ToString, M: Into<Vec<u8>>>(&self, topic_name: &T, message: M);
}
