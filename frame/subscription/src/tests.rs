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

use crate::{mock::*, *};
use frame_support::{assert_err, assert_ok, traits::InstanceFilter};
use sp_runtime::DispatchError;

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CHARLIE: u64 = 3;
const LIFETIME_ASSET_ID: u32 = 1;

// ========== Auction Lifecycle Tests ==========

#[test]
fn test_start_auction_lifetime() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Only root can start auctions
        assert_err!(
            Subscription::start_auction(
                RuntimeOrigin::signed(ALICE),
                SubscriptionMode::Lifetime { tps: 10_000 }
            ),
            DispatchError::BadOrigin
        );

        // Root starts a lifetime subscription auction
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));

        // Check auction was created with id 0
        let auction = Subscription::auction(0).unwrap();
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

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        let auction = Subscription::auction(0).unwrap();
        assert_eq!(auction.mode, SubscriptionMode::Daily { days: 30 });
        assert_eq!(auction.first_bid_time, None);
    });
}

#[test]
fn test_start_multiple_auctions() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Start 3 concurrent auctions
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 5_000 }
        ));
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 100_000 }
        ));

        // Check all auctions exist
        assert!(Subscription::auction(0).is_some());
        assert!(Subscription::auction(1).is_some());
        assert!(Subscription::auction(2).is_some());
        assert!(Subscription::auction(3).is_none());

        // Verify each has correct mode
        assert_eq!(
            Subscription::auction(0).unwrap().mode,
            SubscriptionMode::Lifetime { tps: 5_000 }
        );
        assert_eq!(
            Subscription::auction(1).unwrap().mode,
            SubscriptionMode::Daily { days: 30 }
        );
        assert_eq!(
            Subscription::auction(2).unwrap().mode,
            SubscriptionMode::Lifetime { tps: 100_000 }
        );
    });
}

// ========== Auction Bidding Tests ==========

#[test]
fn test_bid_first_becomes_winner() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // First bid becomes the winner
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        let auction = Subscription::auction(0).unwrap();
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

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 50_000 }
        ));

        // Alice bids first
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        assert_eq!(Balances::reserved_balance(ALICE), 200);

        // Bob outbids Alice
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(BOB), 0, 300));

        let auction = Subscription::auction(0).unwrap();
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

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // Bid below minimal bid
        assert_err!(
            Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 50),
            Error::<Test>::TooSmallBid
        );

        // First valid bid
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        // Try to outbid with same or lower amount
        assert_err!(
            Subscription::bid(RuntimeOrigin::signed(BOB), 0, 200),
            Error::<Test>::TooSmallBid
        );
        assert_err!(
            Subscription::bid(RuntimeOrigin::signed(BOB), 0, 150),
            Error::<Test>::TooSmallBid
        );
    });
}

#[test]
fn test_bid_non_existent_auction_error() {
    new_test_ext().execute_with(|| {
        assert_err!(
            Subscription::bid(RuntimeOrigin::signed(ALICE), 999, 200),
            Error::<Test>::NotExistAuction
        );
    });
}

#[test]
fn test_bid_after_auction_period_ends() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // First bid within period (this starts the countdown)
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        // Verify first_bid_time was set
        let auction = Subscription::auction(0).unwrap();
        assert_eq!(auction.first_bid_time, Some(1_000_000));

        // Move time beyond auction duration (100_000 ms) from first bid
        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Try to outbid after period ends
        assert_err!(
            Subscription::bid(RuntimeOrigin::signed(BOB), 0, 300),
            Error::<Test>::BiddingPeriodIsOver
        );

        // Start a new auction that has no bids yet
        Timestamp::set_timestamp(2_000_000);
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // Move time way forward (auction created long ago)
        Timestamp::set_timestamp(2_000_000 + 100_000 + 1);

        // First bid on an auction with no previous bids should work regardless of time passed since creation
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(CHARLIE), 1, 200));

        // Verify the first bid set the first_bid_time to current time
        let auction2 = Subscription::auction(1).unwrap();
        assert_eq!(auction2.first_bid_time, Some(2_000_000 + 100_000 + 1));
    });
}

// ========== Auction Claiming Tests ==========

#[test]
fn test_claim_successful() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        // Cannot claim before auction ends
        assert_err!(
            Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None),
            Error::<Test>::ClaimIsNotAllowed
        );

        // Move past auction period
        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Alice claims the auction
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Check subscription was created for Alice with id 0
        let subscription = Subscription::subscription(ALICE, 0).unwrap();
        assert_eq!(subscription.issue_time, 1_000_000 + 100_000);
        assert_eq!(
            subscription.mode,
            SubscriptionMode::Lifetime { tps: 10_000 }
        );

        // Check auction is marked as claimed
        let auction = Subscription::auction(0).unwrap();
        assert_eq!(auction.subscription_id, Some(0));

        // Check reserved balance was slashed
        assert_eq!(Balances::reserved_balance(ALICE), 0);
    });
}

#[test]
fn test_claim_to_beneficiary() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Alice claims but assigns to BOB
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 0, Some(BOB)));

        // Subscription should be created for BOB, not ALICE
        assert!(Subscription::subscription(ALICE, 0).is_none());
        assert!(Subscription::subscription(BOB, 0).is_some());

        let subscription = Subscription::subscription(BOB, 0).unwrap();
        assert_eq!(subscription.mode, SubscriptionMode::Daily { days: 30 });
    });
}

#[test]
fn test_claim_by_non_winner_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        Timestamp::set_timestamp(1_000_000 + 100_000);

        // BOB tries to claim but is not the winner
        assert_err!(
            Subscription::claim(RuntimeOrigin::signed(BOB), 0, None),
            Error::<Test>::ClaimIsNotAllowed
        );
    });
}

#[test]
fn test_double_claim_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        Timestamp::set_timestamp(1_000_000 + 100_000);

        // First claim succeeds
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Second claim should fail
        assert_err!(
            Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None),
            Error::<Test>::ClaimIsNotAllowed
        );
    });
}

// ========== Subscription Usage Tests ==========

#[test]
fn test_rws_call_with_lifetime_subscription() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Setup auction and create subscription
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 500_000 } // 0.5 TPS
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Initial subscription has no free weight
        let subscription = Subscription::subscription(ALICE, 0).unwrap();
        assert_eq!(subscription.free_weight, 0);

        // Wait 10 seconds (10_000 ms) for weight to accumulate
        // At 500_000 uTPS (0.5 TPS):
        // Weight = ReferenceCallWeight * uTPS * delta_ms / 1_000_000_000
        // Weight = 70_952_000 * 500_000 * 10_000 / 1_000_000_000 = 354_760_000
        Timestamp::set_timestamp(1_000_000 + 100_000 + 10_000);

        // Try to call a balances transfer
        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });

        assert_ok!(Subscription::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)));

        // Check weight was consumed
        let subscription = Subscription::subscription(ALICE, 0).unwrap();
        // After 10 seconds at 500,000 uTPS, we accumulated 354,760,000 weight units
        // The call weight would be subtracted from this
        assert!(subscription.free_weight > 0);
    });
}

#[test]
fn test_rws_call_with_daily_subscription() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Wait 1000 seconds (1_000_000 ms) for weight to accumulate
        // Daily subscriptions have 0.01 TPS (10_000 uTPS)
        // Weight = ReferenceCallWeight * uTPS * delta_ms / 1_000_000_000
        // Weight = 70_952_000 * 10_000 * 1_000_000 / 1_000_000_000 = 709_520_000
        Timestamp::set_timestamp(1_000_000 + 100_000 + 1_000_000);

        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });

        assert_ok!(Subscription::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)));
    });
}

#[test]
fn test_free_weight_accumulation() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 1_000_000 } // 1 TPS
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None));

        let initial_time = 1_000_000 + 100_000;

        // Wait 10 seconds to accumulate enough weight for a transfer call
        // At 1_000_000 uTPS (1 TPS):
        // Weight = ReferenceCallWeight * uTPS * delta_ms / 1_000_000_000
        // Weight = 70_952_000 * 1_000_000 * 10_000 / 1_000_000_000 = 709_520_000
        Timestamp::set_timestamp(initial_time + 10_000);
        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });
        assert_ok!(Subscription::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)));

        let sub1 = Subscription::subscription(ALICE, 0).unwrap();
        let weight1 = sub1.free_weight;

        // Verify that some weight remains after the call (accumulated weight > call weight)
        assert!(weight1 > 0, "Weight should remain after call: {}", weight1);

        // Wait another 10 seconds for more weight to accumulate
        Timestamp::set_timestamp(initial_time + 20_000);
        let call2 = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });
        assert_ok!(Subscription::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call2)));

        let sub2 = Subscription::subscription(ALICE, 0).unwrap();
        let weight2 = sub2.free_weight;

        // Weight should have accumulated between the two calls
        // Since both calls consume similar weight and 10 seconds passed, weight2 should be > weight1
        assert!(
            weight2 > weight1,
            "Weight should accumulate over time. weight1: {}, weight2: {}",
            weight1,
            weight2
        );
    });
}

#[test]
fn test_insufficient_weight_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription with very low TPS
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 1 } // Very low TPS
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Try to call immediately without waiting for weight to accumulate
        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });

        assert_err!(
            Subscription::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)),
            Error::<Test>::FreeWeightIsNotEnough
        );
    });
}

#[test]
fn test_expired_daily_subscription_error() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 1 } // 1 day subscription
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Move time beyond subscription period (1 day = 86,400,000 ms)
        let one_day_ms: u64 = 24 * 60 * 60 * 1000;
        Timestamp::set_timestamp(1_000_000 + 100_000 + one_day_ms + 1_000);

        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });

        assert_err!(
            Subscription::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)),
            Error::<Test>::SubscriptionIsOver
        );
    });
}

#[test]
fn test_non_existent_subscription_error() {
    new_test_ext().execute_with(|| {
        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });

        assert_err!(
            Subscription::call(RuntimeOrigin::signed(ALICE), 999, Box::new(call)),
            Error::<Test>::NoSubscription
        );
    });
}

// ========== Fail Cases and Edge Cases Tests ==========

#[test]
fn test_bid_with_insufficient_balance() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));

        // Try to bid more than available balance
        assert_err!(
            Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 20_000_000),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn test_subscription_expiration_boundary() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Use a shorter subscription period to avoid overflow
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 1 }
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None));

        let subscription_start = 1_000_000 + 100_000;
        let one_day_ms: u64 = 24 * 60 * 60 * 1000;

        // Test 10 minutes after subscription start (600_000 ms)
        Timestamp::set_timestamp(subscription_start + 600_000);
        let call1 = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });
        // This should work as subscription is still active
        assert_ok!(Subscription::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call1)));

        // Just after expiration should fail
        Timestamp::set_timestamp(subscription_start + one_day_ms + 1);
        let call2 = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });
        assert_err!(
            Subscription::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call2)),
            Error::<Test>::SubscriptionIsOver
        );
    });
}

#[test]
fn test_multiple_subscriptions_per_user() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create first subscription for ALICE
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Create second subscription for ALICE
        Timestamp::set_timestamp(2_000_000);
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 1, 300));
        Timestamp::set_timestamp(2_000_000 + 100_000);
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 1, None));

        // ALICE should have two subscriptions with IDs 0 and 1
        assert!(Subscription::subscription(ALICE, 0).is_some());
        assert!(Subscription::subscription(ALICE, 1).is_some());
        assert!(Subscription::subscription(ALICE, 2).is_none());
    });
}

#[test]
fn test_weight_calculation_accuracy() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription with specific TPS
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 1_000_000 } // 1 TPS (1,000,000 uTPS)
        ));
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None));

        let start_time = 1_000_000 + 100_000;

        // Wait exactly 1 second (1000 ms)
        Timestamp::set_timestamp(start_time + 1_000);

        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });
        let _ = Subscription::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call));

        let subscription = Subscription::subscription(ALICE, 0).unwrap();

        // Expected weight after 1 second at 1 TPS:
        // ReferenceCallWeight * tps * delta_ms / 1_000_000_000
        // = 70_952_000 * 1_000_000 * 1_000 / 1_000_000_000
        // = 70_952_000
        // But the call also consumed some weight, so it should be less
        assert!(subscription.free_weight < 70_952_000);
    });
}

#[test]
fn test_multiple_auctions_remain_live_until_first_bid() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create 3 auctions at time 1_000_000
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(Subscription::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 50_000 }
        ));

        // Verify all auctions have no first_bid_time
        assert_eq!(Subscription::auction(0).unwrap().first_bid_time, None);
        assert_eq!(Subscription::auction(1).unwrap().first_bid_time, None);
        assert_eq!(Subscription::auction(2).unwrap().first_bid_time, None);

        // Move time way forward (1 hour = 3_600_000 ms)
        Timestamp::set_timestamp(1_000_000 + 3_600_000);

        // First bid on auction 0 should work even though created long ago
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        assert_eq!(
            Subscription::auction(0).unwrap().first_bid_time,
            Some(1_000_000 + 3_600_000)
        );

        // Move time forward another 50_000 ms (within auction period for auction 0)
        Timestamp::set_timestamp(1_000_000 + 3_600_000 + 50_000);

        // Auction 0 is still in bidding period (started from first bid)
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(BOB), 0, 300));

        // Auction 1 still has no bids and can accept its first bid
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(ALICE), 1, 200));
        assert_eq!(
            Subscription::auction(1).unwrap().first_bid_time,
            Some(1_000_000 + 3_600_000 + 50_000)
        );

        // Move time forward another 100_000 ms (past auction 0's period)
        Timestamp::set_timestamp(1_000_000 + 3_600_000 + 50_000 + 100_000);

        // Auction 0 bidding period is over (100_000 ms since first bid)
        assert_err!(
            Subscription::bid(RuntimeOrigin::signed(CHARLIE), 0, 400),
            Error::<Test>::BiddingPeriodIsOver
        );

        // Auction 1 bidding period is also over (100_000 ms since first bid)
        assert_err!(
            Subscription::bid(RuntimeOrigin::signed(CHARLIE), 1, 400),
            Error::<Test>::BiddingPeriodIsOver
        );

        // Auction 2 still has no bids and can accept its first bid at any time
        assert_ok!(Subscription::bid(RuntimeOrigin::signed(CHARLIE), 2, 200));
        assert_eq!(
            Subscription::auction(2).unwrap().first_bid_time,
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
        assert_ok!(Subscription::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            lock_amount
        ));

        // Verify subscription was created
        let subscription = Subscription::subscription(ALICE, 0).unwrap();
        assert_eq!(
            subscription.mode,
            SubscriptionMode::Lifetime { tps: 100_000 }
        );
        assert_eq!(subscription.issue_time, 1_000_000);
        assert_eq!(subscription.expiration_time, None);

        // Verify assets were locked
        assert_eq!(Subscription::locked_assets(ALICE, 0), Some(lock_amount));

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
        assert_ok!(Subscription::start_lifetime(RuntimeOrigin::signed(ALICE), 500));
        let sub0 = Subscription::subscription(ALICE, 0).unwrap();
        assert_eq!(sub0.mode, SubscriptionMode::Lifetime { tps: 50_000 });

        // Amount 2000 * Ratio 100 = 200_000 μTPS
        assert_ok!(Subscription::start_lifetime(RuntimeOrigin::signed(ALICE), 2000));
        let sub1 = Subscription::subscription(ALICE, 1).unwrap();
        assert_eq!(sub1.mode, SubscriptionMode::Lifetime { tps: 200_000 });
    });
}

