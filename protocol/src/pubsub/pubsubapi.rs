use crate::pubsub::{gossipsub::ToWorkerMsg, Gossipsub, Inbox};

use futures::{channel::mpsc, prelude::*};
use jsonrpc_derive::rpc;
use std::sync::Arc;

type RpcResult<T> = Result<T, jsonrpc_core::error::Error>;

#[derive(Clone)]
pub struct PubSubApi {
    pub pubsub: Arc<Gossipsub>,
}

impl PubSubApi {
    pub fn new(pubsub: Arc<Gossipsub>) -> Self {
        PubSubApi { pubsub }
    }
}

#[rpc]
pub trait PubSubT {
    /// Subscribe for a topic with given name.
    ///
    /// Returns stream of incoming messages.
    #[rpc(name = "pubsub_subscribe")]
    fn subscribe(&self, topic_name: String) -> RpcResult<Inbox>;
}

impl PubSubT for PubSubApi {
    fn subscribe(&self, topic_name: String) -> RpcResult<Inbox> {
        let (sender, receiver) = mpsc::unbounded();
        let _ = self
            .pubsub
            .to_worker
            .unbounded_send(ToWorkerMsg::Subscribe(topic_name.to_string(), sender));
        Ok(receiver.boxed())
    }
}
