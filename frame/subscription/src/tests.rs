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
//! Robonomics Subscription pallet tests.

use crate::{mock::*, *};
use frame_support::{assert_err, assert_ok};
use sp_runtime::{traits::SignedExtension, DispatchError};

// Import the mock `Subscription` pallet with an explicit alias to distinguish it
// from other `Subscription` items re-exported from `crate` in these tests.
use crate::mock::Subscription as SubscriptionPallet;
use crate::ChargeSubscriptionTransaction;

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CHARLIE: u64 = 3;
const LIFETIME_ASSET_ID: u32 = 1;
const MILLIS_PER_DAY: u64 = 86_400_000; // 24 * 60 * 60 * 1000

// ========== Auction Lifecycle Tests ==========

#[test]
fn test_start_auction_lifetime() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Only root can start auctions
        assert_err!(
            SubscriptionPallet::start_auction(
                RuntimeOrigin::signed(ALICE),
                SubscriptionMode::Lifetime { tps: 10_000 }
            ),
            DispatchError::BadOrigin
        );

        // Root starts a lifetime subscription auction
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));

        // Check auction was created with id 0
        let auction = SubscriptionPallet::auction(0).unwrap();
        assert_eq!(auction.winner, None);
        assert_eq!(auction.best_price, 0);
        assert_eq!(auction.first_bid_time, None);
        assert_eq!(auction.mode, SubscriptionMode::Lifetime { tps: 10_000 });
        assert_eq!(auction.subscription_id, None);
    });
}

#[test]
fn test_start_auction_daily() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(2_000_000);

        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        let auction = SubscriptionPallet::auction(0).unwrap();
        assert_eq!(auction.mode, SubscriptionMode::Daily { days: 30 });
        assert_eq!(auction.first_bid_time, None);
    });
}

#[test]
fn test_start_multiple_auctions() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Start 3 concurrent auctions
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 5_000 }
        ));
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 100_000 }
        ));

        // Check all auctions exist
        assert!(SubscriptionPallet::auction(0).is_some());
        assert!(SubscriptionPallet::auction(1).is_some());
        assert!(SubscriptionPallet::auction(2).is_some());
        assert!(SubscriptionPallet::auction(3).is_none());

        // Verify each has correct mode
        assert_eq!(
            SubscriptionPallet::auction(0).unwrap().mode,
            SubscriptionMode::Lifetime { tps: 5_000 }
        );
        assert_eq!(
            SubscriptionPallet::auction(1).unwrap().mode,
            SubscriptionMode::Daily { days: 30 }
        );
        assert_eq!(
            SubscriptionPallet::auction(2).unwrap().mode,
            SubscriptionMode::Lifetime { tps: 100_000 }
        );
    });
}

// ========== Auction Bidding Tests ==========

#[test]
fn test_bid_first_becomes_winner() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // First bid becomes the winner
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        let auction = SubscriptionPallet::auction(0).unwrap();
        assert_eq!(auction.winner, Some(ALICE));
        assert_eq!(auction.best_price, 200);
        assert_eq!(auction.first_bid_time, Some(1_000_000));

        // Check Alice's balance was reserved
        assert_eq!(Balances::reserved_balance(ALICE), 200);
        assert_eq!(Balances::free_balance(ALICE), 10_000_000 - 200);
    });
}

#[test]
fn test_bid_outbidding() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 50_000 }
        ));

        // Alice bids first
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        assert_eq!(Balances::reserved_balance(ALICE), 200);

        // Bob outbids Alice
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(BOB), 0, 300));

        let auction = SubscriptionPallet::auction(0).unwrap();
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

        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // Bid below minimal bid
        assert_err!(
            SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 50),
            Error::<Test>::TooSmallBid
        );

        // First valid bid
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        // Try to outbid with same or lower amount
        assert_err!(
            SubscriptionPallet::bid(RuntimeOrigin::signed(BOB), 0, 200),
            Error::<Test>::TooSmallBid
        );
        assert_err!(
            SubscriptionPallet::bid(RuntimeOrigin::signed(BOB), 0, 150),
            Error::<Test>::TooSmallBid
        );
    });
}

#[test]
fn test_bid_non_existent_auction_error() {
    new_test_ext().execute_with(|| {
        assert_err!(
            SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 999, 200),
            Error::<Test>::NotExistAuction
        );
    });
}

#[test]
fn test_bid_after_auction_period_ends() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // First bid within period (this starts the countdown)
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        // Verify first_bid_time was set
        let auction = SubscriptionPallet::auction(0).unwrap();
        assert_eq!(auction.first_bid_time, Some(1_000_000));

        // Move time beyond auction duration (100_000 ms) from first bid
        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Try to outbid after period ends
        assert_err!(
            SubscriptionPallet::bid(RuntimeOrigin::signed(BOB), 0, 300),
            Error::<Test>::BiddingPeriodIsOver
        );

        // Start a new auction that has no bids yet
        Timestamp::set_timestamp(2_000_000);
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // Move time way forward (auction created long ago)
        Timestamp::set_timestamp(2_000_000 + 100_000 + 1);

        // First bid on an auction with no previous bids should work regardless of time passed since creation
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(CHARLIE), 1, 200));

        // Verify the first bid set the first_bid_time to current time
        let auction2 = SubscriptionPallet::auction(1).unwrap();
        assert_eq!(auction2.first_bid_time, Some(2_000_000 + 100_000 + 1));
    });
}

// ========== Auction Claiming Tests ==========

#[test]
fn test_claim_successful() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        // Cannot claim before auction ends
        assert_err!(
            SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, None),
            Error::<Test>::ClaimIsNotAllowed
        );

        // Move past auction period
        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Alice claims the auction
        assert_ok!(SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Check subscription was created for Alice with id 0
        let subscription = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        assert_eq!(subscription.issue_time, 1_000_000 + 100_000);
        assert_eq!(
            subscription.mode,
            SubscriptionMode::Lifetime { tps: 10_000 }
        );

        // Check auction is marked as claimed
        let auction = SubscriptionPallet::auction(0).unwrap();
        assert_eq!(auction.subscription_id, Some(0));

        // Check reserved balance was slashed
        assert_eq!(Balances::reserved_balance(ALICE), 0);
    });
}

#[test]
fn test_claim_to_beneficiary() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Alice claims but assigns to BOB
        assert_ok!(SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, Some(BOB)));

        // Subscription should be created for BOB, not ALICE
        assert!(SubscriptionPallet::subscription(ALICE, 0).is_none());
        assert!(SubscriptionPallet::subscription(BOB, 0).is_some());

        let subscription = SubscriptionPallet::subscription(BOB, 0).unwrap();
        assert_eq!(subscription.mode, SubscriptionMode::Daily { days: 30 });
    });
}

#[test]
fn test_claim_by_non_winner_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        Timestamp::set_timestamp(1_000_000 + 100_000);

        // BOB tries to claim but is not the winner
        assert_err!(
            SubscriptionPallet::claim(RuntimeOrigin::signed(BOB), 0, None),
            Error::<Test>::ClaimIsNotAllowed
        );
    });
}

#[test]
fn test_double_claim_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        Timestamp::set_timestamp(1_000_000 + 100_000);

        // First claim succeeds
        assert_ok!(SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Second claim should fail
        assert_err!(
            SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, None),
            Error::<Test>::ClaimIsNotAllowed
        );
    });
}

// ========== Fail Cases and Edge Cases Tests ==========

#[test]
fn test_bid_with_insufficient_balance() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // Try to bid more than available balance
        assert_err!(
            SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 20_000_000),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn test_multiple_subscriptions_per_user() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create first subscription for ALICE
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Create second subscription for ALICE
        Timestamp::set_timestamp(2_000_000);
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 1, 300));
        Timestamp::set_timestamp(2_000_000 + 100_000);
        assert_ok!(SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 1, None));

        // ALICE should have two subscriptions with IDs 0 and 1
        assert!(SubscriptionPallet::subscription(ALICE, 0).is_some());
        assert!(SubscriptionPallet::subscription(ALICE, 1).is_some());
        assert!(SubscriptionPallet::subscription(ALICE, 2).is_none());
    });
}

#[test]
fn test_multiple_auctions_remain_live_until_first_bid() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create 3 auctions at time 1_000_000
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 50_000 }
        ));

        // Verify all auctions have no first_bid_time
        assert_eq!(SubscriptionPallet::auction(0).unwrap().first_bid_time, None);
        assert_eq!(SubscriptionPallet::auction(1).unwrap().first_bid_time, None);
        assert_eq!(SubscriptionPallet::auction(2).unwrap().first_bid_time, None);

        // Move time way forward (1 hour = 3_600_000 ms)
        Timestamp::set_timestamp(1_000_000 + 3_600_000);

        // First bid on auction 0 should work even though created long ago
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        assert_eq!(
            SubscriptionPallet::auction(0).unwrap().first_bid_time,
            Some(1_000_000 + 3_600_000)
        );

        // Move time forward another 50_000 ms (within auction period for auction 0)
        Timestamp::set_timestamp(1_000_000 + 3_600_000 + 50_000);

        // Auction 0 is still in bidding period (started from first bid)
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(BOB), 0, 300));

        // Auction 1 still has no bids and can accept its first bid
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 1, 200));
        assert_eq!(
            SubscriptionPallet::auction(1).unwrap().first_bid_time,
            Some(1_000_000 + 3_600_000 + 50_000)
        );

        // Move time forward another 100_000 ms (past auction 0's period)
        Timestamp::set_timestamp(1_000_000 + 3_600_000 + 50_000 + 100_000);

        // Auction 0 bidding period is over (100_000 ms since first bid)
        assert_err!(
            SubscriptionPallet::bid(RuntimeOrigin::signed(CHARLIE), 0, 400),
            Error::<Test>::BiddingPeriodIsOver
        );

        // Auction 1 bidding period is also over (100_000 ms since first bid)
        assert_err!(
            SubscriptionPallet::bid(RuntimeOrigin::signed(CHARLIE), 1, 400),
            Error::<Test>::BiddingPeriodIsOver
        );

        // Auction 2 still has no bids and can accept its first bid at any time
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(CHARLIE), 2, 200));
        assert_eq!(
            SubscriptionPallet::auction(2).unwrap().first_bid_time,
            Some(1_000_000 + 3_600_000 + 50_000 + 100_000)
        );
    });
}

// ========== Asset Locking Lifetime Subscription Tests ==========

#[test]
fn test_start_lifetime_creates_subscription() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Alice locks 1000 assets
        // With AssetToTpsRatio = 100, this should give 100_000 μTPS
        let lock_amount = 1000;
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            lock_amount
        ));

        // Verify subscription was created
        let subscription = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        assert_eq!(
            subscription.mode,
            SubscriptionMode::Lifetime { tps: 100_000 }
        );
        assert_eq!(subscription.issue_time, 1_000_000);
        assert_eq!(subscription.expiration_time, None);

        // Verify assets were locked
        assert_eq!(SubscriptionPallet::locked_assets(ALICE, 0), Some(lock_amount));

        // Verify asset balance was reduced
        assert_eq!(
            Assets::balance(LIFETIME_ASSET_ID, &ALICE),
            10_000_000 - lock_amount
        );
    });
}

#[test]
fn test_start_lifetime_tps_calculation() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Test different amounts
        // Amount 500 * Ratio 100 = 50_000 μTPS
        assert_ok!(SubscriptionPallet::start_lifetime(RuntimeOrigin::signed(ALICE), 500));
        let sub0 = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        assert_eq!(sub0.mode, SubscriptionMode::Lifetime { tps: 50_000 });

        // Amount 2000 * Ratio 100 = 200_000 μTPS
        assert_ok!(SubscriptionPallet::start_lifetime(RuntimeOrigin::signed(ALICE), 2000));
        let sub1 = SubscriptionPallet::subscription(ALICE, 1).unwrap();
        assert_eq!(sub1.mode, SubscriptionMode::Lifetime { tps: 200_000 });
    });
}

// ========== Subscription Usage Tests (Transaction Extension) ==========

#[test]
fn test_lifetime_subscription_weight_accumulation() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create lifetime subscription with 500_000 μTPS (0.5 TPS)
        // Amount 5000 * Ratio 100 = 500_000 μTPS
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            5000
        ));

        // Verify subscription was created
        let subscription = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        assert_eq!(subscription.mode, SubscriptionMode::Lifetime { tps: 500_000 });
        assert_eq!(subscription.free_weight, 0);
        assert_eq!(subscription.last_update, 1_000_000);

        // Wait for weight to accumulate (10 seconds = 10_000 ms)
        // Expected weight: reference_call_weight * μTPS * delta_ms / 1_000_000_000
        // 70_952_000 * 500_000 * 10_000 / 1_000_000_000 = 354_760_000
        Timestamp::set_timestamp(1_000_000 + 10_000);

        // Execute call using subscription via consume_weight
        let call_weight = frame_support::weights::Weight::from_parts(70_952_000, 0);
        assert_ok!(Pallet::<Test>::consume_weight(&ALICE, 0, call_weight));

        // Verify subscription state after consumption
        let subscription = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        // free_weight should be: 354_760_000 - 70_952_000 = 283_808_000
        assert_eq!(subscription.free_weight, 283_808_000);
        assert_eq!(subscription.last_update, 1_000_000 + 10_000);
    });
}

#[test]
fn test_lifetime_subscription_insufficient_weight() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription with very low TPS (100 μTPS = 0.0001 TPS)
        // Amount 1 * Ratio 100 = 100 μTPS
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            1
        ));

        // Try to execute call immediately without waiting for weight accumulation
        let call_weight = frame_support::weights::Weight::from_parts(70_952_000, 0);
        assert_err!(
            Pallet::<Test>::consume_weight(&ALICE, 0, call_weight),
            Error::<Test>::FreeWeightIsNotEnough
        );

        // Wait a bit (1 second = 1_000 ms)
        // Expected weight in floating-point terms: 70_952_000 * 100 * 1_000 / 1_000_000_000 = 7_095.2,
        // but the actual integer arithmetic used by the pallet evaluates this expression directly to 7_095 units
        Timestamp::set_timestamp(1_000_000 + 1_000);

        // Still not enough weight
        assert_err!(
            Pallet::<Test>::consume_weight(&ALICE, 0, call_weight),
            Error::<Test>::FreeWeightIsNotEnough
        );
    });
}

#[test]
fn test_multiple_calls_with_lifetime_subscription() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription with 100_000 μTPS (0.1 TPS)
        // Amount 1000 * Ratio 100 = 100_000 μTPS
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            1000
        ));

        let call_weight = frame_support::weights::Weight::from_parts(70_952_000, 0);

        // Wait for weight to accumulate (10 seconds)
        // Expected: 70_952_000 * 100_000 * 10_000 / 1_000_000_000 = 70_952_000
        Timestamp::set_timestamp(1_000_000 + 10_000);

        // First call should succeed
        assert_ok!(Pallet::<Test>::consume_weight(&ALICE, 0, call_weight));
        let sub = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        assert_eq!(sub.free_weight, 0); // All weight consumed

        // Second call immediately should fail
        assert_err!(
            Pallet::<Test>::consume_weight(&ALICE, 0, call_weight),
            Error::<Test>::FreeWeightIsNotEnough
        );

        // Wait another 10 seconds
        Timestamp::set_timestamp(1_000_000 + 20_000);

        // Second call should now succeed
        assert_ok!(Pallet::<Test>::consume_weight(&ALICE, 0, call_weight));

        // Wait 5 seconds (half the time)
        // Expected: 70_952_000 * 100_000 * 5_000 / 1_000_000_000 = 35_476_000
        Timestamp::set_timestamp(1_000_000 + 25_000);

        // Try a call - should fail as we need 70_952_000 but only have ~35_476_000
        assert_err!(
            Pallet::<Test>::consume_weight(&ALICE, 0, call_weight),
            Error::<Test>::FreeWeightIsNotEnough
        );
    });
}

#[test]
fn test_active_daily_subscription_usage() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create daily subscription (30 days)
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Verify subscription has 10_000 μTPS (0.01 TPS) and correct expiration
        let subscription = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        assert_eq!(subscription.mode, SubscriptionMode::Daily { days: 30 });
        // Expiration: issue_time + days * MILLIS_PER_DAY
        // (1_000_000 + 100_000) + 30 * 86_400_000 = 1_100_000 + 2_592_000_000 = 2_593_100_000
        assert_eq!(subscription.expiration_time, Some(2_593_100_000));

        // Wait 10 seconds for weight accumulation
        // Expected: 70_952_000 * 10_000 * 10_000 / 1_000_000_000 = 7_095_200
        Timestamp::set_timestamp(1_100_000 + 10_000);

        let call_weight = frame_support::weights::Weight::from_parts(7_095_000, 0);
        assert_ok!(Pallet::<Test>::consume_weight(&ALICE, 0, call_weight));

        // Verify weight was consumed
        let subscription = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        // Actual accumulated: 7_095_200, consumed: 7_095_000, remaining: 200
        assert_eq!(subscription.free_weight, 200);
    });
}

#[test]
fn test_daily_subscription_expiration() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create short daily subscription (1 day)
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 1 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Verify subscription
        let subscription = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        // Expiration: (1_000_000 + 100_000) + 1 * MILLIS_PER_DAY = 87_500_000
        assert_eq!(subscription.expiration_time, Some(87_500_000));

        // Advance time beyond subscription period (1 day + extra)
        Timestamp::set_timestamp(87_500_001);

        // Attempt to execute call after expiration
        let call_weight = frame_support::weights::Weight::from_parts(7_095_000, 0);
        assert_err!(
            Pallet::<Test>::consume_weight(&ALICE, 0, call_weight),
            Error::<Test>::SubscriptionIsOver
        );
    });
}

#[test]
fn test_daily_subscription_expiration_boundary() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create daily subscription (2 days)
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 2 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, None));

        let subscription = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        // Expiration: (1_000_000 + 100_000) + 2 * MILLIS_PER_DAY = 1_100_000 + 172_800_000 = 173_900_000
        assert_eq!(subscription.expiration_time, Some(173_900_000));

        // Test call just before expiration (should succeed)
        // Wait enough time to accumulate weight
        Timestamp::set_timestamp(173_899_990); // 10ms before expiration

        let call_weight = frame_support::weights::Weight::from_parts(7_095_000, 0);
        assert_ok!(Pallet::<Test>::consume_weight(&ALICE, 0, call_weight));

        // Test call exactly at expiration time (should fail)
        Timestamp::set_timestamp(173_900_000);
        assert_err!(
            Pallet::<Test>::consume_weight(&ALICE, 0, call_weight),
            Error::<Test>::SubscriptionIsOver
        );

        // Test call just after expiration (should fail)
        Timestamp::set_timestamp(173_900_001);
        assert_err!(
            Pallet::<Test>::consume_weight(&ALICE, 0, call_weight),
            Error::<Test>::SubscriptionIsOver
        );
    });
}

#[test]
fn test_non_existent_subscription_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Try to use subscription ID that doesn't exist
        let call_weight = frame_support::weights::Weight::from_parts(70_952_000, 0);
        assert_err!(
            Pallet::<Test>::consume_weight(&ALICE, 999, call_weight),
            Error::<Test>::NoSubscription
        );

        // Create subscription with ID 0
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            1000
        ));

        // Try non-existent ID 1
        assert_err!(
            Pallet::<Test>::consume_weight(&ALICE, 1, call_weight),
            Error::<Test>::NoSubscription
        );

        // Try with wrong owner
        assert_err!(
            Pallet::<Test>::consume_weight(&BOB, 0, call_weight),
            Error::<Test>::NoSubscription
        );
    });
}

#[test]
fn test_weight_calculation_accuracy() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription with known TPS: 250_000 μTPS (0.25 TPS)
        // Amount 2500 * Ratio 100 = 250_000 μTPS
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            2500
        ));

        let sub = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        assert_eq!(sub.free_weight, 0);
        assert_eq!(sub.last_update, 1_000_000);

        // Wait exact time period (5 seconds = 5_000 ms)
        Timestamp::set_timestamp(1_000_000 + 5_000);

        // Calculate expected weight:
        // ReferenceCallWeight * μTPS * delta_ms / 1_000_000_000
        // 70_952_000 * 250_000 * 5_000 / 1_000_000_000 = 88_690_000
        let call_weight = frame_support::weights::Weight::from_parts(88_690_000, 0);
        assert_ok!(Pallet::<Test>::consume_weight(&ALICE, 0, call_weight));

        // Verify exact weight was available and consumed
        let sub = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        assert_eq!(sub.free_weight, 0); // Exactly consumed all accumulated weight
        assert_eq!(sub.last_update, 1_000_000 + 5_000);

        // Wait another exact period (3 seconds = 3_000 ms)
        // Expected: 70_952_000 * 250_000 * 3_000 / 1_000_000_000 = 53_214_000
        Timestamp::set_timestamp(1_000_000 + 8_000);

        let call_weight2 = frame_support::weights::Weight::from_parts(53_214_000, 0);
        assert_ok!(Pallet::<Test>::consume_weight(&ALICE, 0, call_weight2));

        let sub = SubscriptionPallet::subscription(ALICE, 0).unwrap();
        assert_eq!(sub.free_weight, 0);

        // Try a call with slightly more weight than accumulated
        // Wait 1 second = 1_000 ms
        // Expected: 70_952_000 * 250_000 * 1_000 / 1_000_000_000 = 17_738_000
        Timestamp::set_timestamp(1_000_000 + 9_000);

        let call_weight3 = frame_support::weights::Weight::from_parts(17_738_001, 0);
        assert_err!(
            Pallet::<Test>::consume_weight(&ALICE, 0, call_weight3),
            Error::<Test>::FreeWeightIsNotEnough
        );

        // But exact weight should work
        let call_weight4 = frame_support::weights::Weight::from_parts(17_738_000, 0);
        assert_ok!(Pallet::<Test>::consume_weight(&ALICE, 0, call_weight4));
    });
}

// ========== Transaction Extension Helper Method Tests ==========

#[test]
fn test_has_permission_owner() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription for ALICE
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            1000
        ));

        // Owner always has permission
        assert!(Pallet::<Test>::has_permission(&ALICE, &ALICE, 0));

        // Other accounts do not have permission by default
        assert!(!Pallet::<Test>::has_permission(&BOB, &ALICE, 0));
        assert!(!Pallet::<Test>::has_permission(&CHARLIE, &ALICE, 0));
    });
}

#[test]
fn test_has_permission_delegate() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription for ALICE
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            1000
        ));

        // BOB initially has no permission
        assert!(!Pallet::<Test>::has_permission(&BOB, &ALICE, 0));

        // ALICE grants permission to BOB
        assert_ok!(SubscriptionPallet::grant_access(
            RuntimeOrigin::signed(ALICE),
            0,
            BOB
        ));

        // BOB now has permission
        assert!(Pallet::<Test>::has_permission(&BOB, &ALICE, 0));

        // CHARLIE still has no permission
        assert!(!Pallet::<Test>::has_permission(&CHARLIE, &ALICE, 0));

        // ALICE revokes BOB's permission
        assert_ok!(SubscriptionPallet::revoke_access(
            RuntimeOrigin::signed(ALICE),
            0,
            BOB
        ));

        // BOB no longer has permission
        assert!(!Pallet::<Test>::has_permission(&BOB, &ALICE, 0));
    });
}

#[test]
fn test_is_subscription_active_lifetime() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Non-existent subscription is not active
        assert!(!Pallet::<Test>::is_subscription_active(&ALICE, 0));

        // Create lifetime subscription
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            1000
        ));

        // Lifetime subscription is always active (never expires)
        assert!(Pallet::<Test>::is_subscription_active(&ALICE, 0));

        // Even after a long time, lifetime subscription remains active
        Timestamp::set_timestamp(1_000_000_000_000);
        assert!(Pallet::<Test>::is_subscription_active(&ALICE, 0));
    });
}

#[test]
fn test_is_subscription_active_daily() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create daily subscription (2 days)
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 2 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Subscription is active immediately after creation
        assert!(Pallet::<Test>::is_subscription_active(&ALICE, 0));

        // Still active before expiration
        Timestamp::set_timestamp(173_899_999); // 1ms before expiration
        assert!(Pallet::<Test>::is_subscription_active(&ALICE, 0));

        // Not active at expiration time
        Timestamp::set_timestamp(173_900_000);
        assert!(!Pallet::<Test>::is_subscription_active(&ALICE, 0));

        // Not active after expiration
        Timestamp::set_timestamp(173_900_001);
        assert!(!Pallet::<Test>::is_subscription_active(&ALICE, 0));
    });
}

#[test]
fn test_has_sufficient_weight_lifetime() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription with 100_000 μTPS
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            1000
        ));

        // Initially has 0 weight
        assert!(!Pallet::<Test>::has_sufficient_weight(&ALICE, 0, 70_952_000));

        // Wait 10 seconds: 70_952_000 * 100_000 * 10_000 / 1_000_000_000 = 70_952_000
        Timestamp::set_timestamp(1_000_000 + 10_000);

        // Now has sufficient weight for one call
        assert!(Pallet::<Test>::has_sufficient_weight(&ALICE, 0, 70_952_000));

        // But not for two calls
        assert!(!Pallet::<Test>::has_sufficient_weight(&ALICE, 0, 141_904_000));
    });
}

#[test]
fn test_has_sufficient_weight_daily_expired() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create short daily subscription (1 day)
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 1 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Wait to accumulate weight
        Timestamp::set_timestamp(1_100_000 + 10_000);
        assert!(Pallet::<Test>::has_sufficient_weight(&ALICE, 0, 7_095_000));

        // After expiration, subscription has no weight
        Timestamp::set_timestamp(87_500_001);
        assert!(!Pallet::<Test>::has_sufficient_weight(&ALICE, 0, 7_095_000));
    });
}

// ========== Transaction Extension Validation Tests ==========

#[test]
fn test_extension_validation_disabled() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create extension with Disabled variant
        let extension = ChargeSubscriptionTransaction::<Test>::Disabled;

        // Disabled extension should always validate successfully
        let info = frame_support::dispatch::DispatchInfo {
            call_weight: frame_support::weights::Weight::from_parts(70_952_000, 0),
            class: frame_support::dispatch::DispatchClass::Normal,
            pays_fee: frame_support::dispatch::Pays::Yes,
            extension_weight: frame_support::weights::Weight::zero(),
        };

        // Validate should succeed even without subscription
        let result = extension.validate(&ALICE, &RuntimeCall::System(
            frame_system::Call::remark { remark: vec![] }
        ), &info, 0);
        assert!(result.is_ok());
    });
}

#[test]
fn test_extension_validation_enabled_success() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription for ALICE
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            1000
        ));

        // Wait for weight to accumulate
        Timestamp::set_timestamp(1_000_000 + 10_000);

        // Create extension with Enabled variant
        let extension = ChargeSubscriptionTransaction::<Test>::Enabled {
            subscription_owner: ALICE,
            subscription_id: 0,
        };

        let info = frame_support::dispatch::DispatchInfo {
            call_weight: frame_support::weights::Weight::from_parts(70_952_000, 0),
            class: frame_support::dispatch::DispatchClass::Normal,
            pays_fee: frame_support::dispatch::Pays::No,
            extension_weight: frame_support::weights::Weight::zero(),
        };

        // Validate should succeed with valid subscription
        let result = extension.validate(&ALICE, &RuntimeCall::System(
            frame_system::Call::remark { remark: vec![] }
        ), &info, 0);
        assert!(result.is_ok());
    });
}

#[test]
fn test_extension_validation_no_permission() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription for ALICE
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            1000
        ));

        // Wait for weight
        Timestamp::set_timestamp(1_000_000 + 10_000);

        // BOB tries to use ALICE's subscription without permission
        let extension = ChargeSubscriptionTransaction::<Test>::Enabled {
            subscription_owner: ALICE,
            subscription_id: 0,
        };

        let info = frame_support::dispatch::DispatchInfo {
            call_weight: frame_support::weights::Weight::from_parts(70_952_000, 0),
            class: frame_support::dispatch::DispatchClass::Normal,
            pays_fee: frame_support::dispatch::Pays::No,
            extension_weight: frame_support::weights::Weight::zero(),
        };

        // Validate should fail with BadSigner
        let result = extension.validate(&BOB, &RuntimeCall::System(
            frame_system::Call::remark { remark: vec![] }
        ), &info, 0);
        assert!(result.is_err());
    });
}

#[test]
fn test_extension_validation_with_permission() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription for ALICE
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            1000
        ));

        // ALICE grants permission to BOB
        assert_ok!(SubscriptionPallet::grant_access(
            RuntimeOrigin::signed(ALICE),
            0,
            BOB
        ));

        // Wait for weight
        Timestamp::set_timestamp(1_000_000 + 10_000);

        // BOB uses ALICE's subscription with permission
        let extension = ChargeSubscriptionTransaction::<Test>::Enabled {
            subscription_owner: ALICE,
            subscription_id: 0,
        };

        let info = frame_support::dispatch::DispatchInfo {
            call_weight: frame_support::weights::Weight::from_parts(70_952_000, 0),
            class: frame_support::dispatch::DispatchClass::Normal,
            pays_fee: frame_support::dispatch::Pays::No,
            extension_weight: frame_support::weights::Weight::zero(),
        };

        // Validate should succeed with permission
        let result = extension.validate(&BOB, &RuntimeCall::System(
            frame_system::Call::remark { remark: vec![] }
        ), &info, 0);
        assert!(result.is_ok());
    });
}

#[test]
fn test_extension_validation_insufficient_weight() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription with low TPS
        assert_ok!(SubscriptionPallet::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            1 // 100 μTPS
        ));

        // Don't wait - subscription has no weight yet
        let extension = ChargeSubscriptionTransaction::<Test>::Enabled {
            subscription_owner: ALICE,
            subscription_id: 0,
        };

        let info = frame_support::dispatch::DispatchInfo {
            call_weight: frame_support::weights::Weight::from_parts(70_952_000, 0),
            class: frame_support::dispatch::DispatchClass::Normal,
            pays_fee: frame_support::dispatch::Pays::No,
            extension_weight: frame_support::weights::Weight::zero(),
        };

        // Validate should fail with Payment error (insufficient weight)
        let result = extension.validate(&ALICE, &RuntimeCall::System(
            frame_system::Call::remark { remark: vec![] }
        ), &info, 0);
        assert!(result.is_err());
    });
}

#[test]
fn test_extension_validation_expired_subscription() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create short daily subscription
        assert_ok!(SubscriptionPallet::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 1 }
        ));
        assert_ok!(SubscriptionPallet::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(SubscriptionPallet::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Move past expiration
        Timestamp::set_timestamp(87_500_001);

        let extension = ChargeSubscriptionTransaction::<Test>::Enabled {
            subscription_owner: ALICE,
            subscription_id: 0,
        };

        let info = frame_support::dispatch::DispatchInfo {
            call_weight: frame_support::weights::Weight::from_parts(7_095_000, 0),
            class: frame_support::dispatch::DispatchClass::Normal,
            pays_fee: frame_support::dispatch::Pays::No,
            extension_weight: frame_support::weights::Weight::zero(),
        };

        // Validate should fail with Payment error (expired)
        let result = extension.validate(&ALICE, &RuntimeCall::System(
            frame_system::Call::remark { remark: vec![] }
        ), &info, 0);
        assert!(result.is_err());
    });
}

#[test]
fn test_extension_validation_non_existent_subscription() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Try to use non-existent subscription
        let extension = ChargeSubscriptionTransaction::<Test>::Enabled {
            subscription_owner: ALICE,
            subscription_id: 999,
        };

        let info = frame_support::dispatch::DispatchInfo {
            call_weight: frame_support::weights::Weight::from_parts(70_952_000, 0),
            class: frame_support::dispatch::DispatchClass::Normal,
            pays_fee: frame_support::dispatch::Pays::No,
            extension_weight: frame_support::weights::Weight::zero(),
        };

        // Validate should fail with Payment error
        let result = extension.validate(&ALICE, &RuntimeCall::System(
            frame_system::Call::remark { remark: vec![] }
        ), &info, 0);
        assert!(result.is_err());
    });
}


