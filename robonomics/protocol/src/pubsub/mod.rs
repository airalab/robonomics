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
///! Robonomics Publisher/Subscriber protocol implements broadcasting layer.

use libp2p::{PeerId, Multiaddr};
use libp2p::core::nodes::ListenerId;
use crate::error::Result;

/// Console line interface support.
#[cfg(feature = "cli")]
pub mod cli;

/// PubSub implementation using libp2p gossipsub.
pub mod gossipsub;

/// Robonomics Publisher/Subscriber interface.
pub trait PubSub {
    /// Returns PubSub peer ID.
    fn peer_id(&self) -> PeerId;

    /// Listen address for incoming connections.
    fn listen(&mut self, address: &Multiaddr) -> Result<ListenerId>;

    /// Returns list of addresses we're listening on.
    fn listeners(&self) -> Vec<Multiaddr>;

    /// Connect to peer and add it into swarm.
    fn connect(&mut self, address: &Multiaddr) -> Result<()>;

    /// Subscribe and set handler for topic with given name.
    ///
    /// Returns true if the subscription worked. Returns false if we were already subscribed.
    fn subscribe<T, F>(&mut self, topic_name: T, callback: F) -> bool
        where T: ToString, F: FnMut(PeerId, Vec<u8>) + 'static;

    /// Unsubscribe and remove handler for given topic name.
    ///
    /// Returns true if we were subscribed to this topic.
    fn unsubscribe<T: ToString>(&mut self, topic_name: T) -> bool;

    /// Publish message into the topic.
    fn publish<T: ToString, M: Into<Vec<u8>>>(&mut self, topic_name: T, message: M);
}
