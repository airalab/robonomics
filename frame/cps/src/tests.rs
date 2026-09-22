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
use frame_support::{
    assert_noop, assert_ok, derive_impl, pallet_prelude::Weight, traits::Hooks, BoundedVec,
};
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

fn data(bytes: &[u8]) -> NodeData {
    BoundedVec::try_from(bytes.to_vec()).unwrap()
}

fn active_scope_id(node_id: NodeId) -> Option<ScopeId> {
    Cps::active_scope(node_id).map(|(scope_id, _)| scope_id)
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
        Cps::active_scope(expected_root),
        Some((expected_id, expected_owner))
    );
}

fn assert_cleanup_state(head: u64, tail: u64) {
    assert_eq!(Cps::cleanup_state(), CleanupQueueState { head, tail });
}

fn run_gc(weight: Weight) -> Weight {
    <Cps as Hooks<u64>>::on_idle(System::block_number(), weight)
}

fn run_gc_step(weight: Weight) -> Weight {
    Cps::do_gc_step(weight)
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

fn access_count(scope_id: ScopeId) -> usize {
    Access::<Runtime>::iter_prefix(scope_id).count()
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
        assert_eq!(Cps::parent_of(NodeId(0)), Some(None));
        assert_eq!(Cps::active_scope(NodeId(0)), Some((ScopeId(0), account)));
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

        assert_eq!(Cps::parent_of(NodeId(1)), Some(Some(NodeId(0))));
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
fn max_tree_depth_enforced() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        for i in 0..MAX_TREE_DEPTH {
            assert_ok!(Cps::create_node(
                RuntimeOrigin::signed(account),
                Some(NodeId(i as u64)),
                None,
                None
            ));
        }

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
        assert_eq!(Cps::parent_of(NodeId(1)), None);
        assert!(Cps::nodes_by_parent(NodeId(0)).is_empty());
    });
}

#[test]
fn delete_root_node_removes_active_scope_and_enqueues_cleanup() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(1), NodeId(0)));

        assert_eq!(Cps::parent_of(NodeId(0)), None);
        assert_eq!(Cps::active_scope(NodeId(0)), None);
        assert_cleanup_state(0, 1);
        assert_eq!(Cps::cleanup_queue(0), Some(ScopeId(0)));
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
        assert_eq!(Cps::active_scope(NodeId(0)), Some((ScopeId(0), 1)));
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
        let new_scope = Cps::active_scope(NodeId(1)).unwrap();
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

        let new_scope = Cps::active_scope(root).unwrap();
        assert_ne!(new_scope.0, old_scope);
        assert_eq!(new_scope.1, 2);
        assert_cleanup_state(0, 1);
        assert_eq!(Cps::cleanup_queue(0), Some(old_scope));
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

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(2), japan));

        let new_scope = Cps::active_scope(japan).unwrap();
        assert_ne!(new_scope.0, old_scope);
        assert_eq!(new_scope.1, 2);
        assert!(Access::<Runtime>::get(old_scope, (japan, 2)).contains(Capability::Write));
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
fn create_scope_capability_rejected_when_node_is_not_active_scope_root() {
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
        assert_noop!(
            Cps::create_scope(RuntimeOrigin::signed(2), child),
            Error::<Runtime>::AccessDenied
        );

        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), child));
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            child,
            2,
            Capability::CreateScope,
            GrantMode::Node,
        ));
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(2), child));
    });
}

#[test]
fn create_scope_subtree_capability_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_noop!(
            Cps::grant_access(
                RuntimeOrigin::signed(1),
                NodeId(0),
                2,
                Capability::CreateScope,
                GrantMode::Subtree,
            ),
            Error::<Runtime>::BadArguments
        );
    });
}

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
        let japan_scope = active_scope_id(japan).unwrap();
        assert_scope(japan, japan_scope, japan, 1);

        assert_ok!(Cps::delete_scope(RuntimeOrigin::signed(1), japan));
        assert_eq!(Cps::active_scope(japan), None);
        assert_scope(japan, ScopeId(0), root, 1);
        assert_eq!(Cps::cleanup_queue(0), Some(japan_scope));
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

        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            japan,
            2,
            Capability::Write,
            GrantMode::Subtree,
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

        assert_ok!(Cps::delete_scope(RuntimeOrigin::signed(1), japan));
        assert_eq!(Cps::active_scope(university), Some((uni_scope, 1)));
        assert_scope(university, uni_scope, university, 1);
        assert_scope(japan, ScopeId(0), global, 1);
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
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            2,
            Capability::Write,
            GrantMode::Subtree,
        ));

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
            Cps::delete_scope(RuntimeOrigin::none(), NodeId(0)),
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
                RuntimeEvent::Cps(Event::CleanupEnqueued(ScopeId(0))),
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
fn replacing_a_scope_enqueues_the_stale_scope() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        let old_scope = active_scope_id(root).unwrap();

        assert_cleanup_state(0, 0);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));

        assert_cleanup_state(0, 1);
        assert_eq!(Cps::cleanup_queue(0), Some(old_scope));
        assert_eq!(Cps::current_cleanup(), None);
    });
}

#[test]
fn delete_scope_enqueues_the_stale_scope() {
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

        assert_ok!(Cps::delete_scope(RuntimeOrigin::signed(1), japan));
        assert_cleanup_state(0, 1);
        assert_eq!(Cps::cleanup_queue(0), Some(scope_id));
    });
}

#[test]
fn creating_a_fresh_root_scope_never_enqueues_anything() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_cleanup_state(0, 0);
        assert_eq!(Cps::current_cleanup(), None);
    });
}

#[test]
fn access_survives_physically_but_is_logically_invalid_before_gc_runs() {
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
        assert!(Access::<Runtime>::get(old_scope, (root, 2u64)).contains(Capability::Write));
        assert_noop!(
            Cps::set_meta(RuntimeOrigin::signed(2), root, Some(data(b"x"))),
            Error::<Runtime>::AccessDenied
        );
    });
}

#[test]
fn gc_processes_multiple_stale_scopes_in_single_on_idle_call() {
    let mut ext = new_test_ext();
    let (scope_a, scope_b, root) = ext.execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        let scope_a = active_scope_id(root).unwrap();
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            10,
            Capability::Write,
            GrantMode::Node,
        ));
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));

        let scope_b = active_scope_id(root).unwrap();
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            root,
            11,
            Capability::Write,
            GrantMode::Node,
        ));
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));
        (scope_a, scope_b, root)
    });
    ext.commit_all().unwrap();

    ext.execute_with(|| {
        assert_eq!(access_count(scope_a), 1);
        assert_eq!(access_count(scope_b), 1);
        assert_cleanup_state(0, 2);
    });

    let consumed = ext.execute_with(|| run_gc(Weight::MAX));
    ext.commit_all().unwrap();

    ext.execute_with(|| {
        assert!(consumed.any_gt(Weight::zero()));
        assert_eq!(access_count(scope_a), 0);
        assert_eq!(access_count(scope_b), 0);
        assert_cleanup_state(2, 2);
        assert_eq!(Cps::current_cleanup(), None);
        assert_eq!(Cps::active_scope(root), Some((ScopeId(2), 1)));

        let completed: Vec<_> = System::events()
            .into_iter()
            .filter_map(|record| match record.event {
                RuntimeEvent::Cps(Event::CleanupCompleted(scope_id)) => Some(scope_id),
                _ => None,
            })
            .collect();
        assert!(completed.contains(&scope_a));
        assert!(completed.contains(&scope_b));
    });
}

#[test]
fn gc_weight_accounting_empty_queue_and_insufficient_weight() {
    new_test_ext().execute_with(|| {
        let empty_budget = Weight::from_parts(123, 0);
        let empty_used = run_gc(empty_budget);
        assert_eq!(empty_used, Weight::zero());
        assert!(!empty_used.any_gt(empty_budget));
        assert_cleanup_state(0, 0);
        assert_eq!(Cps::current_cleanup(), None);

        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), NodeId(0)));
        assert_cleanup_state(0, 1);

        let insufficient_budget = Weight::zero();
        let used = run_gc_step(insufficient_budget);
        assert_eq!(used, Weight::zero());
        assert!(!used.any_gt(insufficient_budget));
        assert_cleanup_state(0, 1);
        assert_eq!(Cps::cleanup_queue(0), Some(ScopeId(0)));
        assert_eq!(Cps::current_cleanup(), None);
    });
}

#[test]
fn current_cleanup_persists_and_resumes_across_multiple_gc_steps() {
    let mut ext = new_test_ext();
    let old_scope = ext.execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let root = NodeId(0);
        let old_scope = active_scope_id(root).unwrap();
        grant_many(1, root, 100, (MAX_GC_BATCH as u64) * 2 + 1);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));
        old_scope
    });
    ext.commit_all().unwrap();

    ext.execute_with(|| {
        assert_eq!(access_count(old_scope), (MAX_GC_BATCH as usize) * 2 + 1);
        assert_eq!(Cps::current_cleanup(), None);
    });

    let used1 = ext.execute_with(|| run_gc_step(Weight::MAX));
    ext.commit_all().unwrap();
    ext.execute_with(|| {
        assert!(used1.any_gt(Weight::zero()));
        assert_eq!(access_count(old_scope), MAX_GC_BATCH as usize + 1);
        let (scope_id, cursor) = Cps::current_cleanup().expect("cleanup should be parked");
        assert_eq!(scope_id, old_scope);
        assert!(cursor.is_some());
    });

    let used2 = ext.execute_with(|| run_gc_step(Weight::MAX));
    ext.commit_all().unwrap();
    ext.execute_with(|| {
        assert!(used2.any_gt(Weight::zero()));
        assert_eq!(access_count(old_scope), 1);
        let (scope_id, cursor) = Cps::current_cleanup().expect("cleanup should continue");
        assert_eq!(scope_id, old_scope);
        assert!(cursor.is_some());
    });

    let used3 = ext.execute_with(|| run_gc_step(Weight::MAX));
    ext.commit_all().unwrap();
    ext.execute_with(|| {
        assert!(used3.any_gt(Weight::zero()));
        assert_eq!(access_count(old_scope), 0);
        assert_eq!(Cps::current_cleanup(), None);
        assert_cleanup_state(1, 1);
    });
}

#[test]
fn gc_never_touches_nested_scope_state() {
    let mut ext = new_test_ext();
    let nested_scope = ext.execute_with(|| {
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
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));
        nested_scope
    });
    ext.commit_all().unwrap();

    ext.execute_with(|| {
        assert_ok!(Cps::grant_access(
            RuntimeOrigin::signed(1),
            NodeId(1),
            3,
            Capability::Write,
            GrantMode::Node,
        ));
    });
    ext.commit_all().unwrap();

    ext.execute_with(|| {
        assert_eq!(Cps::cleanup_queue(0), Some(ScopeId(0)));
    });
    ext.execute_with(|| {
        run_gc(Weight::MAX);
    });
    ext.commit_all().unwrap();

    ext.execute_with(|| {
        assert_eq!(Cps::active_scope(NodeId(1)), Some((nested_scope, 1)));
        assert!(Access::<Runtime>::get(nested_scope, (NodeId(1), 2u64)).contains(Capability::Write));
        assert!(Access::<Runtime>::get(nested_scope, (NodeId(1), 3u64)).contains(Capability::Write));
    });
}

#[test]
fn queue_invariant_enqueued_scope_id_never_becomes_active_again() {
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

        assert_cleanup_state(0, 2);
        assert_eq!(Cps::cleanup_queue(0), Some(stale_a));
        assert_eq!(Cps::cleanup_queue(1), Some(stale_b));
        assert_ne!(stale_a, stale_b);
        assert_ne!(stale_a, current_a);
        assert_ne!(stale_a, current_b);
        assert_ne!(stale_b, current_a);
        assert_ne!(stale_b, current_b);

        let active_ids: Vec<_> = ActiveScope::<Runtime>::iter()
            .map(|(_, (id, _))| id)
            .collect();
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
fn scope_id_is_never_reused_after_gc_completes() {
    let mut ext = new_test_ext();
    let (root, old_scope, new_scope) = ext.execute_with(|| {
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
        (root, old_scope, new_scope)
    });
    ext.commit_all().unwrap();

    ext.execute_with(|| {
        run_gc(Weight::MAX);
    });
    ext.commit_all().unwrap();

    ext.execute_with(|| {
        assert_eq!(access_count(old_scope), 0);
        assert_eq!(Cps::current_cleanup(), None);
        assert_ok!(Cps::create_scope(RuntimeOrigin::signed(1), root));
        let newer_scope = active_scope_id(root).unwrap();
        assert_ne!(newer_scope, old_scope);
        assert_ne!(newer_scope, new_scope);
        assert!(u64::from(newer_scope) > u64::from(old_scope));
    });
}
