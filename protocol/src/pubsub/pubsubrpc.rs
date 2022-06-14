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
//!
//! A basic pubsub client demonstrating libp2p and the gossipsub protocol.
//!
//! Using two terminal windows, start two instances.
//! ```sh
//! target/debug/robonomics --dev --tmp -l rpc=trace
//! target/debug/robonomics --dev --tmp --ws-port 9991 -l rpc=trace
//! ```
//!
//! Then using two terminal windows, start two clients.
//! One of them will send messages, the other one will catch them and print.
//! You can of course open more terminal windows and add more participants.
//!
//! Pubsub subscribe:
//!
//! ```{python}
//! import time
//! import robonomicsinterface as RI
//! from robonomicsinterface import PubSub
//!
//! def subscription_handler(obj, update_nr, subscription_id):
//!     print(obj['params']['result'])
//!     if update_nr >= 2:
//!         return 0
//!
//! interface = RI.RobonomicsInterface(remote_ws="ws://127.0.0.1:9944")
//! pubsub = PubSub(interface)
//!
//! print(pubsub.listen("/ip4/127.0.0.1/tcp/44440"))
//! time.sleep(2)
//! print(pubsub.connect("/ip4/127.0.0.1/tcp/44441"))
//! print(pubsub.subscribe("topic_name", result_handler=subscription_handler))
//! ```
//!
//! Pubsub publish:
//!
//! ```{python}
//! import time
//! import robonomicsinterface as RI
//! from robonomicsinterface import PubSub
//!
//! interface = RI.RobonomicsInterface(remote_ws="ws://127.0.0.1:9991")
//! pubsub = PubSub(interface)
//!
//! print(pubsub.listen("/ip4/127.0.0.1/tcp/44441"))
//! time.sleep(2)
//! print(pubsub.connect("/ip4/127.0.0.1/tcp/44440"))
//! for i in range(10):
//!     time.sleep(2)
//!     print("publish:", pubsub.publish("topic_name", "message_" + str(time.time())))
//! ```

use crate::pubsub::{Gossipsub, PubSub};
use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorCode, ErrorObject},
};
use libp2p::Multiaddr;
use rand::Rng;
use std::{
    collections::HashMap,
    str,
    sync::{Arc, Mutex},
    thread,
};

// pub struct PubSubRpc<C> {
pub struct PubSubRpc {
    pubsub: Arc<Gossipsub>,
    // subscriptions: Arc<Mutex<HashMap<SubscriptionId, Sink<String>>>>,
    // topics: Arc<Mutex<HashMap<SubscriptionId, String>>>,
    // client: Arc<C>,
    // _marker: std::marker::PhantomData<P>,
}

// impl<C> PubSubRpc<C> {
impl PubSubRpc {
    pub fn new(pubsub: Arc<Gossipsub>) -> Self {
        Self {
            pubsub,
            // client,
            // subscriptions: Arc::new(Mutex::new(HashMap::new())),
            // topics: Arc::new(Mutex::new(HashMap::new())),
            // _marker: Default::default(),
        }
    }
}

#[rpc(client, server)]
pub trait PubSubRpc {
    /// Returns local peer ID.
    #[method(name = "pubsub_peer")]
    fn peer_id(&self) -> RpcResult<String>;

    /// Listen address for incoming connections.
    #[method(name = "pubsub_listen")]
    async fn listen(&self, address: Multiaddr) -> RpcResult<bool>;

    /// Returns a list of node addresses.
    #[method(name = "pubsub_listeners")]
    async fn listeners(&self) -> RpcResult<Vec<Multiaddr>>;

    /// Connect to peer and add it into swarm.
    #[method(name = "pubsub_connect")]
    async fn connect(&self, address: Multiaddr) -> RpcResult<bool>;

    // /// Subscribe for a topic with given name.
    // #[pubsub(
    //     subscription = "robonomics_subscription",
    //     subscribe,
    //     name = "pubsub_subscribe"
    // )]
    // fn subscribe(&self, _: Self::Metadata, _: Subscriber<String>, topic_name: String);
    //
    // /// Unsubscribe for incoming messages from topic.
    // #[pubsub(
    //     subscription = "robonomics_subscription",
    //     unsubscribe,
    //     name = "pubsub_unsubscribe"
    // )]
    // fn unsubscribe(&self, _: Option<Self::Metadata>, _: SubscriptionId) -> Result<bool>;
    //
    // /// Publish message into the topic by name.
    // #[rpc(name = "pubsub_publish")]
    // fn publish(&self, topic_name: String, message: String) -> Result<bool>;
}

#[async_trait]
impl PubSubRpcServer for PubSubRpc {
    fn peer_id(&self) -> RpcResult<String> {
        Ok(self.pubsub.peer_id().to_string())
    }

    async fn listen(&self, address: Multiaddr) -> RpcResult<bool> {
        self.pubsub.listen(address).await.or(Ok(false))
        // self.pubsub
        //     .listen(address)
        //     .await
        //     .map_err(|_| JsonRpseeError::Custom("Internal error!".to_string()))
    }

    async fn listeners(&self) -> RpcResult<Vec<Multiaddr>> {
        self.pubsub.listeners().await.or(Ok(vec![]))
    }

    async fn connect(&self, address: Multiaddr) -> RpcResult<bool> {
        self.pubsub.connect(address).await.or(Ok(false))
    }
}

// #[rpc]
// pub trait PubSubT {
//     type Metadata;
//
//     /// Returns local peer ID.
//     #[rpc(name = "pubsub_peer")]
//     fn peer_id(&self) -> Result<String>;
//
//     /// Listen address for incoming connections.
//     #[rpc(name = "pubsub_listen")]
//     fn listen(&self, address: Multiaddr) -> Result<bool>;
//
//     /// Returns a list of node addresses.
//     #[rpc(name = "pubsub_listeners")]
//     fn listeners(&self) -> Result<Vec<Multiaddr>>;
//
//     /// Connect to peer and add it into swarm.
//     #[rpc(name = "pubsub_connect")]
//     fn connect(&self, address: Multiaddr) -> Result<bool>;
//
//     /// Subscribe for a topic with given name.
//     #[pubsub(
//         subscription = "robonomics_subscription",
//         subscribe,
//         name = "pubsub_subscribe"
//     )]
//     fn subscribe(&self, _: Self::Metadata, _: Subscriber<String>, topic_name: String);
//
//     /// Unsubscribe for incoming messages from topic.
//     #[pubsub(
//         subscription = "robonomics_subscription",
//         unsubscribe,
//         name = "pubsub_unsubscribe"
//     )]
//     fn unsubscribe(&self, _: Option<Self::Metadata>, _: SubscriptionId) -> Result<bool>;
//
//     /// Publish message into the topic by name.
//     #[rpc(name = "pubsub_publish")]
//     fn publish(&self, topic_name: String, message: String) -> Result<bool>;
// }
//
// impl PubSubT for PubSubApi {
//     type Metadata = sp_api::Metadata;
//
//     fn peer_id(&self) -> Result<String> {
//         Ok(self.pubsub.peer_id().to_string())
//     }
//
//     fn listen(&self, address: Multiaddr) -> Result<bool> {
//         executor::block_on(async { self.pubsub.listen(address).await }).or(Ok(false))
//     }
//
//     fn listeners(&self) -> Result<Vec<Multiaddr>> {
//         executor::block_on(async { self.pubsub.listeners().await }).or(Ok(vec![]))
//     }
//
//     fn connect(&self, address: Multiaddr) -> Result<bool> {
//         executor::block_on(async { self.pubsub.connect(address).await }).or(Ok(false))
//     }
//
//     fn subscribe(&self, _meta: Self::Metadata, subscriber: Subscriber<String>, topic_name: String) {
//         let mut inbox = self.pubsub.subscribe(&topic_name.clone());
//         let mut rng = rand::thread_rng();
//         let subscription_id = SubscriptionId::Number(rng.gen());
//         let sink = subscriber.assign_id(subscription_id.clone()).unwrap();
//         self.subscriptions
//             .lock()
//             .unwrap()
//             .insert(subscription_id.clone(), sink.clone());
//         self.topics
//             .lock()
//             .unwrap()
//             .insert(subscription_id, topic_name);
//
//         thread::spawn(move || loop {
//             match inbox.try_next() {
//                 // Message is fetched.
//                 Ok(Some(message)) => {
//                     if let Ok(message) = str::from_utf8(&message.data) {
//                         let _ = sink.notify(Ok(message.to_string()));
//                     } else {
//                         continue;
//                     }
//                 }
//                 // Channel is closed and no messages left in the queue.
//                 Ok(None) => break,
//
//                 // There are no messages available, but channel is not yet closed.
//                 Err(_) => {}
//             }
//         });
//     }
//
//     fn unsubscribe(&self, _meta: Option<Self::Metadata>, sid: SubscriptionId) -> Result<bool> {
//         if let Some(topic_name) = self.topics.lock().unwrap().remove(&sid) {
//             let _ = self.subscriptions.lock().unwrap().remove(&sid);
//             let _ = executor::block_on(async { self.pubsub.unsubscribe(&topic_name).await });
//         };
//
//         Ok(true)
//     }
//
//     fn publish(&self, topic_name: String, message: String) -> Result<bool> {
//         executor::block_on(async {
//             self.pubsub
//                 .publish(&topic_name, message.as_bytes().to_vec())
//                 .await
//         })
//         .or(Ok(false))
//     }
// }
