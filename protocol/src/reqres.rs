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
pub enum Request {
    Ping,
    Get(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Pong,
    Data(Vec<u8>),
}
impl ProtocolName for RobonomicsProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/robonomics/1".as_bytes()
    }
}

#[derive(Clone)]
pub struct RobonomicsCodec {
    pub is_ping: bool
}

#[derive(Debug, Clone)]
pub struct RobonomicsProtocol();

#[async_trait]
impl RequestResponseCodec for RobonomicsCodec {
    type Protocol = RobonomicsProtocol;
    type Request = Request;
    type Response = Response;

    async fn read_request<T>(&mut self, _: &RobonomicsProtocol, io: &mut T)
        -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send
    {
        read_one(io, 1024)
            .map(|res| match res {
                Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                Ok(vec) if vec.is_empty() => {
                    self.is_ping = true; // set Ping flag; expected to reset with Pong response
                    Ok(Request::Ping)
                },
                Ok(vec) => Ok(Request::Get(vec)),
            })
            .await
    }

    async fn read_response<T>(&mut self, _: &RobonomicsProtocol, io: &mut T)
        -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send {
        read_one(io, 1024)
            .map(|res| match res {
                Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                Ok(vec) if vec.is_empty() => Ok(Response::Pong),
                Ok(vec) => Ok(Response::Data(vec)),
            })
            .await
    }

    async fn write_request<T>(&mut self, _: &RobonomicsProtocol, io: &mut T, req : Request )
        -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send {
        match req {
         Request::Ping =>  write_one(io, "".as_bytes()).await,
         Request::Get(data) =>  write_one(io, data).await,
       }
    }

    async fn write_response<T>(&mut self, _: &RobonomicsProtocol, io: &mut T, resp: Response)
        -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send {
         match resp {
            Response::Pong => {
                self.is_ping = false; // reset Ping flag 
                write_one(io, "".as_bytes()).await
            },
            // send Pong if somebody try in app logic to obfuscate Ping by sending Data instead of Pong
            Response::Data(data) => {
                if self.is_ping == false {
                    write_one(io, data).await
                } 
                else {
                    write_one(io, "".as_bytes()).await
                }
            },
        }
    }
}
