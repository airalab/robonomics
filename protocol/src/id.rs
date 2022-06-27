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
//! Robonomics Network node identity.

use libp2p::identity::ed25519;
use libp2p::identity::Keypair;
use libp2p::PeerId;
use std::fs;
use std::path::Path;

/// Generate random ED25519 keypair for node identity.
pub fn random() -> Keypair {
    let key = Keypair::generate_ed25519();
    let peer_id = PeerId::from(key.public());
    log::info!(target: "robonomics-identity",
               "Generated random peer id: {}", peer_id.to_base58());
    key
}

/// Load ED25519 keypair for node identity from file.
pub fn load(key_path: &Path) -> Keypair {
    log::info!(target: "robonomics-identity",
               "Loading peer identity keypair from: {}", key_path.to_str().unwrap());
    let key = {
        let mut fd = fs::File::open(key_path).unwrap();
        let mut bytes = [0; 64];
        fd.read(&mut bytes);
        let key_ = ed25519::Keypair::decode(&mut bytes).unwrap();
        Keypair::Ed25519(key_)
    };
    let peer_id = PeerId::from(key.public());
    log::info!(target: "robonomics-identity",
               "Peer id: {}", peer_id.to_base58());
    key
}

/// Save node identity ED25519 keypair at path given.
pub fn save(key: Keypair, dest: &Path) -> std::io::Result<()> {
    log::info!(target: "robonomics-identity",
               "Saving peer identity keypair to: {}", dest.to_str().unwrap());
    let key_ = match key {
        Keypair::Ed25519(k) => Some(k),
        _ => None,
    };
    let bytes = key_.unwrap().encode();
    let mut buff = fs::File::create(dest).unwrap();
    buff.write(&bytes);
    buff.flush();
    Ok(())
}
