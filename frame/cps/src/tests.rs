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
//! Tests for pallet-robonomics-cps

use crate::{self as pallet_cps, *};
use frame_support::{assert_noop, assert_ok, derive_impl, parameter_types, BoundedVec};
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

parameter_types! {
    pub const MaxTreeDepth: u32 = 32;
    pub const MaxChildrenPerNode: u32 = 100;
    pub const MaxRootNodes: u32 = 100;
}

impl pallet_cps::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type CryptoAlgorithm = DefaultCryptoAlgorithm;
    type MaxTreeDepth = MaxTreeDepth;
    type MaxChildrenPerNode = MaxChildrenPerNode;
    type MaxRootNodes = MaxRootNodes;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
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
        let node = Cps::nodes(NodeId(0)).unwrap();
        assert_eq!(node.parent, None);
        assert_eq!(node.owner, account);

        // Check indexes
        assert_eq!(Cps::root_nodes().len(), 1);
    });
}

#[test]
fn create_child_node_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create parent
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        // Create child
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(0)),
            None,
            None
        ));

        let child = Cps::nodes(NodeId(1)).unwrap();
        assert_eq!(child.parent, Some(NodeId(0)));
        assert_eq!(child.owner, account);

        // Check indexes
        assert_eq!(Cps::nodes_by_parent(NodeId(0)).len(), 1);
    });
}

#[test]
fn create_node_with_data_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        let meta = Some(NodeData::Plain(
            BoundedVec::try_from(vec![1, 2, 3]).unwrap(),
        ));
        let payload = Some(NodeData::Plain(
            BoundedVec::try_from(vec![4, 5, 6]).unwrap(),
        ));

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            meta.clone(),
            payload.clone()
        ));

        let node = Cps::nodes(NodeId(0)).unwrap();
        assert_eq!(node.meta, meta);
        assert_eq!(node.payload, payload);
    });
}

#[test]
fn create_node_with_encrypted_data_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        let meta = Some(NodeData::Encrypted {
            algorithm: DefaultCryptoAlgorithm::XChaCha20Poly1305,
            ciphertext: BoundedVec::try_from(vec![7, 8, 9]).unwrap(),
        });

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            meta.clone(),
            None
        ));

        let node = Cps::nodes(NodeId(0)).unwrap();
        assert_eq!(node.meta, meta);
    });
}

#[test]
fn create_node_with_encrypted_payload_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        let payload = Some(NodeData::Encrypted {
            algorithm: DefaultCryptoAlgorithm::XChaCha20Poly1305,
            ciphertext: BoundedVec::try_from(vec![10, 11, 12, 13, 14, 15]).unwrap(),
        });

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            payload.clone()
        ));

        let node = Cps::nodes(NodeId(0)).unwrap();
        assert_eq!(node.payload, payload);
    });
}

#[test]
fn create_node_with_both_encrypted_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        let meta = Some(NodeData::Encrypted {
            algorithm: DefaultCryptoAlgorithm::XChaCha20Poly1305,
            ciphertext: BoundedVec::try_from(vec![1, 2, 3]).unwrap(),
        });

        let payload = Some(NodeData::Encrypted {
            algorithm: DefaultCryptoAlgorithm::XChaCha20Poly1305,
            ciphertext: BoundedVec::try_from(vec![4, 5, 6]).unwrap(),
        });

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            meta.clone(),
            payload.clone()
        ));

        let node = Cps::nodes(NodeId(0)).unwrap();
        assert_eq!(node.meta, meta);
        assert_eq!(node.payload, payload);
    });
}

#[test]
fn create_node_parent_not_found_fails() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        assert_noop!(
            Cps::create_node(
                RuntimeOrigin::signed(account),
                Some(NodeId(999)),
                None,
                None
            ),
            Error::<Runtime>::ParentNotFound
        );
    });
}

#[test]
fn create_child_owner_mismatch_fails() {
    new_test_ext().execute_with(|| {
        let account1 = 1u64;
        let account2 = 2u64;

        // Create parent with account1
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account1),
            None,
            None,
            None
        ));

        // Try to create child with account2
        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(account2), Some(NodeId(0)), None, None),
            Error::<Runtime>::OwnerMismatch
        );
    });
}

#[test]
fn set_meta_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create node
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        // Set meta
        let meta = Some(NodeData::Plain(
            BoundedVec::try_from(vec![1, 2, 3]).unwrap(),
        ));
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(account),
            NodeId(0),
            meta.clone()
        ));

        let node = Cps::nodes(NodeId(0)).unwrap();
        assert_eq!(node.meta, meta);
    });
}

#[test]
fn set_meta_non_owner_fails() {
    new_test_ext().execute_with(|| {
        let account1 = 1u64;
        let account2 = 2u64;

        // Create node with account1
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account1),
            None,
            None,
            None
        ));

        // Try to set meta with account2
        let meta = Some(NodeData::Plain(
            BoundedVec::try_from(vec![1, 2, 3]).unwrap(),
        ));
        assert_noop!(
            Cps::set_meta(RuntimeOrigin::signed(account2), NodeId(0), meta),
            Error::<Runtime>::NotNodeOwner
        );
    });
}

#[test]
fn set_payload_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create node
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        // Set payload
        let payload = Some(NodeData::Plain(
            BoundedVec::try_from(vec![1, 2, 3]).unwrap(),
        ));
        assert_ok!(Cps::set_payload(
            RuntimeOrigin::signed(account),
            NodeId(0),
            payload.clone()
        ));

        let node = Cps::nodes(NodeId(0)).unwrap();
        assert_eq!(node.payload, payload);
    });
}

#[test]
fn move_node_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create first parent (node 0)
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        // Create child (node 1)
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(0)),
            None,
            None
        ));

        // Create second parent (node 2)
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        // Move node 1 from parent 0 to parent 2
        assert_ok!(Cps::move_node(
            RuntimeOrigin::signed(account),
            NodeId(1),
            NodeId(2)
        ));

        let node = Cps::nodes(NodeId(1)).unwrap();
        assert_eq!(node.parent, Some(NodeId(2)));

        // Check indexes updated
        assert_eq!(Cps::nodes_by_parent(NodeId(0)).len(), 0);
        assert_eq!(Cps::nodes_by_parent(NodeId(2)).len(), 1);
    });
}

#[test]
fn move_node_cycle_detection_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create parent (node 0)
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        // Create child (node 1)
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(0)),
            None,
            None
        ));

        // Create grandchild (node 2)
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(1)),
            None,
            None
        ));

        // Try to move node 0 under its child node 1 (would create cycle)
        assert_noop!(
            Cps::move_node(RuntimeOrigin::signed(account), NodeId(0), NodeId(1)),
            Error::<Runtime>::CycleDetected
        );

        // Try to move node 0 under its grandchild node 2 (would create cycle)
        assert_noop!(
            Cps::move_node(RuntimeOrigin::signed(account), NodeId(0), NodeId(2)),
            Error::<Runtime>::CycleDetected
        );
    });
}

#[test]
fn move_node_owner_mismatch_fails() {
    new_test_ext().execute_with(|| {
        let account1 = 1u64;
        let account2 = 2u64;

        // Create parent with account1
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account1),
            None,
            None,
            None
        ));

        // Create child with account1
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account1),
            Some(NodeId(0)),
            None,
            None
        ));

        // Create new parent with account2
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account2),
            None,
            None,
            None
        ));

        // Try to move node owned by account1 to parent owned by account2
        assert_noop!(
            Cps::move_node(RuntimeOrigin::signed(account1), NodeId(1), NodeId(2)),
            Error::<Runtime>::OwnerMismatch
        );
    });
}

#[test]
fn path_tracking_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create tree: 0 -> 1 -> 2 -> 3
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
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(1)),
            None,
            None
        ));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(2)),
            None,
            None
        ));

        // Test path tracking
        let node0 = Cps::nodes(NodeId(0)).unwrap();
        assert_eq!(node0.path.len(), 0); // Root has empty path

        let node1 = Cps::nodes(NodeId(1)).unwrap();
        assert_eq!(node1.path.len(), 1);
        assert_eq!(node1.path[0], NodeId(0));

        let node2 = Cps::nodes(NodeId(2)).unwrap();
        assert_eq!(node2.path.len(), 2);
        assert_eq!(node2.path[0], NodeId(0));
        assert_eq!(node2.path[1], NodeId(1));

        let node3 = Cps::nodes(NodeId(3)).unwrap();
        assert_eq!(node3.path.len(), 3);
        assert_eq!(node3.path[0], NodeId(0));
        assert_eq!(node3.path[1], NodeId(1));
        assert_eq!(node3.path[2], NodeId(2));

        // Test cycle detection via path
        assert!(node3.path.contains(&NodeId(2)));
        assert!(node3.path.contains(&NodeId(1)));
        assert!(node3.path.contains(&NodeId(0)));
    });
}

#[test]
fn move_root_to_child_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create first root (node 0)
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        // Create second root (node 1)
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        assert_eq!(Cps::root_nodes().len(), 2);

        // Move node 0 under node 1
        assert_ok!(Cps::move_node(
            RuntimeOrigin::signed(account),
            NodeId(0),
            NodeId(1)
        ));

        let node = Cps::nodes(NodeId(0)).unwrap();
        assert_eq!(node.parent, Some(NodeId(1)));

        // Check root nodes updated
        assert_eq!(Cps::root_nodes().len(), 1);
        assert_eq!(Cps::root_nodes()[0], NodeId(1));
    });
}

#[test]
fn clear_meta_and_payload_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        let meta = Some(NodeData::Plain(
            BoundedVec::try_from(vec![1, 2, 3]).unwrap(),
        ));
        let payload = Some(NodeData::Plain(
            BoundedVec::try_from(vec![4, 5, 6]).unwrap(),
        ));

        // Create node with data
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            meta,
            payload
        ));

        // Clear meta
        assert_ok!(Cps::set_meta(
            RuntimeOrigin::signed(account),
            NodeId(0),
            None
        ));

        // Clear payload
        assert_ok!(Cps::set_payload(
            RuntimeOrigin::signed(account),
            NodeId(0),
            None
        ));

        let node = Cps::nodes(NodeId(0)).unwrap();
        assert_eq!(node.meta, None);
        assert_eq!(node.payload, None);
    });
}

#[test]
fn move_node_updates_descendant_paths() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create tree: 0 -> 1 -> 2 and separate 3 -> 4
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
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(1)),
            None,
            None
        ));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(3)),
            None,
            None
        ));

        // Before move: 0 -> 1 -> 2 and 3 -> 4
        let node2 = Cps::nodes(NodeId(2)).unwrap();
        assert_eq!(node2.path.as_slice(), &[NodeId(0), NodeId(1)]);

        // Move node 1 (with child 2) under node 3
        assert_ok!(Cps::move_node(
            RuntimeOrigin::signed(account),
            NodeId(1),
            NodeId(3)
        ));

        // After move: 0 and 3 -> 4, 3 -> 1 -> 2
        let node1 = Cps::nodes(NodeId(1)).unwrap();
        assert_eq!(node1.path.as_slice(), &[NodeId(3)]);

        let node2 = Cps::nodes(NodeId(2)).unwrap();
        assert_eq!(node2.path.as_slice(), &[NodeId(3), NodeId(1)]);

        // Node 4 should be unchanged
        let node4 = Cps::nodes(NodeId(4)).unwrap();
        assert_eq!(node4.path.as_slice(), &[NodeId(3)]);
    });
}

#[test]
fn delete_leaf_node_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create parent
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        // Create child
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(0)),
            None,
            None
        ));

        // Delete child node
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(account), NodeId(1)));

        // Verify node is deleted
        assert!(Cps::nodes(NodeId(1)).is_none());

        // Verify parent's children index is updated
        assert_eq!(Cps::nodes_by_parent(NodeId(0)).len(), 0);
    });
}

#[test]
fn delete_root_node_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create root node
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        assert_eq!(Cps::root_nodes().len(), 1);

        // Delete root node
        assert_ok!(Cps::delete_node(RuntimeOrigin::signed(account), NodeId(0)));

        // Verify node is deleted
        assert!(Cps::nodes(NodeId(0)).is_none());

        // Verify root nodes index is updated
        assert_eq!(Cps::root_nodes().len(), 0);
    });
}

#[test]
fn delete_node_with_children_fails() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create parent
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            None,
            None
        ));

        // Create child
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(NodeId(0)),
            None,
            None
        ));

        // Try to delete parent node (should fail)
        assert_noop!(
            Cps::delete_node(RuntimeOrigin::signed(account), NodeId(0)),
            Error::<Runtime>::NodeHasChildren
        );
    });
}

#[test]
fn delete_node_non_owner_fails() {
    new_test_ext().execute_with(|| {
        let account1 = 1u64;
        let account2 = 2u64;

        // Create node with account1
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account1),
            None,
            None,
            None
        ));

        // Try to delete with account2
        assert_noop!(
            Cps::delete_node(RuntimeOrigin::signed(account2), NodeId(0)),
            Error::<Runtime>::NotNodeOwner
        );
    });
}
