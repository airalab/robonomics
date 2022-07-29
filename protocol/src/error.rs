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
//! Errors that can occur during the protocol operations.

use futures::channel::oneshot;
use futures::Future;
use libp2p::core::transport::TransportError;
use libp2p::{gossipsub, swarm};
use std::pin::Pin;

/// Protocol Result typedef.
pub type Result<T> = std::result::Result<T, Error>;

/// Oneshot channel result.
type OneshotResult<T> = std::result::Result<T, oneshot::Canceled>;

/// Async version of protocol Result typedef.
pub type FutureResult<T> = Pin<Box<dyn Future<Output = OneshotResult<T>> + Send>>;

/// Robonomics protocol errors.
#[derive(Debug, derive_more::Display, derive_more::From)]
pub enum Error {
    /// IO error.
    Io(std::io::Error),
    /// Libp2p transport error.
    Transport(TransportError<std::io::Error>),
    /// Libp2p connection limit error.
    //ConnectionLimit(ConnectionLimit),
    /// Codec error.
    Codec(bincode::Error),
    /// Unable to decode address.
    Ss58CodecError,
    /// Libp2p swarm dial error.
    DialError,
    /// Libp2p gossipsub subscribe error.
    SubscriptionError,
    /// Libp2p gossipsub publish error.
    PublishError,
    /// Rpc error.
    RpcError,
    /// Other error.
    Other(String),
}

impl<'a> From<&'a str> for Error {
    fn from(s: &'a str) -> Self {
        Error::Other(s.into())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(ref err) => Some(err),
            Error::Transport(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<swarm::DialError> for Error {
    fn from(_: swarm::DialError) -> Self {
        Error::DialError
    }
}

impl From<gossipsub::error::SubscriptionError> for Error {
    fn from(_: gossipsub::error::SubscriptionError) -> Self {
        Error::SubscriptionError
    }
}

impl From<gossipsub::error::PublishError> for Error {
    fn from(_: gossipsub::error::PublishError) -> Self {
        Error::PublishError
    }
}
