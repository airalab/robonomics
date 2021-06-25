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
//! Simple Req-Resp Protocol

use async_trait::async_trait;
use futures::{AsyncRead, AsyncWrite, FutureExt};
use libp2p::core::upgrade::{read_one, write_one};
use libp2p::core::ProtocolName;
use libp2p::request_response::RequestResponseCodec;
use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Req {
    Ping (),
    Get (Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resp {
    Pong (),
    Data (Vec<u8>),
}
impl ProtocolName for ReqRespProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/robonomics/1".as_bytes()
    }
}

#[derive(Clone)]
pub struct ReqRespCodec();

#[derive(Debug, Clone)]
pub struct ReqRespProtocol();

#[async_trait]
impl RequestResponseCodec for ReqRespCodec {
    type Protocol = ReqRespProtocol;
    type Request = Req;
    type Response = Resp;

    async fn read_request<T>(&mut self, _: &ReqRespProtocol, io: &mut T)
        -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send
    {
        read_one(io, 1024)
            .map(|res| match res {
                Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                Ok(vec) if vec.is_empty() => Ok(Req::Ping()),
                Ok(vec) => Ok(Req::Get(vec)),
            })
            .await
    }

    async fn read_response<T>(&mut self, _: &ReqRespProtocol, io: &mut T)
        -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send {
        read_one(io, 1024)
            .map(|res| match res {
                Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                Ok(vec) if vec.is_empty() => Ok(Resp::Pong()),
                Ok(vec) => Ok(Resp::Data(vec)),
            })
            .await
    }

    async fn write_request<T>(&mut self, _: &ReqRespProtocol, io: &mut T, req : Req )
        -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send {
        match req {
         Req::Ping() =>  write_one(io, "".as_bytes()).await,
         Req::Get(data) =>  write_one(io, data).await,
       }
    }

    async fn write_response<T>(&mut self, _: &ReqRespProtocol, io: &mut T, resp: Resp)
        -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send {
         match resp {
         Resp::Pong() =>  write_one(io, "".as_bytes()).await,
         Resp::Data(data) =>  write_one(io, data).await,
       }
    }
}
