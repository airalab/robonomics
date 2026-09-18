///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
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
//! # CPS Runtime API
//!
//! Runtime API definition exposing canonical CPS Ownership resolution to
//! off-chain clients (e.g. Subxt-based tooling such as `libcps`).
//!
//! This crate must be imported and implemented by the runtime of a node that
//! wants clients to resolve the effective Ownership of a CPS node without
//! reimplementing ownership-inheritance traversal client-side.
//!
//! The API is a thin, read-only wrapper over
//! [`pallet_robonomics_cps::Pallet::resolve_ownership`], the single canonical
//! resolver used by every authorization check inside the CPS pallet. It is
//! callable through the standard generic runtime-call mechanism (e.g.
//! Substrate's `state_call` RPC, as used by Subxt and `polkadot-omni-node`),
//! so no custom Robonomics JSON-RPC endpoint or node-side customization is
//! required.
//!
//! Because it is a normal Runtime API, it is automatically included in
//! runtime metadata and can be queried at any historical block height
//! through the standard Subxt block API, which is important since Ownership
//! may change over time.
#![cfg_attr(not(feature = "std"), no_std)]

use pallet_robonomics_cps::NodeId;
use parity_scale_codec::Codec;

sp_api::decl_runtime_apis! {
    /// The API to resolve canonical CPS Ownership.
    pub trait NodeOwnership<AccountId> where
        AccountId: Codec,
    {
        /// Resolve the effective Ownership boundary and owner for `node`.
        ///
        /// Returns `(root, owner)` where `root` is the `NodeId` of the
        /// resolved Ownership boundary (the node that carries the explicit
        /// `Ownership` entry, either the queried node itself or the nearest
        /// ancestor with one) and `owner` is the effective owner account of
        /// that boundary.
        ///
        /// The root is included (not just the owner) because it represents
        /// the actual administrative and accounting boundary; future
        /// resource-resolution logic may use it directly.
        ///
        /// Returns `None` if `node` does not exist or if no Ownership
        /// boundary could be found while walking its ancestry (this should
        /// not normally happen, since every valid tree has a root boundary,
        /// but a malformed/incomplete tree state is represented as `None`
        /// rather than trapping the call).
        fn resolve_ownership(node: NodeId) -> Option<(NodeId, AccountId)>;
    }
}
