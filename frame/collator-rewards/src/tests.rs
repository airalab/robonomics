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
///////////////////////////////////////////////////////////////////////////////
//! Behavioural tests for `AuthorRewards`.

use crate::mock::{new_test_ext, AccountId, Balances, Test, TestReward};
use crate::AuthorRewards;
use frame_support::traits::fungible::Inspect;
use pallet_authorship::EventHandler;

type Subject = AuthorRewards<Test, TestReward, Balances>;

/// The block author MUST receive the FULL `Reward`, not half of it.
/// `pallet_collator_selection::note_author` only forwards HALF of its pot
/// to the current author, so routing this reward via the pot would
/// silently halve it. This pins the exact end balance to guard against
/// regressions where the reward is accidentally re-routed through the
/// pot.
#[test]
fn block_author_receives_full_reward() {
    new_test_ext().execute_with(|| {
        let author: AccountId = 7;
        assert_eq!(<Balances as Inspect<AccountId>>::balance(&author), 0);

        Subject::note_author(author);

        assert_eq!(
            <Balances as Inspect<AccountId>>::balance(&author),
            4_200_000,
        );
    });
}

#[test]
fn block_author_reward_is_cumulative_across_blocks() {
    new_test_ext().execute_with(|| {
        let author: AccountId = 9;
        for _ in 0..10 {
            Subject::note_author(author);
        }
        assert_eq!(
            <Balances as Inspect<AccountId>>::balance(&author),
            4_200_000 * 10,
        );
    });
}

/// The mandatory weight registered by `note_author` must be reflected in
/// `frame_system::BlockWeight` so the parachain weight limits are
/// respected.
#[test]
fn note_author_registers_extra_weight() {
    new_test_ext().execute_with(|| {
        let before = frame_system::Pallet::<Test>::block_weight().total();
        Subject::note_author(1);
        let after = frame_system::Pallet::<Test>::block_weight().total();
        assert!(
            after.ref_time() > before.ref_time(),
            "extra weight for the mint must be accounted for",
        );
    });
}

/// Different authors accrue independent balances.
#[test]
fn distinct_authors_accrue_independently() {
    new_test_ext().execute_with(|| {
        let a: AccountId = 11;
        let b: AccountId = 22;
        Subject::note_author(a);
        Subject::note_author(a);
        Subject::note_author(b);

        assert_eq!(<Balances as Inspect<AccountId>>::balance(&a), 4_200_000 * 2);
        assert_eq!(<Balances as Inspect<AccountId>>::balance(&b), 4_200_000);
    });
}
