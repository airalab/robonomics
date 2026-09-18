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
use std::cell::RefCell;

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

thread_local! {
    static PAYLOAD_CALLBACK_LOG: RefCell<Vec<(NodeId, Option<NodeData>, Option<NodeData>)>> =
        RefCell::new(Vec::new());
}

/// Test-only [`OnPayloadSet`] implementation that records every invocation so
/// tests can assert the callback fires with the expected arguments.
pub struct RecordingPayloadHandler;
impl OnPayloadSet<u64> for RecordingPayloadHandler {
    fn on_payload_set(node_id: NodeId, meta: Option<NodeData>, payload: Option<NodeData>) {
        // The hook must observe the updated storage, not the previous payload.
        assert_eq!(Cps::meta_of(node_id), meta);
        assert_eq!(Cps::payload_of(node_id), payload);
        PAYLOAD_CALLBACK_LOG.with(|log| log.borrow_mut().push((node_id, meta, payload)));
    }
}

fn payload_callback_log() -> Vec<(NodeId, Option<NodeData>, Option<NodeData>)> {
    PAYLOAD_CALLBACK_LOG.with(|log| log.borrow().clone())
}

impl pallet_cps::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnPayloadSet = RecordingPayloadHandler;
    type WeightInfo = weights::TestWeightInfo;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    PAYLOAD_CALLBACK_LOG.with(|log| log.borrow_mut().clear());

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn data(bytes: &[u8]) -> NodeData {
    BoundedVec::try_from(bytes.to_vec()).unwrap()
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
        assert_eq!(Cps::ownership_of(NodeId(0)), Some(account));
        assert_eq!(Cps::resolve_ownership(NodeId(0)), Ok((NodeId(0), account)));

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
        // The child inherits ownership: no explicit entry of its own.
        assert_eq!(Cps::ownership_of(NodeId(1)), None);
        assert_eq!(Cps::resolve_ownership(NodeId(1)), Ok((NodeId(0), account)));

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
fn create_child_non_owner_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));

        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(2), Some(NodeId(0)), None, None),
            Error::<Runtime>::NotNodeOwner
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
        assert_eq!(Cps::resolve_ownership(deepest), Ok((NodeId(0), account)));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(account),
            deepest,
            Some(data(b"at limit"))
        ));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(account),
            deepest,
            2
        ));
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(2), deepest));
        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(2), Some(deepest), None, None),
            Error::<Runtime>::MaxDepthExceeded
        );
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(2), deepest));
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
fn set_meta_non_owner_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));

        assert_noop!(
            Cps::set_meta(RuntimeOrigin::signed(2), NodeId(0), Some(data(b"x"))),
            Error::<Runtime>::NotNodeOwner
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

#[test]
fn set_payload_invokes_callback() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            Some(data(b"meta")),
            None
        ));

        let payload = Some(data(b"payload"));
        assert_ok!(Cps::set_payload(
            RuntimeOrigin::signed(account),
            NodeId(0),
            payload.clone()
        ));

        assert_eq!(
            payload_callback_log(),
            vec![(NodeId(0), Some(data(b"meta")), payload)]
        );
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
fn delete_root_node_works() {
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
        assert_eq!(Cps::ownership_of(NodeId(0)), None);
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
fn delete_node_non_owner_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));

        assert_noop!(
            Cps::delete_node(RuntimeOrigin::signed(2), NodeId(0)),
            Error::<Runtime>::NotNodeOwner
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
fn delete_node_clears_pending_transfer() {
    new_test_ext().execute_with(|| {
        let owner = 1u64;
        let proposed = 2u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(owner),
            None,
            None,
            None
        ));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(owner),
            NodeId(0),
            proposed
        ));

        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(owner), NodeId(0)));

        assert_eq!(Cps::pending_ownership_transfer(NodeId(0)), None);
    });
}

// ---------------------------------------------------------------------------
// Ownership inheritance & boundaries
// ---------------------------------------------------------------------------

#[test]
fn nested_ownership_inheritance_works() {
    new_test_ext().execute_with(|| {
        let property_manager = 1u64;
        let tenant_corp = 2u64;

        // Building-A (root, owned by PropertyManager)
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(property_manager),
            None,
            None,
            None
        ));
        // Floor-3, child of Building-A
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(property_manager),
            Some(NodeId(0)),
            None,
            None
        ));
        // Establish a new Ownership boundary on Floor-3, transferred to TenantCorp
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(property_manager),
            NodeId(1),
            tenant_corp
        ));
        assert_ok!(Cps::accept_ownership(
            RuntimeOrigin::signed(tenant_corp),
            NodeId(1)
        ));

        // HVAC-Unit-07, child of Floor-3 - now owned by TenantCorp
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(tenant_corp),
            Some(NodeId(1)),
            None,
            None
        ));
        // Thermostat-142, child of HVAC-Unit-07 - still TenantCorp's boundary
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(tenant_corp),
            Some(NodeId(2)),
            None,
            None
        ));

        assert_eq!(
            Cps::resolve_ownership(NodeId(2)),
            Ok((NodeId(1), tenant_corp))
        );
        assert_eq!(
            Cps::resolve_ownership(NodeId(3)),
            Ok((NodeId(1), tenant_corp))
        );
    });
}

#[test]
fn ownership_boundary_isolates_ancestor() {
    new_test_ext().execute_with(|| {
        let property_manager = 1u64;
        let tenant_corp = 2u64;

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(property_manager),
            None,
            None,
            None
        ));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(property_manager),
            Some(NodeId(0)),
            None,
            None
        ));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(property_manager),
            NodeId(1),
            tenant_corp
        ));
        assert_ok!(Cps::accept_ownership(
            RuntimeOrigin::signed(tenant_corp),
            NodeId(1)
        ));

        // PropertyManager no longer has implicit rights inside TenantCorp's boundary.
        assert_noop!(
            Cps::set_meta(
                RuntimeOrigin::signed(property_manager),
                NodeId(1),
                Some(data(b"x"))
            ),
            Error::<Runtime>::NotNodeOwner
        );
        assert_noop!(
            Cps::create_node(
                RuntimeOrigin::signed(property_manager),
                Some(NodeId(1)),
                None,
                None
            ),
            Error::<Runtime>::NotNodeOwner
        );

        // TenantCorp has no implicit rights over PropertyManager's other branches.
        assert_noop!(
            Cps::set_meta(
                RuntimeOrigin::signed(tenant_corp),
                NodeId(0),
                Some(data(b"x"))
            ),
            Error::<Runtime>::NotNodeOwner
        );
    });
}

// ---------------------------------------------------------------------------
// transfer_ownership / accept_ownership
// ---------------------------------------------------------------------------

#[test]
fn transfer_then_accept_ownership_works() {
    new_test_ext().execute_with(|| {
        let owner = 1u64;
        let new_owner = 2u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(owner),
            None,
            None,
            None
        ));

        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(owner),
            NodeId(0),
            new_owner
        ));
        assert_eq!(
            Cps::pending_ownership_transfer(NodeId(0)),
            Some(PendingTransfer {
                proposed_owner: new_owner,
                boundary: NodeId(0),
                generation: 0,
            })
        );
        // Not yet transferred.
        assert_eq!(Cps::resolve_ownership(NodeId(0)), Ok((NodeId(0), owner)));

        assert_ok!(Cps::accept_ownership(
            RuntimeOrigin::signed(new_owner),
            NodeId(0)
        ));

        assert_eq!(Cps::pending_ownership_transfer(NodeId(0)), None);
        assert_eq!(
            Cps::resolve_ownership(NodeId(0)),
            Ok((NodeId(0), new_owner))
        );
    });
}

#[test]
fn transfer_ownership_non_owner_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));

        assert_noop!(
            Cps::transfer_ownership(RuntimeOrigin::signed(2), NodeId(0), 3),
            Error::<Runtime>::NotNodeOwner
        );
    });
}

#[test]
fn accept_ownership_without_pending_transfer_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));

        assert_noop!(
            Cps::accept_ownership(RuntimeOrigin::signed(2), NodeId(0)),
            Error::<Runtime>::NoPendingOwnershipTransfer
        );
    });
}

#[test]
fn accept_ownership_wrong_account_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(0),
            2
        ));

        assert_noop!(
            Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(0)),
            Error::<Runtime>::NotProposedOwner
        );
    });
}

#[test]
fn self_transfer_establishes_new_boundary_without_changing_owner() {
    new_test_ext().execute_with(|| {
        let owner = 1u64;
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(owner),
            None,
            None,
            None
        ));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(owner),
            Some(NodeId(0)),
            None,
            None
        ));

        // Node 1 currently inherits from node 0. Self-transfer to carve out an
        // explicit boundary without changing the effective owner.
        assert_eq!(Cps::ownership_of(NodeId(1)), None);
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(owner),
            NodeId(1),
            owner
        ));
        assert_ok!(Cps::accept_ownership(
            RuntimeOrigin::signed(owner),
            NodeId(1)
        ));

        assert_eq!(Cps::ownership_of(NodeId(1)), Some(owner));
        assert_eq!(Cps::resolve_ownership(NodeId(1)), Ok((NodeId(1), owner)));
    });
}

#[test]
fn resolve_ownership_not_found_fails_for_missing_node() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Cps::resolve_ownership(NodeId(0)),
            Error::<Runtime>::NodeNotFound
        );
    });
}

// ---------------------------------------------------------------------------
// Allocation and ownership regressions
// ---------------------------------------------------------------------------

/// Exhaustion must fail before writing indexes or overwriting an existing node.
#[test]
fn node_id_exhaustion_is_atomic() {
    new_test_ext().execute_with(|| {
        NextNodeId::<Runtime>::put(NodeId(u64::MAX - 1));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            Some(data(b"last")),
            None
        ));
        assert_eq!(Cps::next_node_id(), NodeId(u64::MAX));

        for parent in [None, Some(NodeId(u64::MAX - 1))] {
            assert_noop!(
                Cps::create_node(RuntimeOrigin::signed(1), parent, None, None),
                Error::<Runtime>::NodeIdExhausted
            );
        }
        assert_eq!(Cps::meta_of(NodeId(u64::MAX - 1)), Some(data(b"last")));
        assert_eq!(Cps::parent_of(NodeId(u64::MAX)), None);
        assert!(payload_callback_log().is_empty());
    });
}

/// A former owner cannot leave a claim on descendants sold with their ancestor.
#[test]
fn ancestor_transfer_invalidates_descendant_proposal() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(1),
            3
        ));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(0),
            2
        ));
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(2), NodeId(0)));

        assert_noop!(
            Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(1)),
            Error::<Runtime>::StaleOwnershipTransfer
        );
        assert_eq!(Cps::resolve_ownership(NodeId(1)), Ok((NodeId(0), 2)));

        // Returning the boundary to the original owner must not revive the proposal.
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(2),
            NodeId(0),
            1
        ));
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(1), NodeId(0)));
        assert_noop!(
            Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(1)),
            Error::<Runtime>::StaleOwnershipTransfer
        );

        // The current owner can explicitly authorize a fresh proposal.
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(1),
            3
        ));
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(1)));
        assert_eq!(Cps::resolve_ownership(NodeId(1)), Ok((NodeId(1), 3)));
    });
}

#[test]
fn all_extrinsics_require_signed_origin() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(0),
            2
        ));
        for origin in [RuntimeOrigin::none(), RuntimeOrigin::root()] {
            assert_noop!(
                Cps::create_node(origin.clone(), None, None, None),
                sp_runtime::DispatchError::BadOrigin
            );
            assert_noop!(
                Cps::set_meta(origin.clone(), NodeId(0), None),
                sp_runtime::DispatchError::BadOrigin
            );
            assert_noop!(
                Cps::set_payload(origin.clone(), NodeId(0), None),
                sp_runtime::DispatchError::BadOrigin
            );
            assert_noop!(
                Cps::delete_node(origin.clone(), NodeId(0)),
                sp_runtime::DispatchError::BadOrigin
            );
            assert_noop!(
                Cps::transfer_ownership(origin.clone(), NodeId(0), 3),
                sp_runtime::DispatchError::BadOrigin
            );
            assert_noop!(
                Cps::accept_ownership(origin, NodeId(0)),
                sp_runtime::DispatchError::BadOrigin
            );
        }
        assert!(payload_callback_log().is_empty());
    });
}

#[test]
fn rejected_payload_updates_do_not_invoke_callback() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            Some(data(b"meta")),
            Some(data(b"original"))
        ));
        for payload in [None, Some(data(b"replacement"))] {
            assert_noop!(
                Cps::set_payload(RuntimeOrigin::signed(2), NodeId(0), payload.clone()),
                Error::<Runtime>::NotNodeOwner
            );
            assert_noop!(
                Cps::set_payload(RuntimeOrigin::signed(1), NodeId(99), payload),
                Error::<Runtime>::NodeNotFound
            );
        }
        assert!(payload_callback_log().is_empty());
    });
}

/// Empty data is present storage; `None` removes it. Both fields accept the bound.
#[test]
fn data_limits_and_empty_values_are_preserved() {
    new_test_ext().execute_with(|| {
        let maximum = data(&vec![255; MAX_DATA_SIZE as usize]);
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            None,
            Some(maximum.clone()),
            Some(maximum.clone())
        ));
        assert_eq!(Cps::meta_of(NodeId(0)), Some(maximum.clone()));
        assert_eq!(Cps::payload_of(NodeId(0)), Some(maximum.clone()));
        assert!(payload_callback_log().is_empty());

        for value in [Some(data(b"")), Some(maximum), None, None] {
            assert_ok!(Cps::set_meta(
                RuntimeOrigin::signed(1),
                NodeId(0),
                value.clone()
            ));
            assert_ok!(Cps::set_payload(
                RuntimeOrigin::signed(1),
                NodeId(0),
                value.clone()
            ));
            assert_eq!(Cps::meta_of(NodeId(0)), value);
            assert_eq!(Cps::payload_of(NodeId(0)), value);
            assert_eq!(Meta::<Runtime>::contains_key(NodeId(0)), value.is_some());
            assert_eq!(Payload::<Runtime>::contains_key(NodeId(0)), value.is_some());
        }
        assert_eq!(payload_callback_log().len(), 4);
        assert_eq!(
            payload_callback_log().last(),
            Some(&(NodeId(0), None, None))
        );
    });
}

#[test]
fn oversized_data_is_rejected_during_scale_decoding() {
    let oversized = vec![0u8; MAX_DATA_SIZE as usize + 1];
    assert!(NodeData::try_from(oversized.clone()).is_err());
    assert!(NodeData::decode(&mut &oversized.encode()[..]).is_err());

    // Encode the wire layout with an unbounded Vec to exercise dispatch decoding.
    for (meta, payload) in [
        (Some(oversized.clone()), None),
        (None, Some(oversized.clone())),
    ] {
        let encoded = (0u8, None::<NodeId>, meta, payload).encode();
        assert!(Call::<Runtime>::decode(&mut &encoded[..]).is_err());
    }
    for call_index in [1u8, 2u8] {
        let encoded = (call_index, NodeId(0), Some(oversized.clone())).encode();
        assert!(Call::<Runtime>::decode(&mut &encoded[..]).is_err());
    }
}

#[test]
fn payload_callback_uses_latest_metadata_without_mutating_it() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        let payload = Some(data(b"payload"));
        let meta = Some(data(b"new metadata"));
        assert_ok!(Cps::set_payload(
            RuntimeOrigin::signed(1),
            NodeId(0),
            payload.clone()
        ));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(1),
            NodeId(0),
            meta.clone()
        ));
        assert_eq!(Cps::payload_of(NodeId(0)), payload);
        assert_ok!(Cps::set_payload(RuntimeOrigin::signed(1), NodeId(0), None));
        assert_eq!(Cps::meta_of(NodeId(0)), meta);
        assert_eq!(
            payload_callback_log(),
            vec![(NodeId(0), None, payload), (NodeId(0), meta, None)]
        );
    });
}

#[test]
fn successful_operations_emit_exact_events() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        assert_ok!(Cps::set_meta(RuntimeOrigin::signed(1), NodeId(1), None));
        assert_ok!(Cps::set_payload(RuntimeOrigin::signed(1), NodeId(1), None));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(1),
            2
        ));
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(2), NodeId(1)));
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(2), NodeId(1)));
        assert_eq!(
            System::events()
                .into_iter()
                .map(|record| record.event)
                .collect::<Vec<_>>(),
            vec![
                Event::NodeCreated(NodeId(0), None, 1).into(),
                Event::NodeCreated(NodeId(1), Some(NodeId(0)), 1).into(),
                Event::MetaSet(NodeId(1), 1).into(),
                Event::PayloadSet(NodeId(1), 1).into(),
                Event::OwnershipTransferProposed(NodeId(1), 1, 2).into(),
                Event::OwnershipTransferred(NodeId(1), 2).into(),
                Event::NodeDeleted(NodeId(1), 2).into(),
            ]
        );
    });
}

#[test]
fn deleting_boundary_cleans_all_attributes_without_reusing_id() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            Some(data(b"meta")),
            Some(data(b"payload"))
        ));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(1),
            2
        ));
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(2), NodeId(1)));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(2),
            NodeId(1),
            3
        ));
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(2), NodeId(1)));

        assert!(!Parent::<Runtime>::contains_key(NodeId(1)));
        assert!(!Meta::<Runtime>::contains_key(NodeId(1)));
        assert!(!Payload::<Runtime>::contains_key(NodeId(1)));
        assert!(!Ownership::<Runtime>::contains_key(NodeId(1)));
        assert!(!OwnershipGeneration::<Runtime>::contains_key(NodeId(1)));
        assert!(!PendingOwnershipTransfer::<Runtime>::contains_key(NodeId(
            1
        )));
        assert!(!NodesByParent::<Runtime>::contains_key(NodeId(1)));
        assert_eq!(Cps::nodes_by_parent(NodeId(0)).to_vec(), vec![NodeId(2)]);
        assert_eq!(Cps::root_nodes().to_vec(), vec![NodeId(0)]);
        assert_noop!(
            Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(1)),
            Error::<Runtime>::NoPendingOwnershipTransfer
        );
        assert_noop!(
            Cps::resolve_ownership(NodeId(1)),
            Error::<Runtime>::NodeNotFound
        );

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(1),
            Some(NodeId(0)),
            None,
            None
        ));
        assert_eq!(
            Cps::nodes_by_parent(NodeId(0)).to_vec(),
            vec![NodeId(2), NodeId(3)]
        );
        assert_eq!(Cps::meta_of(NodeId(3)), None);
        assert_eq!(Cps::payload_of(NodeId(3)), None);
        assert!(payload_callback_log().is_empty());
    });
}

#[test]
fn replacing_proposal_revokes_previous_recipient_and_acceptance_is_single_use() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        assert_noop!(
            Cps::transfer_ownership(RuntimeOrigin::signed(1), NodeId(99), 2),
            Error::<Runtime>::NodeNotFound
        );
        assert_noop!(
            Cps::accept_ownership(RuntimeOrigin::signed(2), NodeId(99)),
            Error::<Runtime>::NoPendingOwnershipTransfer
        );
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(0),
            2
        ));
        assert_noop!(
            Cps::set_payload(RuntimeOrigin::signed(2), NodeId(0), None),
            Error::<Runtime>::NotNodeOwner
        );
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(1),
            NodeId(0),
            Some(data(b"still owned"))
        ));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(0),
            3
        ));
        assert_noop!(
            Cps::accept_ownership(RuntimeOrigin::signed(2), NodeId(0)),
            Error::<Runtime>::NotProposedOwner
        );
        assert_eq!(
            Cps::pending_ownership_transfer(NodeId(0)),
            Some(PendingTransfer {
                proposed_owner: 3,
                boundary: NodeId(0),
                generation: 0,
            })
        );
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(0)));
        assert_eq!(Cps::pending_ownership_transfer(NodeId(0)), None);
        assert_noop!(
            Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(0)),
            Error::<Runtime>::NoPendingOwnershipTransfer
        );
    });
}

/// Even equal owners/generations cannot authorize a proposal under a different boundary.
#[test]
fn intervening_self_transfer_invalidates_descendant_proposal() {
    new_test_ext().execute_with(|| {
        for parent in [None, Some(NodeId(0)), Some(NodeId(1))] {
            assert_ok!(Cps::create_node(
                RuntimeOrigin::signed(1),
                parent,
                None,
                None
            ));
        }
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(0),
            1
        ));
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(1), NodeId(0)));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(2),
            3
        ));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(1),
            1
        ));
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(1), NodeId(1)));
        assert_eq!(Cps::ownership_generation(NodeId(0)), 1);
        assert_eq!(Cps::ownership_generation(NodeId(1)), 1);
        assert_noop!(
            Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(2)),
            Error::<Runtime>::StaleOwnershipTransfer
        );
    });
}

#[test]
fn ancestor_transfer_preserves_nested_boundary_proposals() {
    new_test_ext().execute_with(|| {
        for parent in [None, Some(NodeId(0)), Some(NodeId(1)), Some(NodeId(0))] {
            assert_ok!(Cps::create_node(
                RuntimeOrigin::signed(1),
                parent,
                None,
                None
            ));
        }
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(1),
            2
        ));
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(2), NodeId(1)));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(2),
            NodeId(2),
            3
        ));
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(0),
            4
        ));
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(4), NodeId(0)));
        assert_eq!(Cps::resolve_ownership(NodeId(3)), Ok((NodeId(0), 4)));
        assert_eq!(Cps::resolve_ownership(NodeId(2)), Ok((NodeId(1), 2)));
        for node in [NodeId(1), NodeId(2)] {
            assert_noop!(
                Cps::set_payload(RuntimeOrigin::signed(4), node, None),
                Error::<Runtime>::NotNodeOwner
            );
            assert_noop!(
                Cps::transfer_ownership(RuntimeOrigin::signed(4), node, 4),
                Error::<Runtime>::NotNodeOwner
            );
            assert_noop!(
                Cps::delete_node(RuntimeOrigin::signed(4), node),
                Error::<Runtime>::NotNodeOwner
            );
        }
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(2)));
        assert_eq!(Cps::resolve_ownership(NodeId(2)), Ok((NodeId(2), 3)));
    });
}

#[test]
fn ownership_generation_exhaustion_is_atomic() {
    new_test_ext().execute_with(|| {
        assert_ok!(Cps::create_node(RuntimeOrigin::signed(1), None, None, None));
        OwnershipGeneration::<Runtime>::insert(NodeId(0), u64::MAX - 1);
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(1),
            NodeId(0),
            2
        ));
        assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(2), NodeId(0)));
        assert_eq!(Cps::ownership_generation(NodeId(0)), u64::MAX);
        assert_ok!(Cps::transfer_ownership(
            RuntimeOrigin::signed(2),
            NodeId(0),
            3
        ));
        assert_noop!(
            Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(0)),
            Error::<Runtime>::OwnershipGenerationExhausted
        );
        assert_eq!(Cps::resolve_ownership(NodeId(0)), Ok((NodeId(0), 2)));
    });
}

#[test]
fn proposal_for_missing_node_cannot_create_orphan_ownership() {
    new_test_ext().execute_with(|| {
        PendingOwnershipTransfer::<Runtime>::insert(
            NodeId(0),
            PendingTransfer {
                proposed_owner: 2,
                boundary: NodeId(0),
                generation: 0,
            },
        );
        assert_noop!(
            Cps::accept_ownership(RuntimeOrigin::signed(2), NodeId(0)),
            Error::<Runtime>::NodeNotFound
        );
        assert!(!Ownership::<Runtime>::contains_key(NodeId(0)));
    });
}

#[test]
fn ownership_resolution_reports_missing_boundary_and_broken_parent() {
    new_test_ext().execute_with(|| {
        Parent::<Runtime>::insert(NodeId(0), None::<NodeId>);
        Parent::<Runtime>::insert(NodeId(1), Some(NodeId(0)));
        assert_noop!(
            Cps::resolve_ownership(NodeId(1)),
            Error::<Runtime>::OwnershipNotFound
        );
        Parent::<Runtime>::remove(NodeId(0));
        assert_noop!(
            Cps::resolve_ownership(NodeId(1)),
            Error::<Runtime>::NodeNotFound
        );
    });
}

// ---------------------------------------------------------------------------
// Migration v1 -> v2
// ---------------------------------------------------------------------------

mod migration_tests {
    use super::*;
    use crate::migration::MigrationToV2;
    use frame_support::{
        traits::{OnRuntimeUpgrade, StorageVersion},
        Blake2_128Concat, StorageHasher,
    };
    use parity_scale_codec::Encode;

    /// Raw v1 `Node` shape (owner + path + meta + payload), written directly
    /// to storage under the old `Nodes` map key to simulate pre-migration
    /// state.
    #[derive(Encode)]
    struct OldNodeV1 {
        parent: Option<NodeId>,
        owner: u64,
        path: BoundedVec<NodeId, MaxTreeDepth>,
        meta: Option<NodeData>,
        payload: Option<NodeData>,
    }

    fn old_node_key(id: NodeId) -> Vec<u8> {
        let mut key = frame_support::storage::storage_prefix(b"Cps", b"Nodes").to_vec();
        key.extend(Blake2_128Concat::hash(&id.encode()));
        key
    }

    /// Populate the unchanged v1 indexes as well as the legacy node encoding.
    fn write_old_node(id: NodeId, node: OldNodeV1) {
        frame_support::storage::unhashed::put(&old_node_key(id), &node);
        match node.parent {
            Some(parent) => NodesByParent::<Runtime>::mutate(parent, |children| {
                children.try_push(id).unwrap();
            }),
            None => RootNodes::<Runtime>::mutate(|roots| {
                roots.try_push(id).unwrap();
            }),
        }
        NextNodeId::<Runtime>::mutate(|next| {
            next.0 = next.0.max(id.0.saturating_add(1));
        });
    }

    fn old_node(owner: u64, ancestors: &[NodeId]) -> OldNodeV1 {
        OldNodeV1 {
            parent: ancestors.last().copied(),
            owner,
            path: ancestors.to_vec().try_into().unwrap(),
            meta: None,
            payload: None,
        }
    }

    fn migrate_v1() {
        StorageVersion::new(1).put::<Cps>();
        MigrationToV2::<Runtime>::on_runtime_upgrade();
        assert_eq!(StorageVersion::get::<Cps>(), StorageVersion::new(2));
    }

    /// Inspect raw storage rather than an iterator that could skip undecodable entries.
    fn assert_old_prefix_empty() {
        let prefix = frame_support::storage::storage_prefix(b"Cps", b"Nodes");
        assert!(!sp_io::storage::exists(&prefix));
        assert!(!sp_io::storage::next_key(&prefix).is_some_and(|key| key.starts_with(&prefix)));
    }

    #[test]
    fn migration_preserves_effective_ownership() {
        new_test_ext().execute_with(|| {
            let property_manager = 1u64;
            let tenant_corp = 2u64;

            // 0: root, owned by property_manager
            write_old_node(
                NodeId(0),
                OldNodeV1 {
                    parent: None,
                    owner: property_manager,
                    path: BoundedVec::try_from(vec![]).unwrap(),
                    meta: None,
                    payload: None,
                },
            );
            // 1: child of 0, same owner (inherits, no explicit boundary expected)
            write_old_node(
                NodeId(1),
                OldNodeV1 {
                    parent: Some(NodeId(0)),
                    owner: property_manager,
                    path: BoundedVec::try_from(vec![NodeId(0)]).unwrap(),
                    meta: Some(data(b"meta")),
                    payload: None,
                },
            );
            // 2: child of 1, different owner => new boundary
            write_old_node(
                NodeId(2),
                OldNodeV1 {
                    parent: Some(NodeId(1)),
                    owner: tenant_corp,
                    path: BoundedVec::try_from(vec![NodeId(0), NodeId(1)]).unwrap(),
                    meta: None,
                    payload: Some(data(b"payload")),
                },
            );
            // 3: child of 2, same owner as 2 (inherits from the new boundary)
            write_old_node(
                NodeId(3),
                OldNodeV1 {
                    parent: Some(NodeId(2)),
                    owner: tenant_corp,
                    path: BoundedVec::try_from(vec![NodeId(0), NodeId(1), NodeId(2)]).unwrap(),
                    meta: None,
                    payload: None,
                },
            );
            NextNodeId::<Runtime>::put(NodeId(4));

            migrate_v1();

            assert_eq!(Cps::parent_of(NodeId(0)), Some(None));
            assert_eq!(Cps::parent_of(NodeId(1)), Some(Some(NodeId(0))));
            assert_eq!(Cps::parent_of(NodeId(2)), Some(Some(NodeId(1))));
            assert_eq!(Cps::parent_of(NodeId(3)), Some(Some(NodeId(2))));

            assert_eq!(Cps::meta_of(NodeId(1)), Some(data(b"meta")));
            assert_eq!(Cps::payload_of(NodeId(2)), Some(data(b"payload")));

            // Effective ownership is preserved.
            assert_eq!(
                Cps::resolve_ownership(NodeId(0)),
                Ok((NodeId(0), property_manager))
            );
            assert_eq!(
                Cps::resolve_ownership(NodeId(1)),
                Ok((NodeId(0), property_manager))
            );
            assert_eq!(
                Cps::resolve_ownership(NodeId(2)),
                Ok((NodeId(2), tenant_corp))
            );
            assert_eq!(
                Cps::resolve_ownership(NodeId(3)),
                Ok((NodeId(2), tenant_corp))
            );

            // Only nodes that start a new boundary get an explicit entry.
            assert_eq!(Cps::ownership_of(NodeId(0)), Some(property_manager));
            assert_eq!(Cps::ownership_of(NodeId(1)), None);
            assert_eq!(Cps::ownership_of(NodeId(2)), Some(tenant_corp));
            assert_eq!(Cps::ownership_of(NodeId(3)), None);
            assert_eq!(Cps::root_nodes().to_vec(), vec![NodeId(0)]);
            for id in 0..3 {
                assert_eq!(
                    Cps::nodes_by_parent(NodeId(id)).to_vec(),
                    vec![NodeId(id + 1)]
                );
            }
            assert_eq!(Cps::next_node_id(), NodeId(4));
            assert_eq!(Cps::ownership_generation(NodeId(0)), 0);
            assert_eq!(Cps::ownership_generation(NodeId(2)), 0);
            assert_eq!(OwnershipGeneration::<Runtime>::iter().count(), 0);
            assert_eq!(PendingOwnershipTransfer::<Runtime>::iter().count(), 0);
            assert_old_prefix_empty();
            assert!(payload_callback_log().is_empty());
            assert!(System::events().is_empty());
        });
    }

    #[test]
    fn migration_only_runs_at_version_one() {
        for version in [0, 2, 3] {
            new_test_ext().execute_with(|| {
                write_old_node(NodeId(64), old_node(1, &[]));
                StorageVersion::new(version).put::<Cps>();
                // Include conflicting v2 state to detect a missing version guard.
                Parent::<Runtime>::insert(NodeId(64), Some(NodeId(63)));
                Ownership::<Runtime>::insert(NodeId(64), 99);
                Meta::<Runtime>::insert(NodeId(64), data(b"v2"));
                let before = sp_io::storage::root(sp_runtime::StateVersion::V1);

                MigrationToV2::<Runtime>::on_runtime_upgrade();

                assert_eq!(
                    sp_io::storage::root(sp_runtime::StateVersion::V1),
                    before,
                    "migration modified storage at version {version}"
                );
                assert_eq!(StorageVersion::get::<Cps>(), StorageVersion::new(version));
            });
        }
    }

    #[test]
    fn empty_migration_updates_version_without_creating_state() {
        new_test_ext().execute_with(|| {
            migrate_v1();

            assert_eq!(Parent::<Runtime>::iter().count(), 0);
            assert_eq!(Meta::<Runtime>::iter().count(), 0);
            assert_eq!(Payload::<Runtime>::iter().count(), 0);
            assert_eq!(Ownership::<Runtime>::iter().count(), 0);
            assert!(!NextNodeId::<Runtime>::exists());
            assert!(!RootNodes::<Runtime>::exists());
            assert_eq!(NodesByParent::<Runtime>::iter().count(), 0);
            assert_eq!(PendingOwnershipTransfer::<Runtime>::iter().count(), 0);
            assert_old_prefix_empty();
        });
    }

    #[test]
    fn repeated_migration_preserves_subsequent_v2_changes() {
        new_test_ext().execute_with(|| {
            write_old_node(NodeId(0), old_node(1, &[]));
            write_old_node(NodeId(1), old_node(1, &[NodeId(0)]));
            migrate_v1();

            assert_ok!(Cps::set_meta(
                RuntimeOrigin::signed(1),
                NodeId(1),
                Some(data(b"updated"))
            ));
            assert_ok!(Cps::transfer_ownership(
                RuntimeOrigin::signed(1),
                NodeId(1),
                2
            ));
            assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(2), NodeId(1)));
            assert_ok!(Cps::transfer_ownership(
                RuntimeOrigin::signed(1),
                NodeId(0),
                3
            ));
            assert_ok!(Cps::create_node(
                RuntimeOrigin::signed(2),
                Some(NodeId(1)),
                None,
                Some(data(b"new"))
            ));
            let before = sp_io::storage::root(sp_runtime::StateVersion::V1);

            MigrationToV2::<Runtime>::on_runtime_upgrade();
            MigrationToV2::<Runtime>::on_runtime_upgrade();

            assert_eq!(sp_io::storage::root(sp_runtime::StateVersion::V1), before);
            assert_eq!(Cps::resolve_ownership(NodeId(2)), Ok((NodeId(1), 2)));
            assert_old_prefix_empty();
        });
    }

    #[test]
    fn migration_preserves_unset_empty_and_maximum_data_independently() {
        new_test_ext().execute_with(|| {
            // Non-UTF8 bytes and all combinations catch field swaps and Option loss.
            let maximum = data(&vec![0xff; MAX_DATA_SIZE as usize]);
            let values = [None, Some(data(b"")), Some(maximum)];
            for (meta_index, meta) in values.iter().enumerate() {
                for (payload_index, payload) in values.iter().enumerate() {
                    let id = NodeId((meta_index * values.len() + payload_index) as u64);
                    let mut node = old_node(1, &[]);
                    node.meta = meta.clone();
                    node.payload = payload.clone();
                    write_old_node(id, node);
                }
            }

            migrate_v1();

            for (meta_index, meta) in values.iter().enumerate() {
                for (payload_index, payload) in values.iter().enumerate() {
                    let id = NodeId((meta_index * values.len() + payload_index) as u64);
                    assert_eq!(Meta::<Runtime>::get(id), *meta);
                    assert_eq!(Payload::<Runtime>::get(id), *payload);
                    assert_eq!(Meta::<Runtime>::contains_key(id), meta.is_some());
                    assert_eq!(Payload::<Runtime>::contains_key(id), payload.is_some());
                    assert_eq!(Parent::<Runtime>::get(id), Some(None));
                }
            }
            assert_old_prefix_empty();
            assert!(payload_callback_log().is_empty());
        });
    }

    #[test]
    fn migration_preserves_indexes_and_counter() {
        new_test_ext().execute_with(|| {
            // Deletes and moves in v1 can leave gaps and non-sorted indexes.
            write_old_node(NodeId(64), old_node(1, &[]));
            write_old_node(NodeId(2), old_node(2, &[]));
            write_old_node(NodeId(90), old_node(1, &[NodeId(64)]));
            write_old_node(NodeId(7), old_node(2, &[NodeId(64)]));
            write_old_node(NodeId(100), old_node(2, &[NodeId(64), NodeId(7)]));
            NextNodeId::<Runtime>::put(NodeId(200));
            let keys = [
                RootNodes::<Runtime>::hashed_key().to_vec(),
                NextNodeId::<Runtime>::hashed_key().to_vec(),
                NodesByParent::<Runtime>::hashed_key_for(NodeId(64)),
                NodesByParent::<Runtime>::hashed_key_for(NodeId(7)),
            ];
            let before: Vec<_> = keys.iter().map(|key| sp_io::storage::get(key)).collect();

            migrate_v1();

            for (key, value) in keys.iter().zip(before) {
                assert_eq!(sp_io::storage::get(key), value);
            }
            assert_eq!(NodesByParent::<Runtime>::iter().count(), 2);
            assert_eq!(Cps::root_nodes().to_vec(), vec![NodeId(64), NodeId(2)]);
            assert_eq!(
                Cps::nodes_by_parent(NodeId(64)).to_vec(),
                vec![NodeId(90), NodeId(7)]
            );
            assert_noop!(
                Cps::delete_node(RuntimeOrigin::signed(1), NodeId(64)),
                Error::<Runtime>::NodeHasChildren
            );
            assert_ok!(Cps::create_node(
                RuntimeOrigin::signed(2),
                Some(NodeId(100)),
                None,
                None
            ));
            assert_eq!(Cps::parent_of(NodeId(200)), Some(Some(NodeId(100))));
            assert_eq!(Cps::next_node_id(), NodeId(201));
            assert_old_prefix_empty();
        });
    }

    #[test]
    fn migration_preserves_repeated_owners_and_sibling_boundaries() {
        new_test_ext().execute_with(|| {
            // A -> B -> A creates a second A boundary, not inheritance across B.
            // Equal owners in separate roots must also remain separate.
            let nodes = [
                (0, 1, vec![], 0),
                (1, 2, vec![0], 1),
                (2, 1, vec![0, 1], 2),
                (3, 1, vec![0, 1, 2], 2),
                (4, 2, vec![0, 1], 1),
                (5, 1, vec![0], 0),
                (6, 2, vec![0], 6),
                (7, 1, vec![], 7),
            ];
            for (id, owner, ancestors, _) in &nodes {
                let path: Vec<_> = ancestors.iter().copied().map(NodeId).collect();
                write_old_node(NodeId(*id), old_node(*owner, &path));
            }

            migrate_v1();

            for (id, owner, _, boundary) in nodes {
                assert_eq!(
                    Cps::resolve_ownership(NodeId(id)),
                    Ok((NodeId(boundary), owner))
                );
                assert_eq!(
                    Cps::ownership_of(NodeId(id)),
                    (id == boundary).then_some(owner)
                );
            }
            assert_eq!(Ownership::<Runtime>::iter().count(), 5);
            assert_old_prefix_empty();
        });
    }

    #[test]
    fn migrated_boundaries_enforce_proposal_generations() {
        new_test_ext().execute_with(|| {
            write_old_node(NodeId(0), old_node(1, &[]));
            write_old_node(NodeId(1), old_node(1, &[NodeId(0)]));
            migrate_v1();

            assert_ok!(Cps::transfer_ownership(
                RuntimeOrigin::signed(1),
                NodeId(1),
                3
            ));
            assert_eq!(
                Cps::pending_ownership_transfer(NodeId(1)),
                Some(PendingTransfer {
                    proposed_owner: 3,
                    boundary: NodeId(0),
                    generation: 0,
                })
            );
            assert_ok!(Cps::transfer_ownership(
                RuntimeOrigin::signed(1),
                NodeId(0),
                2
            ));
            assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(2), NodeId(0)));
            assert_eq!(Cps::ownership_generation(NodeId(0)), 1);
            assert_noop!(
                Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(1)),
                Error::<Runtime>::StaleOwnershipTransfer
            );
            assert_ok!(Cps::transfer_ownership(
                RuntimeOrigin::signed(2),
                NodeId(1),
                3
            ));
            assert_ok!(Cps::accept_ownership(RuntimeOrigin::signed(3), NodeId(1)));
            assert_eq!(Cps::resolve_ownership(NodeId(1)), Ok((NodeId(1), 3)));
        });
    }

    #[test]
    fn migration_preserves_maximum_depth_inheritance() {
        new_test_ext().execute_with(|| {
            let mut ancestors = Vec::new();
            for id in 0..=u64::from(MAX_TREE_DEPTH) {
                write_old_node(NodeId(id), old_node(1, &ancestors));
                ancestors.push(NodeId(id));
            }

            migrate_v1();

            for id in ancestors {
                assert_eq!(Cps::resolve_ownership(id), Ok((NodeId(0), 1)));
            }
            assert_eq!(Ownership::<Runtime>::iter().count(), 1);
            assert_eq!(
                Cps::parent_of(NodeId(u64::from(MAX_TREE_DEPTH))),
                Some(Some(NodeId(u64::from(MAX_TREE_DEPTH) - 1)))
            );
            assert_old_prefix_empty();
        });
    }

    #[test]
    fn migration_clears_backend_legacy_keys_across_compact_id_boundaries() {
        let mut ext = new_test_ext();
        let ids = [0, 63, 64, 16_383, 16_384, 1 << 30, u64::MAX - 1];
        ext.execute_with(|| {
            for id in ids {
                write_old_node(NodeId(id), old_node(1, &[]));
            }
            StorageVersion::new(1).put::<Cps>();
        });
        // Trie-backed clear has different semantics from clearing overlay entries.
        ext.commit_all().unwrap();
        ext.execute_with(|| {
            MigrationToV2::<Runtime>::on_runtime_upgrade();

            assert_eq!(StorageVersion::get::<Cps>(), StorageVersion::new(2));
            for id in ids {
                assert!(!sp_io::storage::exists(&old_node_key(NodeId(id))));
                assert_eq!(Cps::parent_of(NodeId(id)), Some(None));
                assert_eq!(Cps::resolve_ownership(NodeId(id)), Ok((NodeId(id), 1)));
            }
            assert_eq!(Cps::next_node_id(), NodeId(u64::MAX));
            assert_eq!(Cps::root_nodes().to_vec(), ids.map(NodeId).to_vec());
            assert_old_prefix_empty();
        });
    }
}
