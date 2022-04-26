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
//! Robonomics Publisher/Subscriber protocol.

use crate::pubsub::{Gossipsub, PubSub};
use futures::executor;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use jsonrpc_pubsub::{
    typed::{Sink, Subscriber},
    SubscriptionId,
};
use libp2p::Multiaddr;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, str, thread};

#[derive(Clone)]
pub struct PubSubApi {
    pubsub: Arc<Gossipsub>,
    subscriptions: Arc<Mutex<HashMap<SubscriptionId, Sink<String>>>>,
    topics: Arc<Mutex<HashMap<SubscriptionId, String>>>,
}

impl PubSubApi {
    pub fn new(pubsub: Arc<Gossipsub>) -> Self {
        PubSubApi {
            pubsub,
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            topics: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[rpc(server)]
pub trait PubSubT {
    type Metadata;

    /// Returns local peer ID.
    #[rpc(name = "pubsub_peer")]
    fn peer_id(&self) -> Result<String>;

    /// Listen address for incoming connections.
    #[rpc(name = "pubsub_listen")]
    fn listen(&self, address: Multiaddr) -> Result<bool>;

    /// Returns a list of node addresses.
    #[rpc(name = "pubsub_listeners")]
    fn listeners(&self) -> Result<Vec<Multiaddr>>;

    /// Connect to peer and add it into swarm.
    #[rpc(name = "pubsub_connect")]
    fn connect(&self, address: Multiaddr) -> Result<bool>;

    /// Subscribe for a topic with given name.
    #[pubsub(
        subscription = "robonomics_subscription",
        subscribe,
        name = "pubsub_subscribe"
    )]
    fn subscribe(&self, _: Self::Metadata, _: Subscriber<String>, topic_name: String);

    /// Unsubscribe for incoming messages from topic.
    #[pubsub(
        subscription = "robonomics_subscription",
        unsubscribe,
        name = "pubsub_unsubscribe"
    )]
    fn unsubscribe(&self, _: Option<Self::Metadata>, _: SubscriptionId) -> Result<bool>;

    /// Publish message into the topic by name.
    #[rpc(name = "pubsub_publish")]
    fn publish(&self, topic_name: String, message: String) -> Result<bool>;
}

impl PubSubT for PubSubApi {
    type Metadata = sc_rpc_api::Metadata;

    fn peer_id(&self) -> Result<String> {
        Ok(self.pubsub.peer_id().to_string())
    }

    fn listen(&self, address: Multiaddr) -> Result<bool> {
        executor::block_on(async { self.pubsub.listen(address).await }).or(Ok(false))
    }

    fn listeners(&self) -> Result<Vec<Multiaddr>> {
        executor::block_on(async { self.pubsub.listeners().await }).or(Ok(vec![]))
    }

    fn connect(&self, address: Multiaddr) -> Result<bool> {
        executor::block_on(async { self.pubsub.connect(address).await }).or(Ok(false))
    }

    fn subscribe(&self, _: Self::Metadata, subscriber: Subscriber<String>, topic_name: String) {
        let mut inbox = self.pubsub.subscribe(&topic_name.clone());
        let mut rng = rand::thread_rng();
        let subscription_id = SubscriptionId::Number(rng.gen());
        let sink = subscriber.assign_id(subscription_id.clone()).unwrap();
        self.subscriptions
            .lock()
            .unwrap()
            .insert(subscription_id.clone(), sink.clone());
        self.topics
            .lock()
            .unwrap()
            .insert(subscription_id, topic_name);

        thread::spawn(move || loop {
            match inbox.try_next() {
                // Message is fetched.
                Ok(Some(message)) => {
                    if let Ok(message) = str::from_utf8(&message.data) {
                        let _ = sink.notify(Ok(message.to_string()));
                    } else {
                        continue;
                    }
                }
                // Channel is closed and no messages left in the queue.
                Ok(None) => break,
                // There are no messages available, but channel is not yet closed.
                Err(_) => {}
            }
        });
    }

    fn unsubscribe(
        &self,
        _: Option<Self::Metadata>,
        subscription_id: SubscriptionId,
    ) -> Result<bool> {
        if let Some(topic_name) = self.topics.lock().unwrap().remove(&subscription_id) {
            let _ = self.subscriptions.lock().unwrap().remove(&subscription_id);
            let _ = executor::block_on(async { self.pubsub.unsubscribe(&topic_name).await });
        };

        Ok(true)
    }

    fn publish(&self, topic_name: String, message: String) -> Result<bool> {
        executor::block_on(async {
            self.pubsub
                .publish(&topic_name, message.as_bytes().to_vec())
                .await
        })
        .or(Ok(false))
    }
}
