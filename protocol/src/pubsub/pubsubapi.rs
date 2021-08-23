use crate::pubsub::{Gossipsub, Message, PubSub};
use futures::{executor, StreamExt};
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use jsonrpc_pubsub::{typed::Subscriber, SubscriptionId};
use libp2p::Multiaddr;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, thread};

#[derive(Clone)]
pub struct PubSubApi {
    pubsub: Arc<Gossipsub>,
    subscriptions: Arc<Mutex<HashMap<SubscriptionId, String>>>,
}

impl PubSubApi {
    pub fn new(
        pubsub: Arc<Gossipsub>,
        subscriptions: Arc<Mutex<HashMap<SubscriptionId, String>>>,
    ) -> Self {
        PubSubApi {
            pubsub,
            subscriptions,
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
    fn publish(&self, topic_name: String, message: String);
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
        let inbox = self.pubsub.subscribe(&topic_name);
        let mut rng = rand::thread_rng();
        let subscription_id = SubscriptionId::Number(rng.gen());
        self.subscriptions
            .lock()
            .unwrap()
            .insert(subscription_id.clone(), topic_name);

        thread::spawn(move || {
            let sink = subscriber.assign_id(subscription_id).unwrap();
            let _ = inbox.map(|m: Message| {
                let _ = sink.notify(Ok(m.to_string()));
            });
        });
    }

    fn unsubscribe(
        &self,
        _: Option<Self::Metadata>,
        subscription_id: SubscriptionId,
    ) -> Result<bool> {
        if let Some(topic_name) = self.subscriptions.lock().unwrap().get(&subscription_id) {
            self.pubsub.unsubscribe(&topic_name);
            self.subscriptions.lock().unwrap().remove(&subscription_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn publish(&self, topic_name: String, message: String) {
        executor::block_on(async {
            self.pubsub
                .publish(&topic_name, message.as_bytes().to_vec())
        })
    }
}
