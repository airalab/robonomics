// extern crate chrono;
// use crate::reqres::*;
// use chrono::prelude::*;
// use jsonrpc_core::Result;
// use jsonrpc_derive::rpc;
// use std::fmt;
//
// fn epochu() -> i64 {
//     let now = Utc::now();
//     let seconds: i64 = now.timestamp();
//     let nanoseconds: i64 = now.nanosecond() as i64;
//     (seconds * 1000 * 1000) + (nanoseconds / 1000)
// }
//
// fn get_addr(address: String) -> (String, String) {
//     let ma = address.clone();
//     let v: Vec<&str> = address.split('/').collect();
//     let peer_id = v.last().unwrap().to_string();
//
//     let mut multi_addr = ma.replace(&peer_id.clone(), "");
//     let _ = multi_addr.pop();
//     (multi_addr, peer_id)
// }
//
// #[rpc]
// pub trait ReqRespT {
//     /// Returns for p2p rpc get responce
//     #[rpc(name = "p2p_get")]
//     fn p2p_get(&self, address: String, message: String) -> Result<String>;
//
//     /// Returns for reqresp p2p rpc ping responce
//     #[rpc(name = "p2p_ping")]
//     fn p2p_ping(&self, address: String) -> Result<String>;
// }
//
// pub struct ReqRespApi;
//
// impl ReqRespT for ReqRespApi {
//     fn p2p_get(&self, address: String, message: String) -> Result<String> {
//         let (multiaddr, peerid) = get_addr(address.clone());
//         let value = Some(message.clone().to_string());
//         let method = "get".to_string();
//
//         let res = reqresapi::reqres(
//             multiaddr.clone(),
//             peerid.clone(),
//             method.clone(),
//             value.clone(),
//         );
//         let fres = futures::executor::block_on(res);
//
//         let mut line = String::new();
//         fmt::write(
//             &mut line,
//             format_args!(
//                 "{} {} {} {} {:?}",
//                 multiaddr.clone(),
//                 peerid,
//                 method,
//                 message,
//                 fres
//             ),
//         )
//         .unwrap_or(());
//         log::debug!("{}", line.clone());
//         Ok(line)
//     }
//
//     fn p2p_ping(&self, address: String) -> Result<String> {
//         let (multiaddr, peerid) = get_addr(address.clone());
//         let ping = "ping".to_string();
//
//         let t0 = epochu();
//         let res = reqresapi::reqres(multiaddr.clone(), peerid.clone(), ping.clone(), None);
//         let fres = futures::executor::block_on(res);
//         let dt = epochu() - t0;
//
//         let mut line = String::new();
//         fmt::write(
//             &mut line,
//             format_args!(
//                 "{} {} {} {:?} {} us",
//                 multiaddr.clone(),
//                 peerid,
//                 ping,
//                 fres,
//                 dt
//             ),
//         )
//         .unwrap_or(());
//         log::debug!("{}", line.clone());
//         Ok(line)
//     }
// }
