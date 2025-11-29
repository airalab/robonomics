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
        Timestamp: pallet_timestamp,
        Cps: pallet_cps,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type AccountData = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxTreeDepth: u32 = 32;
    pub const MaxChildrenPerNode: u32 = 100;
    pub const MaxNodesPerOwner: u32 = 1000;
    pub const MaxRootNodes: u32 = 100;
}

impl pallet_cps::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxTreeDepth = MaxTreeDepth;
    type MaxChildrenPerNode = MaxChildrenPerNode;
    type MaxNodesPerOwner = MaxNodesPerOwner;
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

        assert_eq!(Cps::next_node_id(), 1);
        let node = Cps::nodes(0).unwrap();
        assert_eq!(node.parent, None);
        assert_eq!(node.owner, account);

        // Check indexes
        assert_eq!(Cps::root_nodes().len(), 1);
        assert_eq!(Cps::nodes_by_owner(account).len(), 1);
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
            Some(0),
            None,
            None
        ));

        let child = Cps::nodes(1).unwrap();
        assert_eq!(child.parent, Some(0));
        assert_eq!(child.owner, account);

        // Check indexes
        assert_eq!(Cps::nodes_by_parent(0).len(), 1);
        assert_eq!(Cps::nodes_by_owner(account).len(), 2);
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

        let node = Cps::nodes(0).unwrap();
        assert_eq!(node.meta, meta);
        assert_eq!(node.payload, payload);
    });
}

#[test]
fn create_node_with_encrypted_data_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        // Create crypto profile first
        let public_params = BoundedVec::try_from(vec![1, 2, 3]).unwrap();
        assert_ok!(Cps::create_crypto_profile(
            RuntimeOrigin::signed(account),
            1,
            public_params
        ));

        let meta = Some(NodeData::Encrypted {
            crypto_profile: 0,
            ciphertext: BoundedVec::try_from(vec![7, 8, 9]).unwrap(),
        });

        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            None,
            meta.clone(),
            None
        ));

        let node = Cps::nodes(0).unwrap();
        assert_eq!(node.meta, meta);
    });
}

#[test]
fn create_node_with_invalid_crypto_profile_fails() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        let meta = Some(NodeData::Encrypted {
            crypto_profile: 999,
            ciphertext: BoundedVec::try_from(vec![7, 8, 9]).unwrap(),
        });

        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(account), None, meta, None),
            Error::<Runtime>::CryptoProfileNotFound
        );
    });
}

#[test]
fn create_node_parent_not_found_fails() {
    new_test_ext().execute_with(|| {
        let account = 1u64;

        assert_noop!(
            Cps::create_node(RuntimeOrigin::signed(account), Some(999), None, None),
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
            Cps::create_node(RuntimeOrigin::signed(account2), Some(0), None, None),
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
            0,
            meta.clone()
        ));

        let node = Cps::nodes(0).unwrap();
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
            Cps::set_meta(RuntimeOrigin::signed(account2), 0, meta),
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
            0,
            payload.clone()
        ));

        let node = Cps::nodes(0).unwrap();
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
            Some(0),
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
        assert_ok!(Cps::move_node(RuntimeOrigin::signed(account), 1, 2));

        let node = Cps::nodes(1).unwrap();
        assert_eq!(node.parent, Some(2));

        // Check indexes updated
        assert_eq!(Cps::nodes_by_parent(0).len(), 0);
        assert_eq!(Cps::nodes_by_parent(2).len(), 1);
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
            Some(0),
            None,
            None
        ));

        // Create grandchild (node 2)
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(1),
            None,
            None
        ));

        // Try to move node 0 under its child node 1 (would create cycle)
        assert_noop!(
            Cps::move_node(RuntimeOrigin::signed(account), 0, 1),
            Error::<Runtime>::CycleDetected
        );

        // Try to move node 0 under its grandchild node 2 (would create cycle)
        assert_noop!(
            Cps::move_node(RuntimeOrigin::signed(account), 0, 2),
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
            Some(0),
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
            Cps::move_node(RuntimeOrigin::signed(account1), 1, 2),
            Error::<Runtime>::OwnerMismatch
        );
    });
}

#[test]
fn create_crypto_profile_works() {
    new_test_ext().execute_with(|| {
        let account = 1u64;
        let public_params = BoundedVec::try_from(vec![1, 2, 3]).unwrap();

        assert_ok!(Cps::create_crypto_profile(
            RuntimeOrigin::signed(account),
            1,
            public_params.clone()
        ));

        assert_eq!(Cps::next_profile_id(), 1);

        let profile = Cps::crypto_profiles(0).unwrap();
        assert_eq!(profile.algorithm, 1);
        assert_eq!(profile.public_params, public_params);
    });
}

#[test]
fn is_ancestor_works() {
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
            Some(0),
            None,
            None
        ));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(1),
            None,
            None
        ));
        assert_ok!(Cps::create_node(
            RuntimeOrigin::signed(account),
            Some(2),
            None,
            None
        ));

        // Test ancestor relationships
        assert_eq!(Pallet::<Runtime>::is_ancestor(3, 2).unwrap(), true);
        assert_eq!(Pallet::<Runtime>::is_ancestor(3, 1).unwrap(), true);
        assert_eq!(Pallet::<Runtime>::is_ancestor(3, 0).unwrap(), true);
        assert_eq!(Pallet::<Runtime>::is_ancestor(2, 1).unwrap(), true);
        assert_eq!(Pallet::<Runtime>::is_ancestor(2, 0).unwrap(), true);
        assert_eq!(Pallet::<Runtime>::is_ancestor(1, 0).unwrap(), true);

        // Test non-ancestors
        assert_eq!(Pallet::<Runtime>::is_ancestor(0, 1).unwrap(), false);
        assert_eq!(Pallet::<Runtime>::is_ancestor(1, 2).unwrap(), false);
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
        assert_ok!(Cps::move_node(RuntimeOrigin::signed(account), 0, 1));

        let node = Cps::nodes(0).unwrap();
        assert_eq!(node.parent, Some(1));

        // Check root nodes updated
        assert_eq!(Cps::root_nodes().len(), 1);
        assert_eq!(Cps::root_nodes()[0], 1);
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
        assert_ok!(Cps::set_meta(RuntimeOrigin::signed(account), 0, None));

        // Clear payload
        assert_ok!(Cps::set_payload(RuntimeOrigin::signed(account), 0, None));

        let node = Cps::nodes(0).unwrap();
        assert_eq!(node.meta, None);
        assert_eq!(node.payload, None);
    });
}
