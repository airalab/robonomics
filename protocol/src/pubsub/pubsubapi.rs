use crate::pubsub::{Gossipsub, Message, PubSub};
use futures::StreamExt;
use jsonrpc_core::{futures::Sink, Result};
use jsonrpc_derive::rpc;
use jsonrpc_pubsub::{
    // manager::{IdProvider, SubscriptionManager},
    typed::Subscriber,
    SubscriptionId,
};
// use log::warn;
// use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
// use rustc_hex::ToHex;
use std::{sync::Arc, sync::Mutex, thread};

// #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
// pub struct HexEncodedIdProvider {
//     len: usize,
// }
//
// impl Default for HexEncodedIdProvider {
//     fn default() -> Self {
//         Self { len: 16 }
//     }
// }
//
// impl IdProvider for HexEncodedIdProvider {
//     type Id = String;
//     fn next_id(&self) -> Self::Id {
//         let id: String = rand::thread_rng()
//             .sample_iter(&Alphanumeric)
//             .take(self.len)
//             .map(char::from)
//             .collect();
//         let out: String = id.as_bytes().to_hex();
//         format!("0x{}", out)
//     }
// }

use std::collections::HashMap;

#[derive(Clone)]
pub struct PubSubApi {
    pubsub: Arc<Gossipsub>,
    // subscriptions: SubscriptionManager<HexEncodedIdProvider>,
    subscriptions: Arc<Mutex<HashMap<SubscriptionId, String>>>,
}

impl PubSubApi {
    pub fn new(
        pubsub: Arc<Gossipsub>,
        // subscriptions: SubscriptionManager<HexEncodedIdProvider>,
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
}

use jsonrpc_core::futures::Future;
impl PubSubT for PubSubApi {
    type Metadata = sc_rpc_api::Metadata;

    fn subscribe(&self, _: Self::Metadata, subscriber: Subscriber<String>, topic_name: String) {
        let inbox = self.pubsub.subscribe(&topic_name);

        // ???
        // self.subscriptions.add(subscriber, |sink| {
        //     sink.send_all(inbox.into())
        //         .sink_map_err(|e| warn!("Error sending notifications: {:?}", e))
        //         .map(|_| ())
        // });

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
        match self.subscriptions.lock().unwrap().get(&subscription_id) {
            Some(topic_name) => {
                self.pubsub.unsubscribe(&topic_name);
                self.subscriptions.lock().unwrap().remove(&subscription_id);
                Ok(true)
            }
            _ => Ok(false),
        }
        // Ok(self.subscriptions.cancel(subscription_id))
    }
}
