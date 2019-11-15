///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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
//! Robonomics node executable.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

use std::cell::RefCell;
use futures::sync::oneshot;
use futures::{future, Future};
use substrate_cli::VersionInfo;

// handles ctrl-c
struct Exit;
impl substrate_cli::IntoExit for Exit {
    type Exit = future::MapErr<oneshot::Receiver<()>, fn(oneshot::Canceled) -> ()>;
    fn into_exit(self) -> Self::Exit {
        // can't use signal directly here because CtrlC takes only `Fn`.
        let (exit_send, exit) = oneshot::channel();

        let exit_send_cell = RefCell::new(Some(exit_send));
        ctrlc::set_handler(move || {
            if let Some(exit_send) = exit_send_cell.try_borrow_mut().expect("signal handler not reentrant; qed").take() {
                exit_send.send(()).expect("Error sending exit notification");
            }
        }).expect("Error setting Ctrl-C handler");

        exit.map_err(drop)
    }
}

fn main() -> Result<(), substrate_cli::error::Error> {
    let version = VersionInfo {
        name: "Robonomics Node",
        commit: env!("VERGEN_SHA_SHORT"),
        version: env!("CARGO_PKG_VERSION"),
        executable_name: "robonomics-node",
        author: "Airalab <research@aira.life>",
        description: "Substrate based implementation of Robonomics Network",
        support_url: "https://github.com/airalab/substrate-node-robonomics/issues",
    };

    node_cli::run(std::env::args(), Exit, version)
}
