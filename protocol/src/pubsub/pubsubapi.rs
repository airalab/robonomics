use futures::{
    channel::{mpsc, oneshot},
    // executor::block_on,
    prelude::*,
    Future,
};
use jsonrpc_derive::rpc;
use std::sync::{Arc, Mutex};

use super::Gossipsub;
use crate::pubsub::{gossipsub::ToWorkerMsg, Inbox};

type RpcResult<T> = Result<T, jsonrpc_core::error::Error>;

#[rpc]
pub trait PubSubT {
    #[rpc(name = "pubsub_seven")]
    fn silly_7(&self) -> RpcResult<u64>;

    /// Subscribe for a topic with given name.
    ///
    /// Returns stream of incoming messages.
    #[rpc(name = "pubsub_subscribe")]
    fn subscribe(&self, topic_name: String) -> RpcResult<Inbox>;
}

#[derive(Clone)]
pub struct PubSubApi {
    // pub swarm: Arc<Mutex<Swarm<Gossipsub>>>,
    pub worker: Arc<Gossipsub>,
}

impl PubSubApi {
    pub fn new(worker: Arc<Gossipsub>) -> Self {
        PubSubApi { worker }
    }
}

impl PubSubT for PubSubApi {
    fn silly_7(&self) -> RpcResult<u64> {
        Ok(7)
    }

    fn subscribe(&self, topic_name: String) -> RpcResult<super::Inbox> {
        let (sender, receiver) = mpsc::unbounded();
        let _ = self
            .worker
            .to_worker
            .unbounded_send(ToWorkerMsg::Subscribe(topic_name.to_string(), sender));
        Ok(receiver.boxed())
    }
}
