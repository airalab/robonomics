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
//! Tests for Wrapped Native pallet.

use crate::{mock::*, Error, Event};
use frame_support::{assert_err, assert_ok};
use xcm::latest::prelude::*;

#[test]
fn test_successful_unwrap() {
    new_test_ext().execute_with(|| {
        // Initial state: TotalWrapped = 0
        assert_eq!(WrappedNative::total_wrapped(), 0);
        assert_eq!(Balances::free_balance(ALICE), 1000);

        // Call handle_incoming_unwrap
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 100));

        // Verify native tokens minted
        assert_eq!(Balances::free_balance(ALICE), 1100);
        
        // Verify TotalWrapped incremented
        assert_eq!(WrappedNative::total_wrapped(), 100);

        // Verify event emitted
        System::assert_last_event(
            Event::NativeUnwrapped {
                who: ALICE,
                amount: 100,
            }
            .into(),
        );
    });
}

#[test]
fn test_successful_wrap_and_send() {
    new_test_ext().execute_with(|| {
        // Setup: First unwrap to get TotalWrapped = 100
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 100));
        assert_eq!(WrappedNative::total_wrapped(), 100);
        assert_eq!(Balances::free_balance(ALICE), 1100);

        // Wrap 50 tokens
        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(ALICE),
            50,
            None
        ));

        // Verify 50 native tokens burned
        assert_eq!(Balances::free_balance(ALICE), 1050);
        
        // Verify TotalWrapped decremented
        assert_eq!(WrappedNative::total_wrapped(), 50);

        // Verify event emitted
        System::assert_has_event(
            Event::NativeWrapped {
                who: ALICE,
                amount: 50,
                destination: Location::new(
                    0,
                    [Junction::AccountId32 {
                        network: None,
                        id: {
                            let mut bytes = [0u8; 32];
                            bytes[0] = 1; // ALICE = 1
                            bytes
                        },
                    }],
                ),
            }
            .into(),
        );
    });
}

#[test]
fn test_wrap_with_insufficient_wrapped_balance() {
    new_test_ext().execute_with(|| {
        // Initial state: TotalWrapped = 10
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 10));
        assert_eq!(WrappedNative::total_wrapped(), 10);

        // Try to wrap 50 tokens (more than TotalWrapped)
        assert_err!(
            WrappedNative::wrap_and_send(RuntimeOrigin::signed(ALICE), 50, None),
            Error::<Test>::InsufficientWrappedBalance
        );

        // Verify no state changes
        assert_eq!(Balances::free_balance(ALICE), 1010);
        assert_eq!(WrappedNative::total_wrapped(), 10);
    });
}

#[test]
fn test_wrap_with_zero_total_wrapped() {
    new_test_ext().execute_with(|| {
        // Initial state: TotalWrapped = 0
        assert_eq!(WrappedNative::total_wrapped(), 0);

        // Try to wrap any amount
        assert_err!(
            WrappedNative::wrap_and_send(RuntimeOrigin::signed(ALICE), 50, None),
            Error::<Test>::InsufficientWrappedBalance
        );

        // Verify no state changes
        assert_eq!(Balances::free_balance(ALICE), 1000);
        assert_eq!(WrappedNative::total_wrapped(), 0);
    });
}

#[test]
fn test_wrap_with_insufficient_native_balance() {
    new_test_ext().execute_with(|| {
        // Setup: TotalWrapped = 100
        assert_ok!(WrappedNative::handle_incoming_unwrap(BOB, 100));
        assert_eq!(WrappedNative::total_wrapped(), 100);

        // ALICE has 1000 tokens, try to wrap 1001
        assert_err!(
            WrappedNative::wrap_and_send(RuntimeOrigin::signed(ALICE), 1001, None),
            Error::<Test>::InsufficientBalance
        );

        // Verify no state changes
        assert_eq!(Balances::free_balance(ALICE), 1000);
        assert_eq!(WrappedNative::total_wrapped(), 100);
    });
}

#[test]
fn test_wrap_with_zero_amount() {
    new_test_ext().execute_with(|| {
        // Setup: TotalWrapped = 100
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 100));

        // Try to wrap 0 tokens
        assert_err!(
            WrappedNative::wrap_and_send(RuntimeOrigin::signed(ALICE), 0, None),
            Error::<Test>::InvalidAmount
        );

        // Verify no state changes
        assert_eq!(Balances::free_balance(ALICE), 1100);
        assert_eq!(WrappedNative::total_wrapped(), 100);
    });
}

#[test]
fn test_default_beneficiary() {
    new_test_ext().execute_with(|| {
        // Setup: TotalWrapped = 100
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 100));

        // Wrap with None beneficiary (should default to caller)
        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(ALICE),
            50,
            None
        ));

        // Check event has correct destination (ALICE's AccountId as MultiLocation)
        System::assert_has_event(
            Event::NativeWrapped {
                who: ALICE,
                amount: 50,
                destination: Location::new(
                    0,
                    [Junction::AccountId32 {
                        network: None,
                        id: {
                            let mut bytes = [0u8; 32];
                            bytes[0] = 1; // ALICE = 1
                            bytes
                        },
                    }],
                ),
            }
            .into(),
        );
    });
}

#[test]
fn test_custom_beneficiary() {
    new_test_ext().execute_with(|| {
        // Setup: TotalWrapped = 100
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 100));

        // Custom beneficiary location
        let custom_dest = Location::new(
            1,
            [Junction::AccountId32 {
                network: None,
                id: [0xAB; 32],
            }],
        );

        // Wrap with custom beneficiary
        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(ALICE),
            50,
            Some(custom_dest.clone())
        ));

        // Check event has correct destination
        System::assert_has_event(
            Event::NativeWrapped {
                who: ALICE,
                amount: 50,
                destination: custom_dest,
            }
            .into(),
        );
    });
}

#[test]
fn test_round_trip() {
    new_test_ext().execute_with(|| {
        // Start: TotalWrapped = 0, ALICE has 1000
        assert_eq!(WrappedNative::total_wrapped(), 0);
        assert_eq!(Balances::free_balance(ALICE), 1000);

        // Step 1: Unwrap 100 (mint native, TotalWrapped = 100)
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 100));
        assert_eq!(WrappedNative::total_wrapped(), 100);
        assert_eq!(Balances::free_balance(ALICE), 1100);

        // Step 2: Wrap 50 (burn native, TotalWrapped = 50)
        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(ALICE),
            50,
            None
        ));
        assert_eq!(WrappedNative::total_wrapped(), 50);
        assert_eq!(Balances::free_balance(ALICE), 1050);

        // Step 3: Wrap 50 (burn native, TotalWrapped = 0)
        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(ALICE),
            50,
            None
        ));
        assert_eq!(WrappedNative::total_wrapped(), 0);
        assert_eq!(Balances::free_balance(ALICE), 1000);

        // Step 4: Try to wrap more - should fail
        assert_err!(
            WrappedNative::wrap_and_send(RuntimeOrigin::signed(ALICE), 10, None),
            Error::<Test>::InsufficientWrappedBalance
        );

        // Final state unchanged
        assert_eq!(WrappedNative::total_wrapped(), 0);
        assert_eq!(Balances::free_balance(ALICE), 1000);
    });
}

#[test]
fn test_query_functions() {
    new_test_ext().execute_with(|| {
        // Initial state
        assert_eq!(WrappedNative::get_total_wrapped(), 0);
        assert_eq!(WrappedNative::max_wrappable(), 0);
        assert!(!WrappedNative::can_wrap(50));
        assert!(!WrappedNative::can_wrap(0)); // Zero amount

        // Unwrap 100
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 100));

        // Check query functions
        assert_eq!(WrappedNative::get_total_wrapped(), 100);
        assert_eq!(WrappedNative::max_wrappable(), 100);
        assert!(WrappedNative::can_wrap(50));
        assert!(WrappedNative::can_wrap(100));
        assert!(!WrappedNative::can_wrap(101));
        assert!(!WrappedNative::can_wrap(0)); // Zero amount

        // Wrap 30
        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(ALICE),
            30,
            None
        ));

        // Check query functions again
        assert_eq!(WrappedNative::get_total_wrapped(), 70);
        assert_eq!(WrappedNative::max_wrappable(), 70);
        assert!(WrappedNative::can_wrap(50));
        assert!(WrappedNative::can_wrap(70));
        assert!(!WrappedNative::can_wrap(71));
    });
}

#[test]
fn test_multiple_users() {
    new_test_ext().execute_with(|| {
        // Unwrap for multiple users
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 100));
        assert_ok!(WrappedNative::handle_incoming_unwrap(BOB, 50));
        assert_ok!(WrappedNative::handle_incoming_unwrap(CHARLIE, 25));

        // TotalWrapped should be sum of all
        assert_eq!(WrappedNative::total_wrapped(), 175);

        // Each user should have their balance
        assert_eq!(Balances::free_balance(ALICE), 1100);
        assert_eq!(Balances::free_balance(BOB), 1050);
        assert_eq!(Balances::free_balance(CHARLIE), 1025);

        // ALICE wraps 50
        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(ALICE),
            50,
            None
        ));
        assert_eq!(WrappedNative::total_wrapped(), 125);
        assert_eq!(Balances::free_balance(ALICE), 1050);

        // BOB wraps 25
        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(BOB),
            25,
            None
        ));
        assert_eq!(WrappedNative::total_wrapped(), 100);
        assert_eq!(Balances::free_balance(BOB), 1025);

        // CHARLIE tries to wrap 101 (more than TotalWrapped)
        assert_err!(
            WrappedNative::wrap_and_send(RuntimeOrigin::signed(CHARLIE), 101, None),
            Error::<Test>::InsufficientWrappedBalance
        );
    });
}

#[test]
fn test_unwrap_increments_correctly() {
    new_test_ext().execute_with(|| {
        // Multiple unwraps should accumulate
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 25));
        assert_eq!(WrappedNative::total_wrapped(), 25);

        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 25));
        assert_eq!(WrappedNative::total_wrapped(), 50);

        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 50));
        assert_eq!(WrappedNative::total_wrapped(), 100);

        assert_eq!(Balances::free_balance(ALICE), 1100);
    });
}

#[test]
fn test_wrap_decrements_correctly() {
    new_test_ext().execute_with(|| {
        // Setup
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 100));
        assert_eq!(WrappedNative::total_wrapped(), 100);

        // Multiple wraps should decrement
        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(ALICE),
            25,
            None
        ));
        assert_eq!(WrappedNative::total_wrapped(), 75);

        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(ALICE),
            25,
            None
        ));
        assert_eq!(WrappedNative::total_wrapped(), 50);

        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(ALICE),
            50,
            None
        ));
        assert_eq!(WrappedNative::total_wrapped(), 0);
    });
}

#[test]
fn test_wrap_exact_amount() {
    new_test_ext().execute_with(|| {
        // Setup
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 100));

        // Wrap exact amount available
        assert_ok!(WrappedNative::wrap_and_send(
            RuntimeOrigin::signed(ALICE),
            100,
            None
        ));

        assert_eq!(WrappedNative::total_wrapped(), 0);
        assert_eq!(Balances::free_balance(ALICE), 1000);
    });
}

#[test]
fn test_wrap_one_more_than_available() {
    new_test_ext().execute_with(|| {
        // Setup
        assert_ok!(WrappedNative::handle_incoming_unwrap(ALICE, 100));

        // Try to wrap 1 more than available
        assert_err!(
            WrappedNative::wrap_and_send(RuntimeOrigin::signed(ALICE), 101, None),
            Error::<Test>::InsufficientWrappedBalance
        );
    });
}
