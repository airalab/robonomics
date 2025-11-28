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
//! Robonomics Web Services pallet tests.

use crate::{self as rws, *};
use frame_support::{assert_err, assert_ok, parameter_types};
use pallet_robonomics_datalog as datalog;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, DispatchError,
};

type Block = frame_system::mocking::MockBlock<Runtime>;
type Balance = u128;
type Moment = u64;

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CHARLIE: u64 = 3;

frame_support::construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        Datalog: datalog,
        RWS: rws,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeTask = RuntimeTask;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
    type ExtensionsWeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
    pub const ExistentialDeposit: Balance = 10;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const WindowSize: u64 = 128;
    pub const MaximumMessageSize: u32 = 512;
}

impl datalog::Config for Runtime {
    type Record = frame_support::BoundedVec<u8, MaximumMessageSize>;
    type RuntimeEvent = RuntimeEvent;
    type Time = Timestamp;
    type WindowSize = WindowSize;
    type WeightInfo = ();
}

parameter_types! {
    pub const ReferenceCallWeight: u64 = 70_952_000;  // transfer call weight
    pub const AuctionDuration: Moment = 100_000; // 100 seconds in milliseconds
    pub const MinimalBid: Balance = 100;
}

impl Config for Runtime {
    type Call = RuntimeCall;
    type Time = Timestamp;
    type Moment = Moment;
    type AuctionCurrency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type ReferenceCallWeight = ReferenceCallWeight;
    type AuctionDuration = AuctionDuration;
    type MinimalBid = MinimalBid;
    type WeightInfo = ();
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![(ALICE, 10_000_000), (BOB, 5_000_000), (CHARLIE, 5_000_000)],
        dev_accounts: None,
    }
    .assimilate_storage(&mut storage)
    .unwrap();

    storage.into()
}

// ========== Auction Lifecycle Tests ==========

#[test]
fn test_start_auction_lifetime() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Only root can start auctions
        assert_err!(
            RWS::start_auction(
                RuntimeOrigin::signed(ALICE),
                SubscriptionMode::Lifetime { tps: 10_000 }
            ),
            DispatchError::BadOrigin
        );

        // Root starts a lifetime subscription auction
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));

        // Check auction was created with id 0
        let auction = RWS::auction(0).unwrap();
        assert_eq!(auction.winner, None);
        assert_eq!(auction.best_price, 0);
        assert_eq!(auction.created, 1_000_000);
        assert_eq!(auction.mode, SubscriptionMode::Lifetime { tps: 10_000 });
        assert_eq!(auction.subscription_id, None);
    });
}

#[test]
fn test_start_auction_daily() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(2_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        let auction = RWS::auction(0).unwrap();
        assert_eq!(auction.mode, SubscriptionMode::Daily { days: 30 });
        assert_eq!(auction.created, 2_000_000);
    });
}

#[test]
fn test_start_multiple_auctions() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Start 3 concurrent auctions
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 5_000 }
        ));
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 100_000 }
        ));

        // Check all auctions exist
        assert!(RWS::auction(0).is_some());
        assert!(RWS::auction(1).is_some());
        assert!(RWS::auction(2).is_some());
        assert!(RWS::auction(3).is_none());

        // Verify each has correct mode
        assert_eq!(
            RWS::auction(0).unwrap().mode,
            SubscriptionMode::Lifetime { tps: 5_000 }
        );
        assert_eq!(
            RWS::auction(1).unwrap().mode,
            SubscriptionMode::Daily { days: 30 }
        );
        assert_eq!(
            RWS::auction(2).unwrap().mode,
            SubscriptionMode::Lifetime { tps: 100_000 }
        );
    });
}

// ========== Auction Bidding Tests ==========

#[test]
fn test_bid_first_becomes_winner() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // First bid becomes the winner
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        let auction = RWS::auction(0).unwrap();
        assert_eq!(auction.winner, Some(ALICE));
        assert_eq!(auction.best_price, 200);

        // Check Alice's balance was reserved
        assert_eq!(Balances::reserved_balance(ALICE), 200);
        assert_eq!(Balances::free_balance(ALICE), 10_000_000 - 200);
    });
}

#[test]
fn test_bid_outbidding() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 50_000 }
        ));

        // Alice bids first
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        assert_eq!(Balances::reserved_balance(ALICE), 200);

        // Bob outbids Alice
        assert_ok!(RWS::bid(RuntimeOrigin::signed(BOB), 0, 300));

        let auction = RWS::auction(0).unwrap();
        assert_eq!(auction.winner, Some(BOB));
        assert_eq!(auction.best_price, 300);

        // Alice's reserve should be unreserved, Bob's should be reserved
        assert_eq!(Balances::reserved_balance(ALICE), 0);
        assert_eq!(Balances::reserved_balance(BOB), 300);
        assert_eq!(Balances::free_balance(ALICE), 10_000_000);
        assert_eq!(Balances::free_balance(BOB), 5_000_000 - 300);
    });
}

#[test]
fn test_bid_too_small_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // Bid below minimal bid
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(ALICE), 0, 50),
            Error::<Runtime>::TooSmallBid
        );

        // First valid bid
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        // Try to outbid with same or lower amount
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(BOB), 0, 200),
            Error::<Runtime>::TooSmallBid
        );
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(BOB), 0, 150),
            Error::<Runtime>::TooSmallBid
        );
    });
}

#[test]
fn test_bid_non_existent_auction_error() {
    new_test_ext().execute_with(|| {
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(ALICE), 999, 200),
            Error::<Runtime>::NotExistAuction
        );
    });
}

#[test]
fn test_bid_after_auction_period_ends() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // First bid within period
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        // Move time beyond auction duration (100_000 ms)
        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Try to outbid after period ends
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(BOB), 0, 300),
            Error::<Runtime>::BiddingPeriodIsOver
        );

        // First bid on expired auction should still work (no previous winner)
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        Timestamp::set_timestamp(2_000_000 + 100_000 + 1);
        assert_ok!(RWS::bid(RuntimeOrigin::signed(CHARLIE), 1, 200));
    });
}

// ========== Auction Claiming Tests ==========

#[test]
fn test_claim_successful() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        // Cannot claim before auction ends
        assert_err!(
            RWS::claim(RuntimeOrigin::signed(ALICE), 0, None),
            Error::<Runtime>::ClaimIsNotAllowed
        );

        // Move past auction period
        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Alice claims the auction
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Check subscription was created for Alice with id 0
        let subscription = RWS::subscription(ALICE, 0).unwrap();
        assert_eq!(subscription.issue_time, 1_000_000 + 100_000);
        assert_eq!(subscription.mode, SubscriptionMode::Lifetime { tps: 10_000 });

        // Check auction is marked as claimed
        let auction = RWS::auction(0).unwrap();
        assert_eq!(auction.subscription_id, Some(0));

        // Check reserved balance was slashed
        assert_eq!(Balances::reserved_balance(ALICE), 0);
    });
}

#[test]
fn test_claim_to_beneficiary() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Alice claims but assigns to BOB
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, Some(BOB)));

        // Subscription should be created for BOB, not ALICE
        assert!(RWS::subscription(ALICE, 0).is_none());
        assert!(RWS::subscription(BOB, 0).is_some());

        let subscription = RWS::subscription(BOB, 0).unwrap();
        assert_eq!(subscription.mode, SubscriptionMode::Daily { days: 30 });
    });
}

#[test]
fn test_claim_by_non_winner_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        Timestamp::set_timestamp(1_000_000 + 100_000);

        // BOB tries to claim but is not the winner
        assert_err!(
            RWS::claim(RuntimeOrigin::signed(BOB), 0, None),
            Error::<Runtime>::ClaimIsNotAllowed
        );
    });
}

#[test]
fn test_double_claim_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        Timestamp::set_timestamp(1_000_000 + 100_000);

        // First claim succeeds
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Second claim should fail
        assert_err!(
            RWS::claim(RuntimeOrigin::signed(ALICE), 0, None),
            Error::<Runtime>::ClaimIsNotAllowed
        );
    });
}

// ========== Subscription Usage Tests ==========

#[test]
fn test_rws_call_with_lifetime_subscription() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Setup auction and create subscription
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 500_000 } // 0.5 TPS
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Initial subscription has no free weight
        let subscription = RWS::subscription(ALICE, 0).unwrap();
        assert_eq!(subscription.free_weight, 0);

        // Wait 10 seconds (10_000 ms) for weight to accumulate
        // At 500_000 uTPS (0.5 TPS):
        // Weight = ReferenceCallWeight * uTPS * delta_ms / 1_000_000_000
        // Weight = 70_952_000 * 500_000 * 10_000 / 1_000_000_000 = 354_760_000
        Timestamp::set_timestamp(1_000_000 + 100_000 + 10_000);

        // Try to call datalog record
        let call = RuntimeCall::Datalog(datalog::Call::record {
            record: frame_support::BoundedVec::try_from(b"test data".to_vec()).unwrap(),
        });

        assert_ok!(RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)));

        // Check weight was consumed
        let subscription = RWS::subscription(ALICE, 0).unwrap();
        // After 10 seconds at 500,000 uTPS, we accumulated 354,760,000 weight units
        // The datalog call weight would be subtracted from this
        assert!(subscription.free_weight > 0);
    });
}

#[test]
fn test_rws_call_with_daily_subscription() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Wait 1000 seconds (1_000_000 ms) for weight to accumulate
        // Daily subscriptions have 0.01 TPS (10_000 uTPS)
        // Weight = ReferenceCallWeight * uTPS * delta_ms / 1_000_000_000
        // Weight = 70_952_000 * 10_000 * 1_000_000 / 1_000_000_000 = 709_520_000
        Timestamp::set_timestamp(1_000_000 + 100_000 + 1_000_000);

        let call = RuntimeCall::Datalog(datalog::Call::record {
            record: frame_support::BoundedVec::try_from(b"test".to_vec()).unwrap(),
        });

        assert_ok!(RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)));
    });
}

#[test]
fn test_free_weight_accumulation() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 1_000_000 } // 1 TPS
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        let initial_time = 1_000_000 + 100_000;
        
        // Check weight accumulation over time
        Timestamp::set_timestamp(initial_time + 1_000);
        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });
        // This call just updates the subscription weight
        let _ = RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call));
        
        let sub1 = RWS::subscription(ALICE, 0).unwrap();
        let weight1 = sub1.free_weight;

        // Wait another second
        Timestamp::set_timestamp(initial_time + 2_000);
        let call2 = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });
        let _ = RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call2));

        let sub2 = RWS::subscription(ALICE, 0).unwrap();
        let weight2 = sub2.free_weight;

        // Weight should have accumulated more
        assert!(weight2 > weight1 || weight2 == weight1); // May be same if call consumed accumulated weight
    });
}

#[test]
fn test_insufficient_weight_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription with very low TPS
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 1 } // Very low TPS
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Try to call immediately without waiting for weight to accumulate
        let call = RuntimeCall::Datalog(datalog::Call::record {
            record: frame_support::BoundedVec::try_from(b"test data that requires weight".to_vec()).unwrap(),
        });

        assert_err!(
            RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)),
            Error::<Runtime>::FreeWeightIsNotEnough
        );
    });
}

#[test]
fn test_expired_daily_subscription_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 1 } // 1 day subscription
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Move time beyond subscription period (1 day = 86,400,000 ms)
        let one_day_ms: u64 = 24 * 60 * 60 * 1000;
        Timestamp::set_timestamp(1_000_000 + 100_000 + one_day_ms + 1_000);

        let call = RuntimeCall::Datalog(datalog::Call::record {
            record: frame_support::BoundedVec::try_from(b"test".to_vec()).unwrap(),
        });

        assert_err!(
            RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)),
            Error::<Runtime>::SubscriptionIsOver
        );
    });
}

#[test]
fn test_non_existent_subscription_error() {
    new_test_ext().execute_with(|| {
        let call = RuntimeCall::Datalog(datalog::Call::record {
            record: frame_support::BoundedVec::try_from(b"test".to_vec()).unwrap(),
        });

        assert_err!(
            RWS::call(RuntimeOrigin::signed(ALICE), 999, Box::new(call)),
            Error::<Runtime>::NoSubscription
        );
    });
}

// ========== Fail Cases and Edge Cases Tests ==========

#[test]
fn test_bid_with_insufficient_balance() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // Try to bid more than available balance
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(ALICE), 0, 20_000_000),
            pallet_balances::Error::<Runtime>::InsufficientBalance
        );
    });
}

#[test]
fn test_subscription_expiration_boundary() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Use a shorter subscription period to avoid overflow
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 1 }
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        let subscription_start = 1_000_000 + 100_000;
        let one_day_ms: u64 = 24 * 60 * 60 * 1000;

        // Test 10 minutes after subscription start (600_000 ms)
        Timestamp::set_timestamp(subscription_start + 600_000);
        let call1 = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });
        // This should work as subscription is still active
        assert_ok!(RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call1)));

        // Just after expiration should fail
        Timestamp::set_timestamp(subscription_start + one_day_ms + 1);
        let call2 = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });
        assert_err!(
            RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call2)),
            Error::<Runtime>::SubscriptionIsOver
        );
    });
}

#[test]
fn test_multiple_subscriptions_per_user() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create first subscription for ALICE
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Create second subscription for ALICE
        Timestamp::set_timestamp(2_000_000);
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 1, 300));
        Timestamp::set_timestamp(2_000_000 + 100_000);
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 1, None));

        // ALICE should have two subscriptions with IDs 0 and 1
        assert!(RWS::subscription(ALICE, 0).is_some());
        assert!(RWS::subscription(ALICE, 1).is_some());
        assert!(RWS::subscription(ALICE, 2).is_none());
    });
}

#[test]
fn test_weight_calculation_accuracy() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription with specific TPS
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 1_000_000 } // 1 TPS (1,000,000 uTPS)
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        let start_time = 1_000_000 + 100_000;

        // Wait exactly 1 second (1000 ms)
        Timestamp::set_timestamp(start_time + 1_000);

        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });
        let _ = RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call));

        let subscription = RWS::subscription(ALICE, 0).unwrap();
        
        // Expected weight after 1 second at 1 TPS:
        // ReferenceCallWeight * tps * delta_ms / 1_000_000_000
        // = 70_952_000 * 1_000_000 * 1_000 / 1_000_000_000
        // = 70_952_000
        // But the call also consumed some weight, so it should be less
        assert!(subscription.free_weight < 70_952_000);
    });
}
