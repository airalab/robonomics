///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
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

/// Integration tests that spawn the actual binary `polkadot-omni-node`
/// using `assert_cmd`. We verify that the help text
/// excludes the `export-chain-spec` sub‑command exactly as intended
use assert_cmd::Command;

#[test]
fn robonomics_omni_node_export_chain_spec() {
    let output = Command::cargo_bin("robonomics")
        .expect("binary `robonomics` should be built by the workspace")
        .arg("export-chain-spec")
        .arg("--chain")
        .arg("kusama")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("Robonomics Kusama"),
        "binary must export kusama parachain spec"
    );

    let output = Command::cargo_bin("robonomics")
        .expect("binary `robonomics` should be built by the workspace")
        .arg("export-chain-spec")
        .arg("--chain")
        .arg("polkadot")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("Robonomics Polkadot"),
        "binary must export polkadot parachain spec"
    );
}
