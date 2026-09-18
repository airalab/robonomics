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
        assert_eq!(Cps::pending_ownership_transfer(NodeId(0)), Some(new_owner));
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
// Migration v1 -> v2
// ---------------------------------------------------------------------------

mod migration_tests {
    use super::*;
    use crate::migration::UncheckedMigrationToV2;
    use frame_support::{traits::UncheckedOnRuntimeUpgrade, Blake2_128Concat, StorageHasher};
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

    fn write_old_node(id: NodeId, node: OldNodeV1) {
        let mut key = frame_support::storage::storage_prefix(b"Cps", b"Nodes").to_vec();
        key.extend(Blake2_128Concat::hash(&id.encode()));
        frame_support::storage::unhashed::put(&key, &node);
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

            UncheckedMigrationToV2::<Runtime>::on_runtime_upgrade();

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
        });
    }
}
