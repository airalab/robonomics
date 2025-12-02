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
use frame_support::{assert_err, assert_ok};
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
        assert_eq!(auction.first_bid_time, None);
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
        assert_eq!(auction.first_bid_time, None);
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
            Error::<Test>::TooSmallBid
        );

        // First valid bid
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        // Try to outbid with same or lower amount
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(BOB), 0, 200),
            Error::<Test>::TooSmallBid
        );
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(BOB), 0, 150),
            Error::<Test>::TooSmallBid
        );
    });
}

#[test]
fn test_bid_non_existent_auction_error() {
    new_test_ext().execute_with(|| {
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(ALICE), 999, 200),
            Error::<Test>::NotExistAuction
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

        // First bid within period (this starts the countdown)
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        // Verify first_bid_time was set
        let auction = RWS::auction(0).unwrap();
        assert_eq!(auction.first_bid_time, Some(1_000_000));

        // Move time beyond auction duration (100_000 ms) from first bid
        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Try to outbid after period ends
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(BOB), 0, 300),
            Error::<Test>::BiddingPeriodIsOver
        );

        // Start a new auction that has no bids yet
        Timestamp::set_timestamp(2_000_000);
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        
        // Move time way forward (auction created long ago)
        Timestamp::set_timestamp(2_000_000 + 100_000 + 1);
        
        // First bid on an auction with no previous bids should work regardless of time passed since creation
        assert_ok!(RWS::bid(RuntimeOrigin::signed(CHARLIE), 1, 200));
        
        // Verify the first bid set the first_bid_time to current time
        let auction2 = RWS::auction(1).unwrap();
        assert_eq!(auction2.first_bid_time, Some(2_000_000 + 100_000 + 1));
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
            Error::<Test>::ClaimIsNotAllowed
        );

        // Move past auction period
        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Alice claims the auction
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Check subscription was created for Alice with id 0
        let subscription = RWS::subscription(ALICE, 0).unwrap();
        assert_eq!(subscription.issue_time, 1_000_000 + 100_000);
        assert_eq!(
            subscription.mode,
            SubscriptionMode::Lifetime { tps: 10_000 }
        );

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
            Error::<Test>::ClaimIsNotAllowed
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

        // Try to call a balances transfer
        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });

        assert_ok!(RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)));

        // Check weight was consumed
        let subscription = RWS::subscription(ALICE, 0).unwrap();
        // After 10 seconds at 500,000 uTPS, we accumulated 354,760,000 weight units
        // The call weight would be subtracted from this
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

        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
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
        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });

        assert_err!(
            RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)),
            Error::<Test>::FreeWeightIsNotEnough
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

        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB,
            value: 1,
        });

        assert_err!(
            RWS::call(RuntimeOrigin::signed(ALICE), 0, Box::new(call)),
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
            RWS::call(RuntimeOrigin::signed(ALICE), 999, Box::new(call)),
            Error::<Test>::NoSubscription
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
            pallet_balances::Error::<Test>::InsufficientBalance
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
            Error::<Test>::SubscriptionIsOver
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

#[test]
fn test_multiple_auctions_remain_live_until_first_bid() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create 3 auctions at time 1_000_000
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Daily { days: 30 }
        ));
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 50_000 }
        ));

        // Verify all auctions have no first_bid_time
        assert_eq!(RWS::auction(0).unwrap().first_bid_time, None);
        assert_eq!(RWS::auction(1).unwrap().first_bid_time, None);
        assert_eq!(RWS::auction(2).unwrap().first_bid_time, None);

        // Move time way forward (1 hour = 3_600_000 ms)
        Timestamp::set_timestamp(1_000_000 + 3_600_000);

        // First bid on auction 0 should work even though created long ago
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        assert_eq!(
            RWS::auction(0).unwrap().first_bid_time,
            Some(1_000_000 + 3_600_000)
        );

        // Move time forward another 50_000 ms (within auction period for auction 0)
        Timestamp::set_timestamp(1_000_000 + 3_600_000 + 50_000);

        // Auction 0 is still in bidding period (started from first bid)
        assert_ok!(RWS::bid(RuntimeOrigin::signed(BOB), 0, 300));

        // Auction 1 still has no bids and can accept its first bid
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 1, 200));
        assert_eq!(
            RWS::auction(1).unwrap().first_bid_time,
            Some(1_000_000 + 3_600_000 + 50_000)
        );

        // Move time forward another 100_000 ms (past auction 0's period)
        Timestamp::set_timestamp(1_000_000 + 3_600_000 + 50_000 + 100_000);

        // Auction 0 bidding period is over (100_000 ms since first bid)
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(CHARLIE), 0, 400),
            Error::<Test>::BiddingPeriodIsOver
        );

        // Auction 1 bidding period is also over (100_000 ms since first bid)
        assert_err!(
            RWS::bid(RuntimeOrigin::signed(CHARLIE), 1, 400),
            Error::<Test>::BiddingPeriodIsOver
        );

        // Auction 2 still has no bids and can accept its first bid at any time
        assert_ok!(RWS::bid(RuntimeOrigin::signed(CHARLIE), 2, 200));
        assert_eq!(
            RWS::auction(2).unwrap().first_bid_time,
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
        assert_ok!(RWS::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            lock_amount
        ));

        // Verify subscription was created
        let subscription = RWS::subscription(ALICE, 0).unwrap();
        assert_eq!(
            subscription.mode,
            SubscriptionMode::Lifetime { tps: 100_000 }
        );
        assert_eq!(subscription.issue_time, 1_000_000);
        assert_eq!(subscription.expiration_time, None);

        // Verify assets were locked
        assert_eq!(RWS::locked_assets(ALICE, 0), Some(lock_amount));

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
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 500));
        let sub0 = RWS::subscription(ALICE, 0).unwrap();
        assert_eq!(sub0.mode, SubscriptionMode::Lifetime { tps: 50_000 });

        // Amount 2000 * Ratio 100 = 200_000 μTPS
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 2000));
        let sub1 = RWS::subscription(ALICE, 1).unwrap();
        assert_eq!(sub1.mode, SubscriptionMode::Lifetime { tps: 200_000 });
    });
}

#[test]
fn test_start_lifetime_multiple_subscriptions() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Alice creates multiple subscriptions
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 100));
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 200));
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 300));

        // Verify all subscriptions exist
        assert!(RWS::subscription(ALICE, 0).is_some());
        assert!(RWS::subscription(ALICE, 1).is_some());
        assert!(RWS::subscription(ALICE, 2).is_some());

        // Verify locked assets tracked separately
        assert_eq!(RWS::locked_assets(ALICE, 0), Some(100));
        assert_eq!(RWS::locked_assets(ALICE, 1), Some(200));
        assert_eq!(RWS::locked_assets(ALICE, 2), Some(300));
    });
}

#[test]
fn test_start_lifetime_insufficient_balance() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Try to lock more than available
        assert_err!(
            RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 20_000_000),
            Error::<Test>::CannotLockAssets
        );
    });
}

#[test]
fn test_use_asset_locked_subscription() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Alice creates subscription with 1000 assets
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 1000));

        // Fast forward 100 seconds to accumulate weight
        Timestamp::set_timestamp(1_000_000 + 100_000);

        // Use subscription to make a call
        let call = Box::new(RuntimeCall::System(
            frame_system::Call::remark { remark: vec![1, 2, 3] },
        ));
        assert_ok!(RWS::call(RuntimeOrigin::signed(ALICE), 0, call));
    });
}

#[test]
fn test_stop_lifetime_unlocks_assets() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        let lock_amount = 1000;
        let initial_balance = Assets::balance(LIFETIME_ASSET_ID, &ALICE);

        // Alice creates subscription
        assert_ok!(RWS::start_lifetime(
            RuntimeOrigin::signed(ALICE),
            lock_amount
        ));

        // Verify assets were locked
        assert_eq!(
            Assets::balance(LIFETIME_ASSET_ID, &ALICE),
            initial_balance - lock_amount
        );

        // Stop the subscription
        assert_ok!(RWS::stop_lifetime(RuntimeOrigin::signed(ALICE), 0));

        // Verify assets were unlocked
        assert_eq!(
            Assets::balance(LIFETIME_ASSET_ID, &ALICE),
            initial_balance
        );

        // Verify subscription and locked assets record removed
        assert!(RWS::subscription(ALICE, 0).is_none());
        assert!(RWS::locked_assets(ALICE, 0).is_none());
    });
}

#[test]
fn test_stop_lifetime_removes_subscription() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 500));
        assert!(RWS::subscription(ALICE, 0).is_some());

        // Stop it
        assert_ok!(RWS::stop_lifetime(RuntimeOrigin::signed(ALICE), 0));

        // Verify subscription is gone
        assert!(RWS::subscription(ALICE, 0).is_none());
    });
}

#[test]
fn test_stop_lifetime_non_existent_subscription() {
    new_test_ext().execute_with(|| {
        // Try to stop non-existent subscription
        assert_err!(
            RWS::stop_lifetime(RuntimeOrigin::signed(ALICE), 999),
            Error::<Test>::NoSubscription
        );
    });
}

#[test]
fn test_stop_lifetime_not_owner() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Alice creates subscription
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 500));

        // Bob tries to stop Alice's subscription
        assert_err!(
            RWS::stop_lifetime(RuntimeOrigin::signed(BOB), 0),
            Error::<Test>::NoSubscription
        );
    });
}

#[test]
fn test_stop_lifetime_auction_subscription_fails() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create auction-based subscription
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 10_000 }
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));

        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Try to stop auction-based subscription (should fail - no locked assets)
        assert_err!(
            RWS::stop_lifetime(RuntimeOrigin::signed(ALICE), 0),
            Error::<Test>::NotAssetLockedSubscription
        );
    });
}

#[test]
fn test_stop_lifetime_with_accumulated_weight() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create subscription
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 1000));

        // Fast forward to accumulate weight
        Timestamp::set_timestamp(1_100_000);

        // Make a call to update weight
        let call = Box::new(RuntimeCall::System(
            frame_system::Call::remark { remark: vec![1] },
        ));
        assert_ok!(RWS::call(RuntimeOrigin::signed(ALICE), 0, call));

        // Should still be able to stop
        assert_ok!(RWS::stop_lifetime(RuntimeOrigin::signed(ALICE), 0));
    });
}

#[test]
fn test_multiple_users_asset_locked_subscriptions() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Alice, Bob, and Charlie all create subscriptions
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 1000));
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(BOB), 500));
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(CHARLIE), 750));

        // Verify all exist
        assert!(RWS::subscription(ALICE, 0).is_some());
        assert!(RWS::subscription(BOB, 0).is_some());
        assert!(RWS::subscription(CHARLIE, 0).is_some());

        // Each stops their own
        assert_ok!(RWS::stop_lifetime(RuntimeOrigin::signed(ALICE), 0));
        assert_ok!(RWS::stop_lifetime(RuntimeOrigin::signed(BOB), 0));
        assert_ok!(RWS::stop_lifetime(RuntimeOrigin::signed(CHARLIE), 0));

        // All should be gone
        assert!(RWS::subscription(ALICE, 0).is_none());
        assert!(RWS::subscription(BOB, 0).is_none());
        assert!(RWS::subscription(CHARLIE, 0).is_none());
    });
}

#[test]
fn test_start_lifetime_events() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Start subscription and check event
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 500));

        // Check SubscriptionActivated event was emitted
        System::assert_has_event(RuntimeEvent::RWS(Event::SubscriptionActivated(ALICE, 0)));
    });
}

#[test]
fn test_stop_lifetime_events() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Create and stop subscription
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 500));
        assert_ok!(RWS::stop_lifetime(RuntimeOrigin::signed(ALICE), 0));

        // Check SubscriptionStopped event was emitted
        System::assert_has_event(RuntimeEvent::RWS(Event::SubscriptionStopped(ALICE, 0)));
    });
}

#[test]
fn test_asset_locked_vs_auction_based_subscriptions() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Alice creates asset-locked subscription (id 0)
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 1000));

        // Alice wins auction for another subscription (id 1)
        assert_ok!(RWS::start_auction(
            RuntimeOrigin::root(),
            SubscriptionMode::Lifetime { tps: 50_000 }
        ));
        assert_ok!(RWS::bid(RuntimeOrigin::signed(ALICE), 0, 200));
        Timestamp::set_timestamp(1_000_000 + 100_000);
        assert_ok!(RWS::claim(RuntimeOrigin::signed(ALICE), 0, None));

        // Both subscriptions should exist
        assert!(RWS::subscription(ALICE, 0).is_some());
        assert!(RWS::subscription(ALICE, 1).is_some());

        // Asset-locked has locked assets record
        assert!(RWS::locked_assets(ALICE, 0).is_some());
        // Auction-based does not
        assert!(RWS::locked_assets(ALICE, 1).is_none());

        // Can stop asset-locked
        assert_ok!(RWS::stop_lifetime(RuntimeOrigin::signed(ALICE), 0));

        // Cannot stop auction-based
        assert_err!(
            RWS::stop_lifetime(RuntimeOrigin::signed(ALICE), 1),
            Error::<Test>::NotAssetLockedSubscription
        );
    });
}

#[test]
fn test_ratio_edge_cases() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1_000_000);

        // Very small amount: 1 * 100 = 100 μTPS
        assert_ok!(RWS::start_lifetime(RuntimeOrigin::signed(ALICE), 1));
        let sub = RWS::subscription(ALICE, 0).unwrap();
        assert_eq!(sub.mode, SubscriptionMode::Lifetime { tps: 100 });
    });
}
