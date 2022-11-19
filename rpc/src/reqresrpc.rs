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
use robonomics_protocol::reqres::*;

use chrono::prelude::*;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
};
use std::fmt;
//use futures::{AsyncRead, AsyncWrite, FutureExt};

fn epochu() -> i64 {
    let now = Utc::now();
    let seconds: i64 = now.timestamp();
    let nanoseconds: i64 = now.nanosecond() as i64;
    (seconds * 1000 * 1000) + (nanoseconds / 1000)
}

pub fn get_addr(address: String) -> (String, String) {
    let ma = address.clone();
    let v: Vec<&str> = address.split('/').collect();
    let peer_id = v.last().unwrap().to_string();

    let mut multi_addr = ma.replace(&peer_id.clone(), "");
    let _ = multi_addr.pop();
    (multi_addr, peer_id)
}

#[rpc(server)]
pub trait ReqRespRpc {
    /// Returns for p2p rpc get responce
    #[method(name = "p2p_get")]
    fn p2p_get(&self, address: String, message: String) -> RpcResult<String>;

    /// Returns for reqresp p2p rpc ping responce
    #[method(name = "p2p_ping")]
    fn p2p_ping(&self, address: String) -> RpcResult<String>;
}

pub struct ReqRespRpc;

impl ReqRespRpc {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ReqRespRpcServer for ReqRespRpc {
    fn p2p_get(&self, address: String, message: String) -> RpcResult<String> {
        let (multiaddr, peerid) = get_addr(address.clone());
        let value = Some(message.clone().to_string());
        let method = "get".to_string();

        let res = reqres(
            multiaddr.clone(),
            peerid.clone(),
            method.clone(),
            value.clone(),
        );
        let fres = futures::executor::block_on(res);

        let mut line = String::new();
        fmt::write(
            &mut line,
            format_args!(
                "{} {} {} {} {:?}",
                multiaddr.clone(),
                peerid,
                method,
                message,
                fres
            ),
        )
        .unwrap_or(());
        log::debug!("{}", line.clone());

        Ok(line)
    }

    fn p2p_ping(&self, address: String) -> RpcResult<String> {
        let (multiaddr, peerid) = get_addr(address.clone());
        let ping = "ping".to_string();

        let t0 = epochu();
        let res = reqres(multiaddr.clone(), peerid.clone(), ping.clone(), None);
        let fres = futures::executor::block_on(res);
        let dt = epochu() - t0;

        let mut line = String::new();
        fmt::write(
            &mut line,
            format_args!(
                "{} {} {} {:?} {} us",
                multiaddr.clone(),
                peerid,
                ping,
                fres,
                dt
            ),
        )
        .unwrap_or(());
        log::debug!("{}", line.clone());

        Ok(line)
    }
}
