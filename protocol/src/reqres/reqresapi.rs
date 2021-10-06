extern crate chrono;
use crate::reqres::*;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use std::fmt;
use chrono::prelude::*;

fn epochu() -> i64 {
  let now = Utc::now();
  let seconds: i64 = now.timestamp();
  let nanoseconds: i64 = now.nanosecond() as i64;
  (seconds * 1000 * 1000) + (nanoseconds / 1000)
}

fn get_addr(address: String) -> (String,String) {
  let ma = address.clone();
  let v: Vec<&str> = address.split('/').collect();
  let peer_id = v.last().unwrap().to_string();
  
  let mut multi_addr = ma.replace(&peer_id.clone(),"");
  let _ = multi_addr.pop();
  (multi_addr,peer_id)
}

#[rpc]
pub trait  ReqRespT {
    #[rpc(name = "p2p_get")]
    /// Returns for p2p rpc get responce    
    fn p2p_get(&self, address: String, message: String) -> Result<String> ;
    #[rpc(name = "p2p_ping")]
    /// Returns for reqresp p2p rpc ping responce
    fn p2p_ping(&self, address: String) -> Result<String> ;
}

pub struct ReqRespApi;

impl ReqRespT for ReqRespApi {

    fn p2p_get(&self, address: String, message: String) -> Result<String> {
        let (multiaddr, peerid) = get_addr (address.clone());
        let value = Some(message.clone().to_string()); 
        let method = "get".to_string();   

        let res = reqresapi::reqres(multiaddr.clone(), peerid.clone(), method.clone(), value.clone()).unwrap();

        let mut line = String::new();
        println!("sent: {} {} {}",multiaddr.clone(), peerid, method);
        fmt::write (&mut line, format_args!("{} {} {} {} {:?}",multiaddr.clone(), peerid, method, message, res)).unwrap_or(());
      
        Ok(line)
  }

  fn p2p_ping(&self, address: String) -> Result<String> {
        let (multiaddr, peerid) = get_addr (address.clone());
        let ping = "ping".to_string();

        let t0 = epochu();
        let res = reqresapi::reqres(multiaddr.clone(), peerid.clone(), ping.clone(), None).unwrap();
        let dt = epochu() - t0;
        
        let mut line = String::new();
        println!("sent: {} {} {} {} us",multiaddr.clone(), peerid, ping.clone(), dt);
        fmt::write (&mut line, format_args!("{} {} {} {:?} {} us",multiaddr.clone(), peerid, ping, res, dt)).unwrap_or(());
      
        Ok(line)
  }
}
