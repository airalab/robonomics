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
//! Tests for pallet-robonomics-cps

use crate::{self as pallet_cps, *};
use frame_support::{assert_noop, assert_ok, derive_impl, BoundedVec};
use parity_scale_codec::Encode;
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Runtime>;

frame_support::construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Cps: pallet_cps,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type AccountData = ();
    type DbWeight = frame_support::weights::constants::RocksDbWeight;
}

impl pallet_cps::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::TestWeightInfo;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Build a bounded byte vector of any bound `S` - used for both `NodeMeta`
/// (bound = [`MaxMetaSize`]) and `NodePayload` (bound = [`MaxPayloadSize`])
/// values in these tests; the bound to use is inferred from the calling
/// context.
fn data<S: frame_support::traits::Get<u32>>(bytes: &[u8]) -> BoundedVec<u8, S> {
    BoundedVec::try_from(bytes.to_vec()).unwrap()
}

fn active_scope_id(node_id: NodeId) -> Option<ScopeId> {
    Cps::node_info(node_id).and_then(|info| info.scope)
}

/// `Some(Some(parent))` / `Some(None)` mirror the pre-refactor `Parents`
/// accessor shape: outer `Option` is node existence, inner is "has a
/// parent". `None` means the node does not exist.
fn parent_of(node_id: NodeId) -> Option<Option<NodeId>> {
    Cps::node_info(node_id).map(|info| info.parent)
}

/// Mirrors the pre-refactor `ActiveScope` accessor: `Some((scope_id,
/// owner))` only if `node_id` is itself the root of an active Scope.
fn active_scope(node_id: NodeId) -> Option<(ScopeId, u64)> {
    let scope_id = Cps::node_info(node_id).and_then(|info| info.scope)?;
    let owner = Cps::scope_info(scope_id)?.owner;
    Some((scope_id, owner))
}

/// Mirrors the pre-refactor `NodesByParent` accessor.
fn children_of(node_id: NodeId) -> BoundedVec<NodeId, MaxChildrenPerNode> {
    Cps::children_of(node_id)
}

fn assert_scope(node_id: NodeId, expected_id: ScopeId, expected_root: NodeId, expected_owner: u64) {
    assert_eq!(
        Cps::resolve_scope(node_id),
        Ok(ResolvedScope {
            id: expected_id,
            root: expected_root,
            owner: expected_owner,
        })
    );
    assert_eq!(
        Cps::scope_info(expected_id).map(|info| info.owner),
        Some(expected_owner)
    );
}

fn grant_many(owner: u64, node_id: NodeId, first_principal: u64, count: u64) {
    for principal in first_principal..(first_principal + count) {
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(owner),
            node_id,
            principal,
            Capability::Write,
            GrantMode::Node,
        ));
    }
}

/// Number of physical `Access` entries actually stored under `scope_id`.
fn access_count(scope_id: ScopeId) -> usize {
    Access::<Runtime>::iter_prefix(scope_id).count()
}

/// Asserts that [`ScopeInfo::access_count`] exactly matches the number of
/// physical `Access` entries actually stored under `scope_id` - the
/// invariant that makes single-call synchronous Scope cleanup sound.
fn assert_access_count_consistent(scope_id: ScopeId) {
    assert_eq!(
        Cps::scope_info(scope_id).map(|info| info.access_count as usize),
        Some(access_count(scope_id)),
        "ScopeInfo.access_count out of sync with actual Access entries for {scope_id:?}"
    );
}

#[test]
fn create_root_node_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        assert_eq!(Cps::next_node_id(), NodeId(1));
        assert_eq!(parent_of(NodeId(0)), Some(None));
        assert_eq!(active_scope(NodeId(0)), Some((ScopeId(0), account)));
        assert_eq!(Cps::next_scope_id(), ScopeId(1));
        assert_scope(NodeId(0), ScopeId(0), NodeId(0), account);
    });
}

#[test]
fn create_child_node_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(0)),
            None,
            None
        ));

        assert_eq!(parent_of(NodeId(1)), Some(Some(NodeId(0))));
        assert_eq!(active_scope(NodeId(1)), None);
        assert_scope(NodeId(1), ScopeId(0), NodeId(0), account);
        assert_eq!(children_of(NodeId(0)).len(), 1);
        assert_eq!(children_of(NodeId(0))[0], NodeId(1));
    });
}

#[test]
fn create_node_with_data_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        let meta = Some(data(b"meta"));
        let payload = Some(data(b"payload"));

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            meta.clone(),
            payload.clone()
        ));

        assert_eq!(Cps::meta_of(NodeId(0)), meta);
        assert_eq!(Cps::payload_of(NodeId(0)), payload);
    });
}

#[test]
fn create_node_with_meta_only_works() {
    new_test_ext().execute_with(|| {
        let meta = Some(data(b"meta"));

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            meta.clone(),
            None
        ));

        assert_eq!(Cps::meta_of(NodeId(0)), meta);
        assert_eq!(Cps::payload_of(NodeId(0)), None);
    });
}

#[test]
fn create_node_with_payload_only_works() {
    new_test_ext().execute_with(|| {
        let payload = Some(data(b"payload"));

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            None,
            payload.clone()
        ));

        assert_eq!(Cps::meta_of(NodeId(0)), None);
        assert_eq!(Cps::payload_of(NodeId(0)), payload);
    });
}

#[test]
fn create_node_without_data_stores_nothing() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_eq!(Cps::meta_of(NodeId(0)), None);
        assert_eq!(Cps::payload_of(NodeId(0)), None);
    });
}

#[test]
fn create_node_parent_not_found_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(1), Some(NodeId(42)), None, None),
            Error::<Runtime>::ParentNotFound
        );
    });
}

#[test]
fn create_child_without_owner_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(2), Some(NodeId(0)), None, None),
            Error::<Runtime>::NotScopeOwner
        );
    });
}

#[test]
fn max_scope_depth_enforced() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        for i in 0..MAX_SCOPE_DEPTH - 1 {
            assert_ok!(Cps::create_node(
                RuntimeOrigin::signed(account),
                Some(NodeId(i as u64)),
                None,
                None
            ));
        }

        let deepest = NodeId(MAX_SCOPE_DEPTH as u64 - 1);
        assert_eq!(
            Cps::resolve_scope_path(deepest).unwrap().path.len(),
            MAX_SCOPE_DEPTH as usize
        );
        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(account), Some(deepest), None, None),
            Error::<Runtime>::MaxScopeDepthExceeded
        );
        assert_scope(deepest, ScopeId(0), NodeId(0), account);
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(account),
            deepest,
            Some(data(b"at limit"))
        ));
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(account), deepest));
    });
}

#[test]
fn create_scope_resets_local_depth_to_zero() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        // Walk all the way to the scope-depth limit under the root Scope.
        for i in 0..MAX_SCOPE_DEPTH - 1 {
            assert_ok!(Cps::create_node(
                RuntimeOrigin::signed(account),
                Some(NodeId(i as u64)),
                None,
                None
            ));
        }
        let deepest = NodeId(MAX_SCOPE_DEPTH as u64 - 1);

        // Carving out a fresh Scope at `deepest` resets its local depth to
        // zero, so a child can immediately be created under it even though
        // the *global* tree depth already exceeds `MAX_SCOPE_DEPTH`.
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(account), deepest));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(deepest),
            None,
            None
        ));
    });
}

#[test]
fn max_children_per_node_enforced() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        for _ in 0..MAX_CHILDREN_PER_NODE {
            assert_ok!(Cps::create_node(
                RuntimeOrigin::signed(account),
                Some(NodeId(0)),
                None,
                None
            ));
        }

        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(account), Some(NodeId(0)), None, None),
            Error::<Runtime>::TooManyChildren
        );
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(account), NodeId(1)));
        let next = Cps::next_node_id();
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(0)),
            None,
            None
        ));

        let children = children_of(NodeId(0));
        assert_eq!(children.len(), MAX_CHILDREN_PER_NODE as usize);
        assert!(!children.contains(&NodeId(1)));
        assert_eq!(children.last(), Some(&next));
    });
}

#[test]
fn set_meta_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let meta = Some(data(b"updated"));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(1),
            NodeId(0),
            meta.clone()
        ));
        assert_eq!(Cps::meta_of(NodeId(0)), meta);
    });
}

#[test]
fn set_meta_without_access_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_noop!(
            Cps::set_meta(RuntimeOrigin::signed(2), NodeId(0), Some(data(b"x"))),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn set_meta_missing_node_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Cps::set_meta(RuntimeOrigin::signed(1), NodeId(0), Some(data(b"x"))),
            Error::<Runtime>::NodeNotFound
        );
    });
}

#[test]
fn set_payload_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let payload = Some(data(b"updated"));
        assert_ok!(Cps::set_payload(
            RuntimeOrigin::signed(1),
            NodeId(0),
            payload.clone()
        ));
        assert_eq!(Cps::payload_of(NodeId(0)), payload);
    });
}

#[test]
fn replace_meta_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            Some(data(b"original")),
            None
        ));
        let replacement = Some(data(b"replacement"));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(1),
            NodeId(0),
            replacement.clone()
        ));
        assert_eq!(Cps::meta_of(NodeId(0)), replacement);
    });
}

#[test]
fn replace_payload_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            None,
            Some(data(b"original"))
        ));
        let replacement = Some(data(b"replacement"));
        assert_ok!(Cps::set_payload(
            RuntimeOrigin::signed(1),
            NodeId(0),
            replacement.clone()
        ));
        assert_eq!(Cps::payload_of(NodeId(0)), replacement);
    });
}

#[test]
fn remove_meta_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            Some(data(b"meta")),
            None
        ));
        assert_ok!(Cps::set_meta(RuntimeOrigin::signed(1), NodeId(0), None));
        assert_eq!(Cps::meta_of(NodeId(0)), None);
    });
}

#[test]
fn remove_payload_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            None,
            Some(data(b"payload"))
        ));
        assert_ok!(Cps::set_payload(RuntimeOrigin::signed(1), NodeId(0), None));
        assert_eq!(Cps::payload_of(NodeId(0)), None);
    });
}

#[test]
fn clear_meta_and_payload_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            Some(data(b"meta")),
            Some(data(b"payload"))
        ));
        assert_ok!(Cps::set_meta(RuntimeOrigin::signed(1), NodeId(0), None));
        assert_ok!(Cps::set_payload(RuntimeOrigin::signed(1), NodeId(0), None));
        assert_eq!(Cps::meta_of(NodeId(0)), None);
        assert_eq!(Cps::payload_of(NodeId(0)), None);
    });
}

#[test]
fn delete_leaf_node_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));

        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(1), NodeId(1)));
        assert_eq!(parent_of(NodeId(1)), None);
        assert!(children_of(NodeId(0)).is_empty());
    });
}

#[test]
fn delete_root_node_synchronously_clears_scope_access() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        grant_many(1, NodeId(0), 100, 3);
        assert_eq!(access_count(ScopeId(0)), 3);

        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(1), NodeId(0)));

        assert_eq!(Cps::node_info(NodeId(0)).and_then(|i| i.parent), None);
        assert_eq!(active_scope(NodeId(0)), None);
        assert_eq!(access_count(ScopeId(0)), 0);
        assert_eq!(access_count(ScopeId(0)), 0);
    });
}

#[test]
fn delete_node_with_children_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        assert_noop!(
            Cps::delete_node(RuntimeOrigin::signed(1), NodeId(0)),
            Error::<Runtime>::NodeHasChildren
        );
    });
}

#[test]
fn delete_node_without_owner_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_noop!(
            Cps::delete_node(RuntimeOrigin::signed(2), NodeId(0)),
            Error::<Runtime>::NotScopeOwner
        );
    });
}

#[test]
fn delete_node_not_found_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Cps::delete_node(RuntimeOrigin::signed(1), NodeId(0)),
            Error::<Runtime>::NodeNotFound
        );
    });
}

#[test]
fn nested_scope_inheritance_works() {
    new_test_ext().execute_with(|| {
        let owner = 1u64;

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(owner),
            None,
            None,
            None
        ));
        let global = NodeId(0);
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(owner),
            Some(global),
            None,
            None
        ));
        let japan = NodeId(1);
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(owner),
            Some(japan),
            None,
            None
        ));
        let university = NodeId(2);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(owner), university));
        let uni_scope = active_scope_id(university).unwrap();

        assert_scope(global, ScopeId(0), global, owner);
        assert_scope(japan, ScopeId(0), global, owner);
        assert_scope(university, uni_scope, university, owner);

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(owner),
            Some(university),
            None,
            None
        ));
        let sensor = NodeId(3);
        assert_scope(sensor, uni_scope, university, owner);
    });
}

#[test]
fn same_owner_nested_scope_is_still_a_hard_boundary() {
    new_test_ext().execute_with(|| {
        let owner = 1u64;
        let delegate = 2u64;

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(owner),
            None,
            None,
            None
        ));
        let root = NodeId(0);
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(owner),
            Some(root),
            None,
            None
        ));
        let child = NodeId(1);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(owner), child));

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(owner),
            root,
            delegate,
            Capability::Write,
            GrantMode::Subtree,
        ));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(delegate),
            root,
            Some(data(b"root"))
        ));
        assert_noop!(
            Cps::set_meta(RuntimeOrigin::signed(delegate), child, Some(data(b"child"))),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn resolve_scope_missing_node_fails() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            Cps::resolve_scope(NodeId(0)),
            Err(Error::<Runtime>::NodeNotFound)
        );
    });
}

#[test]
fn root_scope_created_on_root_creation() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_eq!(active_scope(NodeId(0)), Some((ScopeId(0), 1)));
        assert_scope(NodeId(0), ScopeId(0), NodeId(0), 1);
    });
}

#[test]
fn owner_can_create_nested_scope_on_child() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), NodeId(1)));
        let new_scope = active_scope(NodeId(1)).unwrap();
        assert_ne!(new_scope.0, ScopeId(0));
        assert_eq!(new_scope, (ScopeId(1), 1));
        assert_scope(NodeId(1), ScopeId(1), NodeId(1), 1);
    });
}

#[test]
fn delegated_create_scope_can_replace_existing_scope_root() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        let old_scope = active_scope_id(root).unwrap();

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::CreateScope,
            GrantMode::Node,
        ));
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(2), root));

        let new_scope = active_scope(root).unwrap();
        assert_ne!(new_scope.0, old_scope);
        assert_eq!(new_scope.1, 2);
        // The old Scope's Access entries (including the delegation just
        // exercised) are synchronously cleared as part of replacement.
        assert_eq!(access_count(old_scope), 0);
        assert_eq!(access_count(old_scope), 0);
    });
}

#[test]
fn scope_replacement_allocates_fresh_id_and_clears_old_access_synchronously() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        let japan = NodeId(1);

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), japan));
        let old_scope = active_scope_id(japan).unwrap();
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            japan,
            2,
            Capability::Write,
            GrantMode::Subtree,
        ));
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            japan,
            2,
            Capability::CreateScope,
            GrantMode::Node,
        ));
        assert_eq!(access_count(old_scope), 1);

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(2), japan));

        let new_scope = active_scope(japan).unwrap();
        assert_ne!(new_scope.0, old_scope);
        assert_eq!(new_scope.1, 2);
        // The old Scope's `Access` entry is synchronously cleared as part
        // of replacement - it no longer physically exists at all.
        assert!(!Access::<Runtime>::contains_key(old_scope, (japan, 2)));
        assert_eq!(access_count(old_scope), 0);
        assert_eq!(access_count(old_scope), 0);
        assert_noop!(
            Cps::set_meta(RuntimeOrigin::signed(1), japan, Some(data(b"x"))),
            Error::<Runtime>::AccessDenied
        );
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(2),
            japan,
            Some(data(b"x"))
        ));
    });
}

#[test]
fn create_scope_id_never_reused() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        let node = NodeId(1);

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), node));
        let first = active_scope_id(node).unwrap();
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), node));
        let second = active_scope_id(node).unwrap();
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), node));
        let third = active_scope_id(node).unwrap();

        assert_ne!(first, second);
        assert_ne!(second, third);
        assert_ne!(first, third);
    });
}

#[test]
fn create_scope_without_access_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_noop!(
            Cps::create_scope(RuntimeOrigin::signed(2), NodeId(0)),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn create_scope_capability_rejected_on_descendants() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        let root = NodeId(0);
        let child = NodeId(1);

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::CreateScope,
            GrantMode::Node,
        ));
        assert_noop!(
            Cps::create_scope(RuntimeOrigin::signed(2), child),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn write_access_does_not_authorize_create_scope() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Subtree,
        ));
        assert_noop!(
            Cps::create_scope(RuntimeOrigin::signed(2), root),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn create_scope_node_capability_authorizes_exact_node_even_if_not_active_scope_root() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        let child = NodeId(1);

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            child,
            2,
            Capability::CreateScope,
            GrantMode::Node,
        ));
        // A `Node` grant authorizes `create_scope` at the exact granted
        // node, carving out a brand-new nested Scope there even though
        // `child` was not previously an active Scope root.
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(2), child));
    });
}

#[test]
fn create_scope_subtree_capability_authorizes_descendants() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        let root = NodeId(0);
        let child = NodeId(1);

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::CreateScope,
            GrantMode::Subtree,
        ));

        // A `Subtree` grant on an ancestor authorizes carving out a
        // brand-new nested Scope on any descendant within the same Scope.
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(2), child));
        let new_scope = active_scope(child).unwrap();
        assert_eq!(new_scope.1, 2);
    });
}

// A Scope boundary can no longer be removed independently of its node
// (there is no `delete_scope` extrinsic anymore): it only disappears when
// its root node is deleted, and `delete_node` stays leaf-only. The tests
// below replace the old `delete_scope`-based coverage.

#[test]
fn delete_node_on_scope_root_leaf_clears_scope_and_access() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        let japan = NodeId(1);

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), japan));
        let japan_scope = active_scope_id(japan).unwrap();
        assert_scope(japan, japan_scope, japan, 1);
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            japan,
            2,
            Capability::Write,
            GrantMode::Subtree,
        ));
        assert_eq!(access_count(japan_scope), 1);

        // `japan` is a leaf (no children), so it can be deleted directly;
        // deleting a Scope-root leaf synchronously clears both its
        // `ScopeInfo` and its `Access` entries.
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(1), japan));

        assert_eq!(parent_of(japan), None);
        assert_eq!(Cps::scope_info(japan_scope), None);
        assert_eq!(access_count(japan_scope), 0);
        assert!(children_of(NodeId(0)).is_empty());
    });
}

#[test]
fn delete_node_on_scope_root_with_children_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        let japan = NodeId(1);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), japan));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(japan),
            None,
            None
        ));

        // `delete_node` stays leaf-only, so a Scope root with children
        // cannot be deleted (and its Scope therefore cannot disappear)
        // until every descendant is removed first.
        assert_noop!(
            Cps::delete_node(RuntimeOrigin::signed(1), japan),
            Error::<Runtime>::NodeHasChildren
        );
    });
}

#[test]
fn nested_scope_unaffected_by_sibling_operations() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let global = NodeId(0);
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(global),
            None,
            None
        ));
        let japan = NodeId(1);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), japan));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(japan),
            None,
            None
        ));
        let university = NodeId(2);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), university));
        let uni_scope = active_scope_id(university).unwrap();

        // A sibling Scope root of `japan` is created and deleted; `japan`'s
        // nested Scope (and its descendant `university`'s Scope) are
        // unaffected since Scope boundaries are now purely per-node.
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(global),
            None,
            None
        ));
        let korea = NodeId(3);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), korea));
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(1), korea));

        assert_eq!(active_scope(university), Some((uni_scope, 1)));
        assert_scope(university, uni_scope, university, 1);
        assert_scope(japan, active_scope_id(japan).unwrap(), japan, 1);
    });
}

#[test]
fn exact_node_access_does_not_apply_to_descendants() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(root),
            None,
            None
        ));
        let child = NodeId(1);

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Node,
        ));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(2),
            root,
            Some(data(b"x"))
        ));
        assert_noop!(
            Cps::set_meta(RuntimeOrigin::signed(2), child, Some(data(b"x"))),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn inherited_access_applies_to_descendants() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(root),
            None,
            None
        ));
        let child = NodeId(1);

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Subtree,
        ));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(2),
            root,
            Some(data(b"x"))
        ));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(2),
            child,
            Some(data(b"y"))
        ));
    });
}

#[test]
fn access_stopped_by_nested_scope_boundary() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(root),
            None,
            None
        ));
        let nested_root = NodeId(1);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), nested_root));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(nested_root),
            None,
            None
        ));
        let nested_child = NodeId(2);

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Subtree,
        ));
        assert_noop!(
            Cps::set_meta(RuntimeOrigin::signed(2), nested_root, Some(data(b"x"))),
            Error::<Runtime>::AccessDenied
        );
        assert_noop!(
            Cps::set_meta(RuntimeOrigin::signed(2), nested_child, Some(data(b"x"))),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn owner_has_implicit_authority_without_access_entries() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));

        assert!(!Access::<Runtime>::contains_key(
            ScopeId(0),
            (NodeId(0), 1u64)
        ));
        assert!(Cps::access(ScopeId(0), (NodeId(0), 1u64)).is_empty());
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(1),
            NodeId(1),
            Some(data(b"x"))
        ));
    });
}

#[test]
fn access_flags_bit_packing_and_storage_cleanup() {
    new_test_ext().execute_with(|| {
        let mut flags = AccessFlags::default();
        assert!(flags.is_empty());

        flags.grant(Capability::CreateScope, GrantMode::Node);
        assert!(flags.contains(Capability::CreateScope));
        assert!(!flags.applies_to_descendants(Capability::CreateScope));
        assert!(!flags.contains(Capability::Write));

        flags.grant(Capability::Write, GrantMode::Subtree);
        assert!(flags.contains(Capability::Write));
        assert!(flags.applies_to_descendants(Capability::Write));

        flags.grant(Capability::Write, GrantMode::Node);
        assert!(flags.contains(Capability::Write));
        assert!(!flags.applies_to_descendants(Capability::Write));

        flags.revoke(Capability::CreateScope);
        assert!(!flags.contains(Capability::CreateScope));
        assert!(flags.contains(Capability::Write));

        flags.revoke(Capability::Write);
        assert!(flags.is_empty());

        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        let scope_id = ScopeId(0);
        let key = (root, 2u64);

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::CreateScope,
            GrantMode::Node,
        ));
        assert_eq!(access_count(scope_id), 1);
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Subtree,
        ));
        // Adding a second capability to the same (node, principal) entry
        // does not consume another slot.
        assert_eq!(access_count(scope_id), 1);

        let stored = Cps::access(scope_id, key);
        assert!(stored.contains(Capability::CreateScope));
        assert!(!stored.applies_to_descendants(Capability::CreateScope));
        assert!(stored.contains(Capability::Write));
        assert!(stored.applies_to_descendants(Capability::Write));
        assert!(Access::<Runtime>::contains_key(scope_id, key));

        assert_ok!(Cps::revoke_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
        ));
        assert!(Access::<Runtime>::contains_key(scope_id, key));
        // A capability remains, so the entry - and the count - are
        // untouched by this revoke.
        assert_eq!(access_count(scope_id), 1);
        let stored = Cps::access(scope_id, key);
        assert!(stored.contains(Capability::CreateScope));
        assert!(!stored.contains(Capability::Write));

        assert_ok!(Cps::revoke_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::CreateScope,
        ));
        assert!(!Access::<Runtime>::contains_key(scope_id, key));
        assert!(Cps::access(scope_id, key).is_empty());
        // The last capability was revoked, so the entry is fully removed
        // and the slot is freed.
        assert_eq!(access_count(scope_id), 0);
    });
}

#[test]
fn grant_and_revoke_access_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Subtree,
        ));
        assert!(Cps::access(ScopeId(0), (root, 2u64)).contains(Capability::Write));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(2),
            root,
            Some(data(b"x"))
        ));

        assert_ok!(Cps::revoke_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
        ));
        assert!(!Access::<Runtime>::contains_key(ScopeId(0), (root, 2u64)));
        assert_noop!(
            Cps::set_meta(RuntimeOrigin::signed(2), root, Some(data(b"y"))),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn grant_access_requires_scope_owner() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_noop!(
            Cps::grant_access(
                RuntimeOrigin::signed(2),
                NodeId(0),
                3,
                Capability::Write,
                GrantMode::Subtree,
            ),
            Error::<Runtime>::NotScopeOwner
        );
    });
}

#[test]
fn revoke_access_requires_scope_owner() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            NodeId(0),
            2,
            Capability::Write,
            GrantMode::Subtree,
        ));
        assert_noop!(
            Cps::revoke_access(RuntimeOrigin::signed(2), NodeId(0), 2, Capability::Write),
            Error::<Runtime>::NotScopeOwner
        );
    });
}

#[test]
fn node_id_exhaustion_is_atomic() {
    new_test_ext().execute_with(|| {
        NextNodeId::<Runtime>::put(NodeId(u64::MAX));
        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(1), None, None, None),
            Error::<Runtime>::NodeIdExhausted
        );
        assert_eq!(Cps::next_node_id(), NodeId(u64::MAX));
    });
}

#[test]
fn scope_id_exhaustion_is_atomic() {
    new_test_ext().execute_with(|| {
        NextScopeId::<Runtime>::put(ScopeId(u64::MAX));
        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(1), None, None, None),
            Error::<Runtime>::ScopeIdExhausted
        );
        assert_eq!(Cps::next_scope_id(), ScopeId(u64::MAX));
    });
}

#[test]
fn all_extrinsics_require_signed_origin() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Cps::create_node(RuntimeOrigin::none(), None, None, None),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            Cps::set_meta(RuntimeOrigin::none(), NodeId(0), None),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            Cps::set_payload(RuntimeOrigin::none(), NodeId(0), None),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            Cps::delete_node(RuntimeOrigin::none(), NodeId(0)),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            Cps::create_scope(RuntimeOrigin::none(), NodeId(0)),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            Cps::grant_access(
                RuntimeOrigin::none(),
                NodeId(0),
                1,
                Capability::Write,
                GrantMode::Subtree,
            ),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            Cps::revoke_access(RuntimeOrigin::none(), NodeId(0), 1, Capability::Write),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn meta_exactly_at_max_size_succeeds() {
    new_test_ext().execute_with(|| {
        let meta: NodeMeta = BoundedVec::try_from(vec![7u8; MAX_META_SIZE as usize]).unwrap();

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            Some(meta.clone()),
            None
        ));

        assert_eq!(Cps::meta_of(NodeId(0)), Some(meta));
    });
}

/// The hard size limit is enforced by `NodeMeta`'s `BoundedVec` bound
/// itself: a value over `MAX_META_SIZE` cannot even be constructed, which
/// is what makes it impossible to submit a call with an over-limit `meta`
/// byte vector in the first place (the SCALE decoder rejects such an
/// extrinsic before the dispatchable ever runs).
#[test]
fn meta_above_max_size_is_rejected() {
    assert!(
        BoundedVec::<u8, MaxMetaSize>::try_from(vec![7u8; (MAX_META_SIZE + 1) as usize]).is_err()
    );
}

#[test]
fn payload_exactly_at_max_size_succeeds() {
    new_test_ext().execute_with(|| {
        let payload: NodePayload =
            BoundedVec::try_from(vec![7u8; MAX_PAYLOAD_SIZE as usize]).unwrap();

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            None,
            Some(payload.clone())
        ));

        assert_eq!(Cps::payload_of(NodeId(0)), Some(payload));
    });
}

/// See [`meta_above_max_size_is_rejected`] - same reasoning, for `Payload`.
#[test]
fn payload_above_max_size_is_rejected() {
    assert!(
        BoundedVec::<u8, MaxPayloadSize>::try_from(vec![7u8; (MAX_PAYLOAD_SIZE + 1) as usize])
            .is_err()
    );
}

#[test]
fn empty_data_values_are_preserved() {
    new_test_ext().execute_with(|| {
        let empty_meta: NodeMeta = data(b"");
        let empty_payload: NodePayload = data(b"");

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            Some(empty_meta.clone()),
            Some(empty_payload.clone())
        ));

        assert_eq!(Cps::meta_of(NodeId(0)), Some(empty_meta));
        assert_eq!(Cps::payload_of(NodeId(0)), Some(empty_payload));
    });
}

#[test]
fn successful_operations_emit_exact_events() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(1),
            NodeId(0),
            Some(data(b"m"))
        ));
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            NodeId(0),
            2,
            Capability::Write,
            GrantMode::Node,
        ));
        assert_ok!(Cps::revoke_access(
            RuntimeOrigin::signed(1),
            NodeId(0),
            2,
            Capability::Write,
        ));
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(1), NodeId(0)));

        let events: Vec<_> = System::events().into_iter().map(|r| r.event).collect();
        assert_eq!(
            events,
            vec![
                RuntimeEvent::Cps(Event::ScopeCreated(ScopeId(0), NodeId(0), 1)),
                RuntimeEvent::Cps(Event::NodeCreated(NodeId(0), None, 1)),
                RuntimeEvent::Cps(Event::MetaSet(NodeId(0), 1)),
                RuntimeEvent::Cps(Event::AccessGranted(
                    ScopeId(0),
                    NodeId(0),
                    2,
                    Capability::Write,
                    GrantMode::Node,
                )),
                RuntimeEvent::Cps(Event::AccessRevoked(
                    ScopeId(0),
                    NodeId(0),
                    2,
                    Capability::Write,
                )),
                RuntimeEvent::Cps(Event::ScopeDeleted(ScopeId(0), NodeId(0))),
                RuntimeEvent::Cps(Event::NodeDeleted(NodeId(0), 1)),
            ]
        );
    });
}

#[test]
fn deleting_node_cleans_attributes_without_reusing_id() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            Some(data(b"m")),
            Some(data(b"p"))
        ));
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(1), NodeId(0)));

        assert_eq!(Cps::node_info(NodeId(0)).and_then(|i| i.parent), None);
        assert_eq!(Cps::meta_of(NodeId(0)), None);
        assert_eq!(Cps::payload_of(NodeId(0)), None);
        assert_eq!(active_scope(NodeId(0)), None);

        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_eq!(Cps::next_node_id(), NodeId(2));
    });
}

#[test]
fn has_capability_reflects_owner_and_access_write() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        let root = NodeId(0);
        let child = NodeId(1);

        assert!(Cps::has_capability(root, &1, Capability::Write));
        assert!(Cps::has_capability(child, &1, Capability::Write));
        assert!(!Cps::has_capability(root, &2, Capability::Write));

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Node,
        ));
        assert!(Cps::has_capability(root, &2, Capability::Write));
        assert!(!Cps::has_capability(child, &2, Capability::Write));
    });
}

#[test]
fn has_capability_reflects_create_scope_semantics() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);

        assert!(Cps::has_capability(root, &1, Capability::CreateScope));
        assert!(!Cps::has_capability(root, &2, Capability::CreateScope));

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Subtree,
        ));
        assert!(!Cps::has_capability(root, &2, Capability::CreateScope));
    });
}

#[test]
fn has_capability_returns_false_for_missing_node() {
    new_test_ext().execute_with(|| {
        assert!(!Cps::has_capability(NodeId(0), &1, Capability::Write));
    });
}

#[test]
fn create_scope_replacement_has_nothing_to_clear_when_old_scope_had_no_access() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        let old_scope = active_scope_id(root).unwrap();
        assert_eq!(access_count(old_scope), 0);

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));

        // The old `ScopeInfo` entry is removed entirely on replacement (it
        // never held any Access entries to clear).
        assert_eq!(Cps::scope_info(old_scope), None);
        assert_eq!(access_count(old_scope), 0);
    });
}

#[test]
fn create_scope_replacement_synchronously_clears_a_single_access_entry() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        let old_scope = active_scope_id(root).unwrap();
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Node,
        ));
        assert_eq!(access_count(old_scope), 1);

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));

        assert_eq!(access_count(old_scope), 0);
        assert_eq!(access_count(old_scope), 0);
        assert!(!Access::<Runtime>::contains_key(old_scope, (root, 2u64)));
    });
}

#[test]
fn create_scope_replacement_synchronously_clears_access_at_the_bound() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        let old_scope = active_scope_id(root).unwrap();
        grant_many(1, root, 100, MAX_ACCESS_ENTRIES_PER_SCOPE as u64);
        assert_eq!(
            access_count(old_scope),
            MAX_ACCESS_ENTRIES_PER_SCOPE as usize
        );

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));

        assert_eq!(access_count(old_scope), 0);
        assert_eq!(access_count(old_scope), 0);
    });
}

#[test]
fn delete_node_on_scope_root_leaf_synchronously_clears_access_at_the_bound() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        let japan = NodeId(1);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), japan));
        let scope_id = active_scope_id(japan).unwrap();
        grant_many(1, japan, 100, MAX_ACCESS_ENTRIES_PER_SCOPE as u64);
        assert_eq!(
            access_count(scope_id),
            MAX_ACCESS_ENTRIES_PER_SCOPE as usize
        );

        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(1), japan));

        assert_eq!(Cps::scope_info(scope_id), None);
        assert_eq!(access_count(scope_id), 0);
    });
}

#[test]
fn clear_scope_access_removes_all_entries_in_a_single_bounded_pass() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        let scope_id = active_scope_id(root).unwrap();
        grant_many(1, root, 100, MAX_ACCESS_ENTRIES_PER_SCOPE as u64);
        let tracked = Cps::scope_info(scope_id).unwrap().access_count;
        assert_eq!(tracked, MAX_ACCESS_ENTRIES_PER_SCOPE);
        assert_eq!(access_count(scope_id), tracked as usize);

        Cps::clear_scope_access(scope_id, tracked);

        assert_eq!(access_count(scope_id), 0);
    });
}

#[test]
fn grant_access_up_to_the_limit_succeeds_and_beyond_it_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);

        grant_many(1, root, 100, MAX_ACCESS_ENTRIES_PER_SCOPE as u64);
        assert_eq!(
            access_count(ScopeId(0)),
            MAX_ACCESS_ENTRIES_PER_SCOPE as usize
        );

        // One more distinct (node, principal) pair past the bound is
        // rejected outright.
        assert_noop!(
            Cps::grant_access(
                RuntimeOrigin::signed(1),
                root,
                100 + MAX_ACCESS_ENTRIES_PER_SCOPE as u64,
                Capability::Write,
                GrantMode::Node,
            ),
            Error::<Runtime>::TooManyAccessEntries
        );
        assert_eq!(
            access_count(ScopeId(0)),
            MAX_ACCESS_ENTRIES_PER_SCOPE as usize
        );
    });
}

#[test]
fn grant_access_mode_change_on_existing_entry_never_consumes_a_slot() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);

        // Fill every slot but one, then grant the last principal a
        // `Node` capability, using up the final slot.
        grant_many(1, root, 100, MAX_ACCESS_ENTRIES_PER_SCOPE as u64 - 1);
        let last_principal = 100 + MAX_ACCESS_ENTRIES_PER_SCOPE as u64 - 1;
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            last_principal,
            Capability::Write,
            GrantMode::Node,
        ));
        assert_eq!(
            access_count(ScopeId(0)),
            MAX_ACCESS_ENTRIES_PER_SCOPE as usize
        );

        // Changing that same principal's `GrantMode`, or adding another
        // `Capability` bit, never needs a fresh slot even though the Scope
        // is already at its bound.
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            last_principal,
            Capability::Write,
            GrantMode::Subtree,
        ));
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            last_principal,
            Capability::CreateScope,
            GrantMode::Node,
        ));
        assert_eq!(
            access_count(ScopeId(0)),
            MAX_ACCESS_ENTRIES_PER_SCOPE as usize
        );
        assert_access_count_consistent(ScopeId(0));
    });
}

#[test]
fn revoke_access_leaving_capabilities_does_not_free_a_slot() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Node,
        ));
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::CreateScope,
            GrantMode::Node,
        ));
        assert_eq!(access_count(ScopeId(0)), 1);

        assert_ok!(Cps::revoke_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
        ));
        assert_eq!(access_count(ScopeId(0)), 1);
        assert_access_count_consistent(ScopeId(0));
    });
}

#[test]
fn revoke_access_last_capability_frees_a_slot() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Node,
        ));
        assert_eq!(access_count(ScopeId(0)), 1);

        assert_ok!(Cps::revoke_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
        ));
        assert_eq!(access_count(ScopeId(0)), 0);
        assert_access_count_consistent(ScopeId(0));

        // The freed slot can immediately be used again.
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            3,
            Capability::Write,
            GrantMode::Node,
        ));
        assert_eq!(access_count(ScopeId(0)), 1);
    });
}

#[test]
fn revoking_nonexistent_access_entry_does_not_free_slots() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);

        grant_many(1, root, 100, MAX_ACCESS_ENTRIES_PER_SCOPE as u64);
        assert_eq!(
            access_count(ScopeId(0)),
            MAX_ACCESS_ENTRIES_PER_SCOPE as usize
        );

        // Revoking an entry that does not exist is a no-op for accounting.
        for _ in 0..3 {
            assert_ok!(Cps::revoke_access(
                RuntimeOrigin::signed(1),
                root,
                999,
                Capability::Write,
            ));
        }
        assert_eq!(
            access_count(ScopeId(0)),
            MAX_ACCESS_ENTRIES_PER_SCOPE as usize
        );
        assert_access_count_consistent(ScopeId(0));

        assert_noop!(
            Cps::grant_access(
                RuntimeOrigin::signed(1),
                root,
                100 + MAX_ACCESS_ENTRIES_PER_SCOPE as u64,
                Capability::Write,
                GrantMode::Node,
            ),
            Error::<Runtime>::TooManyAccessEntries
        );
    });
}

#[test]
fn clearing_scope_access_never_touches_nested_scope_state() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(root),
            None,
            None
        ));
        let nested_root = NodeId(1);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), nested_root));
        let nested_scope = active_scope_id(nested_root).unwrap();
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            nested_root,
            2,
            Capability::Write,
            GrantMode::Node,
        ));
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            3,
            Capability::Write,
            GrantMode::Node,
        ));

        // Replacing the outer Scope synchronously clears only the outer
        // Scope's Access entries; the nested Scope's own Access entries
        // are untouched (it's a hard boundary, and a different `ScopeId`
        // altogether).
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));

        assert_eq!(active_scope(nested_root), Some((nested_scope, 1)));
        assert!(
            Access::<Runtime>::get(nested_scope, (nested_root, 2u64)).contains(Capability::Write)
        );
        assert_access_count_consistent(nested_scope);
    });
}

#[test]
fn stale_scope_id_never_becomes_active_again() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root_a = NodeId(0);
        let stale_a = active_scope_id(root_a).unwrap();
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root_a));
        let stale_b = active_scope_id(root_a).unwrap();
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root_b = NodeId(1);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root_a));
        let current_a = active_scope_id(root_a).unwrap();
        let current_b = active_scope_id(root_b).unwrap();

        assert_ne!(stale_a, stale_b);
        assert_ne!(stale_a, current_a);
        assert_ne!(stale_a, current_b);
        assert_ne!(stale_b, current_a);
        assert_ne!(stale_b, current_b);

        // Both stale Scopes were cleared synchronously the moment they
        // were replaced, and never became reachable again.
        assert_eq!(access_count(stale_a), 0);
        assert_eq!(access_count(stale_b), 0);

        let active_ids: Vec<_> = Scopes::<Runtime>::iter().map(|(id, _)| id).collect();
        assert!(active_ids.contains(&current_a));
        assert!(active_ids.contains(&current_b));
        assert!(!active_ids.contains(&stale_a));
        assert!(!active_ids.contains(&stale_b));

        let resolved_a = Cps::resolve_scope(root_a).unwrap();
        let resolved_b = Cps::resolve_scope(root_b).unwrap();
        assert_eq!(resolved_a.id, current_a);
        assert_eq!(resolved_b.id, current_b);
        assert_ne!(resolved_a.id, stale_a);
        assert_ne!(resolved_a.id, stale_b);
        assert_ne!(resolved_b.id, stale_a);
        assert_ne!(resolved_b.id, stale_b);
        assert_eq!(Cps::next_scope_id(), ScopeId(4));
    });
}

#[test]
fn scope_id_is_never_reused_after_synchronous_cleanup() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        let old_scope = active_scope_id(root).unwrap();
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Node,
        ));
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));
        let new_scope = active_scope_id(root).unwrap();

        assert_eq!(access_count(old_scope), 0);

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));
        let newer_scope = active_scope_id(root).unwrap();
        assert_ne!(newer_scope, old_scope);
        assert_ne!(newer_scope, new_scope);
        assert!(u64::from(newer_scope) > u64::from(old_scope));
    });
}

/// `GrantMode`'s SCALE encoding is pinned by explicit `#[codec(index = ..)]`
/// attributes rather than derived from declaration order, so reordering the
/// variants in source can never silently change already-shipped on-chain
/// encoding (mirroring the same guarantee already relied upon for
/// `Capability`, see its `index` doc comment).
#[test]
fn grant_mode_scale_indices_are_explicit_and_stable() {
    assert_eq!(GrantMode::Node.encode(), sp_std::vec![0u8]);
    assert_eq!(GrantMode::Subtree.encode(), sp_std::vec![1u8]);
}

/// Verifies that the `#[pallet::weight(...)]` attributes on `create_node`,
/// `set_meta`, and `set_payload` actually pass the caller-supplied data's
/// *logical* byte length (not a fixed worst-case constant) into
/// `WeightInfo`, for `None`, empty, small, and maximum-size values (issue
/// #671, "Weight inputs" test category).
///
/// This needs its own mock runtime: the main `Runtime` above uses
/// `weights::TestWeightInfo`, which returns a constant zero `Weight`
/// regardless of its arguments, so it cannot distinguish "byte length
/// wasn't propagated" from "byte length was propagated, but is irrelevant
/// to a constant-zero weight". `RecordingWeightInfo` instead records
/// whatever byte length(s) it was last called with, so a dispatch info
/// query for a given call can be checked against the length actually
/// supplied by the test.
mod weight_component_tests {
    use super::*;
    use frame_support::dispatch::GetDispatchInfo;
    use frame_support::weights::Weight;
    use std::cell::Cell;

    thread_local! {
        static LAST_CREATE_NODE: Cell<(u32, u32)> = const { Cell::new((0, 0)) };
        static LAST_SET_META: Cell<u32> = const { Cell::new(0) };
        static LAST_SET_PAYLOAD: Cell<u32> = const { Cell::new(0) };
    }

    pub struct RecordingWeightInfo;
    impl WeightInfo for RecordingWeightInfo {
        fn create_node(meta_bytes: u32, payload_bytes: u32) -> Weight {
            LAST_CREATE_NODE.with(|c| c.set((meta_bytes, payload_bytes)));
            Weight::zero()
        }
        fn set_meta(bytes: u32) -> Weight {
            LAST_SET_META.with(|c| c.set(bytes));
            Weight::zero()
        }
        fn set_payload(bytes: u32) -> Weight {
            LAST_SET_PAYLOAD.with(|c| c.set(bytes));
            Weight::zero()
        }
        fn delete_node(_access_items: u32) -> Weight {
            Weight::zero()
        }
        fn create_scope(_access_items: u32) -> Weight {
            Weight::zero()
        }
        fn grant_access() -> Weight {
            Weight::zero()
        }
        fn revoke_access() -> Weight {
            Weight::zero()
        }
    }

    type WeightTestBlock = frame_system::mocking::MockBlock<WeightTestRuntime>;

    frame_support::construct_runtime!(
        pub enum WeightTestRuntime {
            System: frame_system,
            Cps: pallet_cps,
        }
    );

    #[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
    impl frame_system::Config for WeightTestRuntime {
        type Block = WeightTestBlock;
        type AccountData = ();
        type DbWeight = frame_support::weights::constants::RocksDbWeight;
    }

    impl pallet_cps::Config for WeightTestRuntime {
        type RuntimeEvent = RuntimeEvent;
        type WeightInfo = RecordingWeightInfo;
    }

    fn meta_of_len(len: usize) -> Option<NodeMeta> {
        if len == 0 {
            None
        } else {
            Some(BoundedVec::try_from(sp_std::vec![7u8; len]).unwrap())
        }
    }

    fn payload_of_len(len: usize) -> Option<NodePayload> {
        if len == 0 {
            None
        } else {
            Some(BoundedVec::try_from(sp_std::vec![7u8; len]).unwrap())
        }
    }

    #[test]
    fn set_meta_weight_uses_actual_byte_length() {
        // (input, expected logical length passed to `WeightInfo::set_meta`)
        let cases: [(Option<NodeMeta>, u32); 4] = [
            (None, 0),
            (Some(BoundedVec::try_from(sp_std::vec![]).unwrap()), 0),
            (meta_of_len(10), 10),
            (meta_of_len(MAX_META_SIZE as usize), MAX_META_SIZE),
        ];

        for (meta, expected) in cases {
            let call = pallet_cps::Call::<WeightTestRuntime>::set_meta {
                node_id: NodeId(0),
                meta,
            };
            let _ = call.get_dispatch_info();
            LAST_SET_META.with(|c| assert_eq!(c.get(), expected));
        }
    }

    #[test]
    fn set_payload_weight_uses_actual_byte_length() {
        let cases: [(Option<NodePayload>, u32); 4] = [
            (None, 0),
            (Some(BoundedVec::try_from(sp_std::vec![]).unwrap()), 0),
            (payload_of_len(10), 10),
            (payload_of_len(MAX_PAYLOAD_SIZE as usize), MAX_PAYLOAD_SIZE),
        ];

        for (payload, expected) in cases {
            let call = pallet_cps::Call::<WeightTestRuntime>::set_payload {
                node_id: NodeId(0),
                payload,
            };
            let _ = call.get_dispatch_info();
            LAST_SET_PAYLOAD.with(|c| assert_eq!(c.get(), expected));
        }
    }

    #[test]
    fn create_node_weight_uses_actual_byte_lengths() {
        type Case = (Option<NodeMeta>, Option<NodePayload>, (u32, u32));
        let cases: [Case; 4] = [
            (None, None, (0, 0)),
            (meta_of_len(10), None, (10, 0)),
            (None, payload_of_len(10), (0, 10)),
            (
                meta_of_len(MAX_META_SIZE as usize),
                payload_of_len(MAX_PAYLOAD_SIZE as usize),
                (MAX_META_SIZE, MAX_PAYLOAD_SIZE),
            ),
        ];

        for (meta, payload, expected) in cases {
            let call = pallet_cps::Call::<WeightTestRuntime>::create_node {
                parent_id: None,
                meta,
                payload,
            };
            let _ = call.get_dispatch_info();
            LAST_CREATE_NODE.with(|c| assert_eq!(c.get(), expected));
        }
    }
}
