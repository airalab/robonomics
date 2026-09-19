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
//! Runtime API definition exposing canonical CPS Scope resolution to
//! off-chain clients (e.g. Subxt-based tooling such as `libcps`).
//!
//! This crate must be imported and implemented by the runtime of a node that
//! wants clients to resolve the active `ScopeId` of a CPS node without
//! reimplementing Scope-inheritance traversal client-side.
//!
//! The API is a thin, read-only wrapper over
//! [`pallet_robonomics_cps::Pallet::resolve_scope`], the single canonical
//! resolver used by every authorization check inside the CPS pallet. It is
//! callable through the standard generic runtime-call mechanism (e.g.
//! Substrate's `state_call` RPC, as used by Subxt and `polkadot-omni-node`),
//! so no custom Robonomics JSON-RPC endpoint or node-side customization is
//! required.
//!
//! Only the `ScopeId` is returned: the Scope's root and owner are ordinary
//! `ScopeRoot` / `ScopeOwner` storage entries that a client can already read
//! directly once it knows the `ScopeId`.
//!
//! Because it is a normal Runtime API, it is automatically included in
//! runtime metadata and can be queried at any historical block height
//! through the standard Subxt block API, which is important since a node's
//! resolved Scope may change over time.
#![cfg_attr(not(feature = "std"), no_std)]

use pallet_robonomics_cps::{NodeId, ScopeId};

sp_api::decl_runtime_apis! {
    /// The API to resolve canonical CPS Scope.
    pub trait CpsApi {
        /// Resolve the `ScopeId` of the Scope currently active for `node`.
        ///
        /// This is the `ScopeId` of `node` itself if it carries an
        /// `ActiveScope` entry, or of the nearest ancestor that does.
        ///
        /// Returns `None` if `node` does not exist or if no active Scope
        /// could be found while walking its ancestry (this should not
        /// normally happen, since every valid tree has a root Scope, but a
        /// malformed/incomplete tree state is represented as `None` rather
        /// than trapping the call).
        fn resolve_scope(node: NodeId) -> Option<ScopeId>;
    }
}
