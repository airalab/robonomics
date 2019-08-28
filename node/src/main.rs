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

fn main() {
    let version = cli::VersionInfo {
        name: "Robonomics Node",
        author: "Airalab <research@aira.life>",
        commit: env!("VERGEN_SHA_SHORT"),
        version: env!("CARGO_PKG_VERSION"),
        description: "Substrate based implementation of Robonomics Network",
        support_url: "https://github.com/airalab/substrate-node-robonomics/issues",
        executable_name: "robonomics",
    };

    if let Err(e) = cli::run(::std::env::args(), cli::Exit, version) {
        eprintln!("Error starting the node: {}\n\n{:?}", e, e);
        std::process::exit(1)
    }
}
