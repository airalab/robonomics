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

extern crate vergen;

use vergen::{ConstantsFlags, Vergen};

const ERROR_MSG: &'static str = "Failed to generate metadata files";

fn main() {
	let vergen = Vergen::new(ConstantsFlags::all()).expect(ERROR_MSG);

	for (k, v) in vergen.build_info() {
		println!("cargo:rustc-env={}={}", k.name(), v);
	}

	println!("cargo:rerun-if-changed=.git/HEAD");
}
