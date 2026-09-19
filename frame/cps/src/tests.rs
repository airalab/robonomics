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

fn data(bytes: &[u8]) -> NodeData {
    BoundedVec::try_from(bytes.to_vec()).unwrap()
}

/// Assert that `node_id` resolves to the Scope `expected_id`, rooted at
/// `expected_root` and owned by `expected_owner`. `ScopeRoot` / `ScopeOwner`
/// are looked up directly, mirroring how any external caller (e.g. the
/// `CpsApi` runtime API consumer) would use the plain `ScopeId` returned by
/// [`Cps::resolve_scope`].
fn assert_scope(node_id: NodeId, expected_id: ScopeId, expected_root: NodeId, expected_owner: u64) {
    assert_eq!(Cps::resolve_scope(node_id), Ok(expected_id));
    assert_eq!(Cps::scope_root(expected_id), Some(expected_root));
    assert_eq!(Cps::scope_owner(expected_id), Some(expected_owner));
}

// ---------------------------------------------------------------------------
// create_node
// ---------------------------------------------------------------------------

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
        assert_eq!(Cps::parent_of(NodeId(0)), Some(None));

        // Root creation allocates a fresh Scope owned by the creator.
        assert_eq!(Cps::active_scope(NodeId(0)), Some(ScopeId(0)));
        assert_eq!(Cps::scope_root(ScopeId(0)), Some(NodeId(0)));
        assert_eq!(Cps::scope_owner(ScopeId(0)), Some(account));
        assert_eq!(Cps::next_scope_id(), ScopeId(1));
        assert_scope(NodeId(0), ScopeId(0), NodeId(0), account);

        // Root is indexed
        assert_eq!(Cps::root_nodes().len(), 1);
        assert_eq!(Cps::root_nodes()[0], NodeId(0));
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

        assert_eq!(Cps::parent_of(NodeId(1)), Some(Some(NodeId(0))));
        // The child inherits the parent's Scope: no explicit entry of its own.
        assert_eq!(Cps::active_scope(NodeId(1)), None);
        assert_scope(NodeId(1), ScopeId(0), NodeId(0), account);

        assert_eq!(Cps::nodes_by_parent(NodeId(0)).len(), 1);
        assert_eq!(Cps::nodes_by_parent(NodeId(0))[0], NodeId(1));
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
fn create_node_without_data_stores_nothing() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

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
fn create_child_without_access_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));

        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(2), Some(NodeId(0)), None, None),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn max_tree_depth_enforced() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        // Build a chain 0 -> 1 -> 2 -> ... down to depth `MAX_TREE_DEPTH`.
        for i in 0..MAX_TREE_DEPTH {
            assert_ok!(Cps::create_node(
                RuntimeOrigin::signed(account),
                Some(NodeId(i as u64)),
                None,
                None
            ));
        }

        // The last node created is at depth `MAX_TREE_DEPTH`; adding one more
        // child would put the new node past the limit.
        let deepest = NodeId(MAX_TREE_DEPTH as u64);
        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(account), Some(deepest), None, None),
            Error::<Runtime>::MaxDepthExceeded
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
        let children = Cps::nodes_by_parent(NodeId(0));
        assert_eq!(children.len(), MAX_CHILDREN_PER_NODE as usize);
        assert!(!children.contains(&NodeId(1)));
        assert_eq!(children.last(), Some(&next));
    });
}

#[test]
fn max_root_nodes_enforced() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        for _ in 0..MAX_ROOT_NODES {
            assert_ok!(Cps::create_node(
                RuntimeOrigin::signed(account),
                None,
                None,
                None
            ));
        }

        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(account), None, None, None),
            Error::<Runtime>::TooManyRootNodes
        );
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(account), NodeId(0)));
        let next = Cps::next_node_id();
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(2), None, None, None));
        let roots = Cps::root_nodes();
        assert_eq!(roots.len(), MAX_ROOT_NODES as usize);
        assert!(!roots.contains(&NodeId(0)));
        assert_eq!(roots.last(), Some(&next));
    });
}

// ---------------------------------------------------------------------------
// set_meta / set_payload
// ---------------------------------------------------------------------------

#[test]
fn set_meta_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        let meta = Some(data(b"updated"));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(account),
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
fn set_meta_not_found_fails() {
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
        let account = 1u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        let payload = Some(data(b"updated"));
        assert_ok!(Cps::set_payload(
            RuntimeOrigin::signed(account),
            NodeId(0),
            payload.clone()
        ));

        assert_eq!(Cps::payload_of(NodeId(0)), payload);
    });
}

#[test]
fn clear_meta_and_payload_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            Some(data(b"meta")),
            Some(data(b"payload"))
        ));

        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(account),
            NodeId(0),
            None
        ));
        assert_ok!(Cps::set_payload(
            RuntimeOrigin::signed(account),
            NodeId(0),
            None
        ));

        assert_eq!(Cps::meta_of(NodeId(0)), None);
        assert_eq!(Cps::payload_of(NodeId(0)), None);
    });
}

// ---------------------------------------------------------------------------
// delete_node
// ---------------------------------------------------------------------------

#[test]
fn delete_leaf_node_works() {
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

        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(account), NodeId(1)));

        assert_eq!(Cps::parent_of(NodeId(1)), None);
        assert!(Cps::nodes_by_parent(NodeId(0)).is_empty());
    });
}

#[test]
fn delete_root_node_removes_active_scope() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(account), NodeId(0)));

        assert_eq!(Cps::parent_of(NodeId(0)), None);
        assert!(Cps::root_nodes().is_empty());
        assert_eq!(Cps::active_scope(NodeId(0)), None);
    });
}

#[test]
fn delete_node_with_children_fails() {
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

        assert_noop!(
            Cps::delete_node(RuntimeOrigin::signed(account), NodeId(0)),
            Error::<Runtime>::NodeHasChildren
        );
    });
}

#[test]
fn delete_node_without_access_fails() {
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

// ---------------------------------------------------------------------------
// Scope resolution & inheritance
// ---------------------------------------------------------------------------

#[test]
fn nested_scope_inheritance_works() {
    new_test_ext().execute_with(|| {
        let a = 1u64;
        let b = 2u64;

        // Global (root, Scope #0 / A)
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(a), None, None, None));
        let global = NodeId(0);

        // Japan under Global, still owned by A.
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(a),
            Some(global),
            None,
            None
        ));
        let japan = NodeId(1);

        // University under Japan, carved into its own Scope owned by B.
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(a),
            Some(japan),
            None,
            None
        ));
        let university = NodeId(2);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(a), university));
        // Ownership of the new Scope always belongs to the caller of
        // create_scope; grant B access to actually own it in this scenario.
        let uni_scope = Cps::active_scope(university).unwrap();
        assert_eq!(Cps::scope_owner(uni_scope), Some(a));

        assert_scope(global, ScopeId(0), global, a);
        assert_scope(japan, ScopeId(0), global, a);
        assert_scope(university, uni_scope, university, a);

        // Sensor under University inherits University's Scope, not Global's.
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(a),
            Some(university),
            None,
            None
        ));
        let sensor = NodeId(3);
        assert_scope(sensor, uni_scope, university, a);
        let _ = b; // silence unused warning if scenario changes
    });
}

#[test]
fn same_owner_nested_scope_is_still_a_hard_boundary() {
    new_test_ext().execute_with(|| {
        let a = 1u64;

        assert_ok!(Cps::create_node(RuntimeOrigin::signed(a), None, None, None));
        let root = NodeId(0);
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(a),
            Some(root),
            None,
            None
        ));
        let child = NodeId(1);

        // A carves out a new Scope on `child`, still owned by themselves.
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(a), child));
        let child_scope = Cps::active_scope(child).unwrap();
        let root_scope = Cps::active_scope(root).unwrap();
        assert_ne!(child_scope, root_scope);

        // Access granted at the root does not propagate into the nested Scope,
        // even though both Scopes share the same owner.
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(a),
            root,
            2,
            Capability::Write,
            true
        ));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(2),
            Some(root),
            None,
            None
        ));
        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(2), Some(child), None, None),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn resolve_scope_missing_node_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Cps::resolve_scope(NodeId(0)).map_err(|e| e),
            Error::<Runtime>::NodeNotFound
        );
    });
}

// ---------------------------------------------------------------------------
// create_scope / Scope replacement
// ---------------------------------------------------------------------------

#[test]
fn root_scope_created_on_root_creation() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_eq!(Cps::active_scope(NodeId(0)), Some(ScopeId(0)));
        assert_eq!(Cps::scope_owner(ScopeId(0)), Some(1));
        assert_eq!(Cps::scope_root(ScopeId(0)), Some(NodeId(0)));
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

        let new_scope = Cps::active_scope(NodeId(1)).unwrap();
        assert_ne!(new_scope, ScopeId(0));
        assert_eq!(Cps::scope_owner(new_scope), Some(1));
        assert_eq!(Cps::scope_root(new_scope), Some(NodeId(1)));
    });
}

#[test]
fn scope_replacement_allocates_fresh_id_and_invalidates_old_access() {
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
        let old_scope = Cps::active_scope(japan).unwrap();
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            japan,
            2,
            Capability::Write,
            true
        ));

        // B replaces the Scope via a delegated CreateScope grant.
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            japan,
            2,
            Capability::CreateScope,
            false
        ));
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(2), japan));

        let new_scope = Cps::active_scope(japan).unwrap();
        assert_ne!(new_scope, old_scope);
        assert_eq!(Cps::scope_owner(new_scope), Some(2));

        // The old Scope's Access entries still exist physically (GC deferred)
        // but are no longer consulted: the old Write grant for account 1
        // (the previous owner) is not valid under the new Scope.
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
        let first = Cps::active_scope(node).unwrap();
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), node));
        let second = Cps::active_scope(node).unwrap();
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), node));
        let third = Cps::active_scope(node).unwrap();

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

        // Grant a non-inherited CreateScope at the root; it must never
        // authorize create_scope on a descendant - CreateScope never
        // propagates through descendants.
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::CreateScope,
            false
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

        // A full, inherited `Write` grant is a data capability only; it must
        // never let the grantee create/replace a Scope.
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            true
        ));

        assert_noop!(
            Cps::create_scope(RuntimeOrigin::signed(2), root),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn create_scope_inherited_capability_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);

        // CreateScope grants must be non-inherited: attempting to grant one
        // with inherited=true is rejected outright at `grant_access`.
        assert_noop!(
            Cps::grant_access(
                RuntimeOrigin::signed(1),
                root,
                2,
                Capability::CreateScope,
                true
            ),
            Error::<Runtime>::BadArguments
        );
    });
}

// ---------------------------------------------------------------------------
// delete_scope
// ---------------------------------------------------------------------------

#[test]
fn delete_scope_falls_back_to_parent_scope() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        let root = NodeId(0);
        let japan = NodeId(1);

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), japan));
        let japan_scope = Cps::active_scope(japan).unwrap();
        assert_scope(japan, japan_scope, japan, 1);

        assert_ok!(Cps::delete_scope(RuntimeOrigin::signed(1), japan));

        assert_eq!(Cps::active_scope(japan), None);
        assert_scope(japan, ScopeId(0), root, 1);
    });
}

#[test]
fn root_scope_deletion_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));

        assert_noop!(
            Cps::delete_scope(RuntimeOrigin::signed(1), NodeId(0)),
            Error::<Runtime>::CannotDeleteRootScope
        );
    });
}

#[test]
fn delete_scope_requires_owner_not_inherited_access() {
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

        // Even full Write Access does not allow deleting the Scope.
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            japan,
            2,
            Capability::Write,
            true
        ));
        assert_noop!(
            Cps::delete_scope(RuntimeOrigin::signed(2), japan),
            Error::<Runtime>::NotScopeOwner
        );
    });
}

#[test]
fn nested_scope_preserved_after_parent_scope_deletion() {
    new_test_ext().execute_with(|| {
        // Global(#0/A) -> Japan(#1/B) -> University(#2/C)
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
        let uni_scope = Cps::active_scope(university).unwrap();

        assert_ok!(Cps::delete_scope(RuntimeOrigin::signed(1), japan));

        // University's Scope is untouched.
        assert_eq!(Cps::active_scope(university), Some(uni_scope));
        assert_scope(university, uni_scope, university, 1);
        // Japan itself now falls back to Global's Scope.
        assert_scope(japan, ScopeId(0), global, 1);
    });
}

// ---------------------------------------------------------------------------
// Access: exact-node vs inherited, Scope boundary
// ---------------------------------------------------------------------------

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
            false
        ));

        // Exact node: works.
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(2),
            root,
            Some(data(b"x"))
        ));
        // Does not propagate to a descendant.
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
            true
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

        // Inherited Write Access granted at `root` never crosses into the
        // nested Scope rooted at `nested_root`.
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            true
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

        assert!(Cps::access(ScopeId(0), (NodeId(0), 1u64, Capability::Write)).is_none());
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(1),
            NodeId(1),
            Some(data(b"x"))
        ));
    });
}

// ---------------------------------------------------------------------------
// grant_access / revoke_access
// ---------------------------------------------------------------------------

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
            true
        ));
        assert_eq!(
            Cps::access(ScopeId(0), (root, 2u64, Capability::Write)),
            Some(true)
        );
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(2),
            root,
            Some(data(b"x"))
        ));

        assert_ok!(Cps::revoke_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write
        ));
        assert_eq!(
            Cps::access(ScopeId(0), (root, 2u64, Capability::Write)),
            None
        );
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
                true
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
            true
        ));

        assert_noop!(
            Cps::revoke_access(RuntimeOrigin::signed(2), NodeId(0), 2, Capability::Write),
            Error::<Runtime>::NotScopeOwner
        );
    });
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

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
            Cps::delete_scope(RuntimeOrigin::none(), NodeId(0)),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            Cps::grant_access(RuntimeOrigin::none(), NodeId(0), 1, Capability::Write, true),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            Cps::revoke_access(RuntimeOrigin::none(), NodeId(0), 1, Capability::Write),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn data_limits_and_empty_values_are_preserved() {
    new_test_ext().execute_with(|| {
        let max = data(&vec![7u8; MAX_DATA_SIZE as usize]);
        let empty = data(b"");

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            Some(max.clone()),
            Some(empty.clone())
        ));

        assert_eq!(Cps::meta_of(NodeId(0)), Some(max));
        assert_eq!(Cps::payload_of(NodeId(0)), Some(empty));
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
            true
        ));
        assert_ok!(Cps::revoke_access(
            RuntimeOrigin::signed(1),
            NodeId(0),
            2,
            Capability::Write
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
                    true
                )),
                RuntimeEvent::Cps(Event::AccessRevoked(
                    ScopeId(0),
                    NodeId(0),
                    2,
                    Capability::Write
                )),
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

        assert_eq!(Cps::parent_of(NodeId(0)), None);
        assert_eq!(Cps::meta_of(NodeId(0)), None);
        assert_eq!(Cps::payload_of(NodeId(0)), None);
        assert_eq!(Cps::active_scope(NodeId(0)), None);

        // Node IDs are never reused, even though the ID's node was deleted.
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_eq!(Cps::next_node_id(), NodeId(2));
        assert!(!Cps::root_nodes().contains(&NodeId(0)));
        assert_eq!(Cps::root_nodes(), vec![NodeId(1)]);
    });
}

// ---------------------------------------------------------------------------
// has_capability
// ---------------------------------------------------------------------------

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

        // Scope owner has implicit Write everywhere in the Scope.
        assert!(Cps::has_capability(root, &1, Capability::Write));
        assert!(Cps::has_capability(child, &1, Capability::Write));

        // Non-owner without Access has no Write authority.
        assert!(!Cps::has_capability(root, &2, Capability::Write));

        // Exact-node grant applies only to that node.
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            false
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

        // A full, inherited Write grant never authorizes CreateScope.
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            true
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
