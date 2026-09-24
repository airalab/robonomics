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
//! wants clients to resolve the Scope effective for a CPS node without
//! reimplementing Scope-resolution traversal client-side.
//!
//! The API is a thin, read-only wrapper over
//! [`pallet_robonomics_cps::Pallet::resolve_scope`] and
//! [`pallet_robonomics_cps::Pallet::has_capability`], the same canonical
//! Scope resolver and authorization check used by every dispatchable inside
//! the CPS pallet. It is callable through the standard generic runtime-call
//! mechanism (e.g. Substrate's `state_call` RPC, as used by Subxt and
//! `polkadot-omni-node`), so no custom Robonomics JSON-RPC endpoint or
//! node-side customization is required.
//!
//! `resolve_scope` returns the `ScopeId`, root `NodeId`, and owner
//! `AccountId` together (as a [`pallet_robonomics_cps::ResolvedScope`]),
//! since the pallet's `ActiveScope` storage already merges them into a
//! single entry and resolving all three only requires one walk of the
//! node's ancestry. The pallet's own `resolve_scope` returns a `Result`;
//! the runtime implementation collapses any error into `None` before
//! crossing the API boundary.
//!
//! Because these are normal Runtime API methods, they are automatically
//! included in runtime metadata and can be queried at any historical block
//! height through the standard Subxt block API, which is important since a
//! node's resolved Scope and granted capabilities may change over time.
#![cfg_attr(not(feature = "std"), no_std)]

use pallet_robonomics_cps::{Capability, NodeId, ResolvedScope};
use parity_scale_codec::Codec;

sp_api::decl_runtime_apis! {
    /// Runtime API for resolving CPS Scope and capability checks.
    ///
    /// Exposes read-only access to the CPS pallet's canonical authorization
    /// logic (Scope resolution and capability checks) so off-chain clients
    /// can query them directly via `state_call`, without duplicating the
    /// Scope-resolution traversal logic client-side.
    pub trait CpsApi<AccountId> where
        AccountId: Codec
    {
        /// Resolve the Scope currently active for `node`.
        ///
        /// This is the Scope of `node` itself if it carries an
        /// `ActiveScope` entry, or of the nearest ancestor that does. The
        /// returned [`ResolvedScope`] carries the `ScopeId`, the `NodeId` of
        /// the Scope's root, and the owner `AccountId` together, since the
        /// pallet resolves all three in a single ancestry walk.
        ///
        /// Returns `None` if `node` does not exist or if no active Scope
        /// could be found while walking its ancestry (this should not
        /// normally happen, since every valid tree has a root Scope, but a
        /// malformed/incomplete tree state is represented as `None` rather
        /// than trapping the call).
        fn resolve_scope(node: NodeId) -> Option<ResolvedScope<AccountId>>;

        /// Check whether `account_id` currently holds `capability` at `node_id`.
        ///
        /// Reuses the same canonical authorization logic enforced by the
        /// pallet's dispatchables (Scope owner implicit authority, exact
        /// `GrantMode::Node` / propagating `GrantMode::Subtree` `Access`
        /// never crossing a nested Scope boundary).
        /// Returns `false` (rather than trapping the call) if `node_id`
        /// does not exist or no Scope can be resolved for it.
        fn has_capability(node_id: NodeId, account_id: AccountId, capability: Capability) -> bool;
    }
}
