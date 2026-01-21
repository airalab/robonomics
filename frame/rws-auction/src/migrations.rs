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
//! Storage migrations for the RWS pallet.

use super::*;
use frame_support::{
    pallet_prelude::*,
    storage_alias,
    traits::{Currency, Get, OnRuntimeUpgrade, StorageVersion, Time},
    weights::Weight,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use sp_std::prelude::*;

/// Migration from v1 to v2 storage format.
pub mod v2 {
    use super::*;

    /// V1 Subscription type (renamed to SubscriptionMode in v2).
    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
    pub enum SubscriptionV1 {
        /// Lifetime subscription.
        Lifetime {
            /// How much Transactions Per Second this subscription gives (in uTPS).
            #[codec(compact)]
            tps: u32,
        },
        /// Daily subscription (each daily subscription have 1 TPS).
        Daily {
            /// How long days this subscription active.
            #[codec(compact)]
            days: u32,
        },
    }

    /// V1 AuctionLedger structure (without created, subscription_id fields).
    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
    pub struct AuctionLedgerV1<AccountId: MaxEncodedLen, Balance: HasCompact + MaxEncodedLen> {
        /// Auction winner address.
        pub winner: Option<AccountId>,
        /// Current best price.
        #[codec(compact)]
        pub best_price: Balance,
        /// Kind of subscription for this auction
        pub kind: SubscriptionV1,
    }

    /// V1 SubscriptionLedger structure (kind instead of mode).
    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
    pub struct SubscriptionLedgerV1<Moment: HasCompact + MaxEncodedLen> {
        /// Free execution weights accumulator.
        #[codec(compact)]
        pub free_weight: u64,
        /// Subscription creation timestamp.
        #[codec(compact)]
        pub issue_time: Moment,
        /// Moment of time for last subscription update (used for TPS estimation).
        #[codec(compact)]
        pub last_update: Moment,
        /// Kind of subscription (lifetime, daily, etc).
        pub kind: SubscriptionV1,
    }

    // Storage aliases for v1 storage items
    #[storage_alias]
    pub type Oracle<T: Config> = StorageValue<Pallet<T>, <T as frame_system::Config>::AccountId>;

    #[storage_alias]
    pub type Ledger<T: Config> = StorageMap<
        Pallet<T>,
        Twox64Concat,
        <T as frame_system::Config>::AccountId,
        SubscriptionLedgerV1<<<T as Config>::Time as Time>::Moment>,
    >;

    #[storage_alias]
    pub type Devices<T: Config> = StorageMap<
        Pallet<T>,
        Twox64Concat,
        <T as frame_system::Config>::AccountId,
        BoundedVec<<T as frame_system::Config>::AccountId, ConstU32<32>>,
    >;

    #[storage_alias]
    pub type AuctionQueue<T: Config> = StorageValue<Pallet<T>, BoundedVec<u32, ConstU32<100>>>;

    #[storage_alias]
    pub type AuctionNext<T: Config> = StorageValue<Pallet<T>, u32>;

    #[storage_alias]
    pub type AuctionV1<T: Config> =
        StorageMap<
            Pallet<T>,
            Twox64Concat,
            u32,
            AuctionLedgerV1<
                <T as frame_system::Config>::AccountId,
                <<T as Config>::AuctionCurrency as Currency<
                    <T as frame_system::Config>::AccountId,
                >>::Balance,
            >,
        >;

    #[storage_alias]
    pub type UnspendBondValue<T: Config> = StorageValue<
        Pallet<T>,
        <<T as Config>::AuctionCurrency as Currency<
            <T as frame_system::Config>::AccountId,
        >>::Balance,
    >;

    /// Convert v1 SubscriptionV1 to v2 SubscriptionMode.
    fn convert_subscription_mode(v1: SubscriptionV1) -> SubscriptionMode {
        match v1 {
            SubscriptionV1::Lifetime { tps } => SubscriptionMode::Lifetime { tps },
            SubscriptionV1::Daily { days } => SubscriptionMode::Daily { days },
        }
    }

    /// Migrate RWS pallet storage from v1 to v2.
    pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
        fn on_runtime_upgrade() -> Weight {
            let onchain_version = Pallet::<T>::on_chain_storage_version();
            let current_version = Pallet::<T>::in_code_storage_version();

            log::info!(
                "🔄 RWS migration: on-chain version: {onchain_version:?}, current version: {current_version:?}"
            );

            if onchain_version == 1 && current_version == 2 {
                log::info!("🔄 Migrating RWS pallet from v1 to v2...");

                let mut weight = T::DbWeight::get().reads(1);
                let mut migrated_subscriptions = 0u32;
                let mut migrated_auctions = 0u32;

                // Migrate Ledger to Subscription double map
                // In v1: Ledger<AccountId -> SubscriptionLedger>
                // In v2: Subscription<AccountId, u32 -> SubscriptionLedger>
                // Strategy: All v1 subscriptions get subscription_id = 0
                for (account, old_ledger) in Ledger::<T>::drain() {
                    let mode = convert_subscription_mode(old_ledger.kind.clone());
                    let expiration_time = match &old_ledger.kind {
                        SubscriptionV1::Daily { days } => {
                            let duration_ms =
                                <T::Time as Time>::Moment::from(days * crate::DAYS_TO_MS);
                            Some(old_ledger.issue_time.clone() + duration_ms)
                        }
                        SubscriptionV1::Lifetime { .. } => None,
                    };
                    let new_ledger = crate::SubscriptionLedger {
                        free_weight: old_ledger.free_weight,
                        issue_time: old_ledger.issue_time,
                        last_update: old_ledger.last_update,
                        mode,
                        expiration_time,
                    };
                    crate::Subscription::<T>::insert(&account, 0u32, new_ledger);
                    migrated_subscriptions += 1;
                }
                weight = weight.saturating_add(
                    T::DbWeight::get()
                        .reads_writes(migrated_subscriptions as u64, migrated_subscriptions as u64),
                );

                // Migrate Auction storage
                // In v1: Auction<AuctionIndex -> AuctionLedger> (without created, subscription_id, first_bid_time)
                // In v2: Auction<u32 -> AuctionLedger> (with first_bid_time, subscription_id, mode instead of kind)
                // Strategy: Set first_bid_time to None (we don't know when bidding started), subscription_id to None
                for (auction_id, old_auction) in AuctionV1::<T>::drain() {
                    let new_auction = crate::AuctionLedger {
                        winner: old_auction.winner,
                        best_price: old_auction.best_price,
                        first_bid_time: None,
                        mode: convert_subscription_mode(old_auction.kind),
                        subscription_id: None,
                    };
                    crate::Auction::<T>::insert(auction_id, new_auction);
                    migrated_auctions += 1;
                }
                weight = weight.saturating_add(
                    T::DbWeight::get()
                        .reads_writes(migrated_auctions as u64, migrated_auctions as u64),
                );

                // Clear deprecated storage items
                Oracle::<T>::kill();
                weight = weight.saturating_add(T::DbWeight::get().writes(1));

                // Clear Devices storage in batches to avoid unbounded operations.
                // This attempts to clear up to MAX_REMOVALS entries. If there are more entries,
                // they will remain and should be manually cleaned or handled in a follow-up migration.
                const MAX_REMOVALS: u32 = 1000;
                let clear_result = Devices::<T>::clear(MAX_REMOVALS, None);
                let devices_count = clear_result.unique as u64;
                weight = weight.saturating_add(T::DbWeight::get().writes(devices_count));

                // Check if there might be more entries to clear (cursor indicates incomplete clearing)
                if clear_result.maybe_cursor.is_some() {
                    log::warn!(
                        "[WARNING] Devices storage not fully cleared in migration. {} entries cleared, but more remain. \
                        Consider running a follow-up migration or manual cleanup.",
                        clear_result.unique
                    );
                }

                AuctionQueue::<T>::kill();
                weight = weight.saturating_add(T::DbWeight::get().writes(1));

                AuctionNext::<T>::kill();
                weight = weight.saturating_add(T::DbWeight::get().writes(1));

                UnspendBondValue::<T>::kill();
                weight = weight.saturating_add(T::DbWeight::get().writes(1));

                // Update storage version
                StorageVersion::new(2).put::<Pallet<T>>();
                weight = weight.saturating_add(T::DbWeight::get().writes(1));

                log::info!(
                    "✅ RWS migration complete: {migrated_subscriptions} subscriptions, {migrated_auctions} auctions migrated"
                );

                weight
            } else {
                log::info!("🔄 RWS migration skipped: no migration needed");
                T::DbWeight::get().reads(1)
            }
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            let onchain_version = Pallet::<T>::on_chain_storage_version();
            if onchain_version == 1 {
                let ledger_count = Ledger::<T>::iter().count() as u32;
                let auction_count = AuctionV1::<T>::iter().count() as u32;
                let devices_count = Devices::<T>::iter().count() as u32;

                log::info!(
                    "Pre-upgrade: {} subscriptions, {} auctions, {} devices",
                    ledger_count,
                    auction_count,
                    devices_count
                );

                Ok((ledger_count, auction_count, devices_count).encode())
            } else {
                Ok(Vec::new())
            }
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
            let onchain_version = Pallet::<T>::on_chain_storage_version();
            if onchain_version == 2 {
                if !state.is_empty() {
                    let (old_ledger_count, old_auction_count, old_devices_count): (u32, u32, u32) =
                        Decode::decode(&mut &state[..])
                            .map_err(|_| "Failed to decode pre-upgrade state")?;

                    // Check all subscriptions migrated
                    let mut actual_subscription_count = 0u32;
                    for _ in crate::Subscription::<T>::iter() {
                        actual_subscription_count += 1;
                    }

                    ensure!(
                        actual_subscription_count >= old_ledger_count,
                        "Not all subscriptions were migrated"
                    );

                    // Check all auctions migrated
                    let new_auction_count = crate::Auction::<T>::iter().count() as u32;
                    ensure!(
                        new_auction_count == old_auction_count,
                        "Not all auctions were migrated"
                    );

                    // Verify old storage is cleared
                    ensure!(Ledger::<T>::iter().count() == 0, "Old Ledger not cleared");
                    ensure!(
                        AuctionV1::<T>::iter().count() == 0,
                        "Old Auction not cleared"
                    );
                    ensure!(Devices::<T>::iter().count() == 0, "Devices not cleared");
                    ensure!(Oracle::<T>::get().is_none(), "Oracle not cleared");
                    ensure!(
                        AuctionQueue::<T>::get().is_none(),
                        "AuctionQueue not cleared"
                    );
                    ensure!(AuctionNext::<T>::get().is_none(), "AuctionNext not cleared");
                    ensure!(
                        UnspendBondValue::<T>::get().is_none(),
                        "UnspendBondValue not cleared"
                    );

                    log::info!(
                        "Post-upgrade: {} subscriptions migrated successfully",
                        actual_subscription_count
                    );
                }
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mock::*, SubscriptionMode};
    use frame_support::traits::OnRuntimeUpgrade;

    #[test]
    fn test_migration_v1_to_v2() {
        new_test_ext().execute_with(|| {
            // Set storage version to v1
            StorageVersion::new(1).put::<Pallet<Test>>();

            // Prepare v1 storage data
            let account1 = 1u64;
            let account2 = 2u64;

            // Insert v1 Ledger entries
            let ledger1 = v2::SubscriptionLedgerV1 {
                free_weight: 1000,
                issue_time: 100u64,
                last_update: 100u64,
                kind: v2::SubscriptionV1::Lifetime { tps: 500 },
            };
            v2::Ledger::<Test>::insert(account1, ledger1);

            let ledger2 = v2::SubscriptionLedgerV1 {
                free_weight: 2000,
                issue_time: 200u64,
                last_update: 200u64,
                kind: v2::SubscriptionV1::Daily { days: 30 },
            };
            v2::Ledger::<Test>::insert(account2, ledger2);

            // Insert v1 Auction entries
            let auction1 = v2::AuctionLedgerV1 {
                winner: Some(account1),
                best_price: 1000u128,
                kind: v2::SubscriptionV1::Lifetime { tps: 100 },
            };
            v2::AuctionV1::<Test>::insert(0u32, auction1);

            // Insert deprecated storage
            v2::Oracle::<Test>::put(account1);
            v2::Devices::<Test>::insert(
                account1,
                sp_runtime::BoundedVec::try_from(vec![account2])
                    .expect("test vector is within bounds"),
            );
            v2::AuctionQueue::<Test>::put(
                sp_runtime::BoundedVec::try_from(vec![0u32]).expect("test vector is within bounds"),
            );
            v2::AuctionNext::<Test>::put(1u32);
            v2::UnspendBondValue::<Test>::put(500u128);

            // Verify v1 storage exists
            assert!(v2::Ledger::<Test>::get(account1).is_some());
            assert!(v2::Ledger::<Test>::get(account2).is_some());
            assert!(v2::AuctionV1::<Test>::get(0u32).is_some());
            assert!(v2::Oracle::<Test>::get().is_some());

            // Run migration
            let weight = v2::MigrateToV2::<Test>::on_runtime_upgrade();
            assert!(weight != Weight::zero());

            // Verify v2 storage
            assert_eq!(Pallet::<Test>::on_chain_storage_version(), 2);

            // Check subscriptions migrated to double map with id = 0
            let sub1 = crate::Subscription::<Test>::get(account1, 0u32).unwrap();
            assert_eq!(sub1.free_weight, 1000);
            assert_eq!(sub1.issue_time, 100u64);
            assert_eq!(sub1.mode, SubscriptionMode::Lifetime { tps: 500 });

            let sub2 = crate::Subscription::<Test>::get(account2, 0u32).unwrap();
            assert_eq!(sub2.free_weight, 2000);
            assert_eq!(sub2.issue_time, 200u64);
            assert_eq!(sub2.mode, SubscriptionMode::Daily { days: 30 });

            // Check auctions migrated with new fields
            let new_auction = crate::Auction::<Test>::get(0u32).unwrap();
            assert_eq!(new_auction.winner, Some(account1));
            assert_eq!(new_auction.best_price, 1000u128);
            assert_eq!(new_auction.mode, SubscriptionMode::Lifetime { tps: 100 });
            assert!(new_auction.subscription_id.is_none());
            // first_bid_time should be None for migrated auctions since we don't know when bidding started
            assert!(new_auction.first_bid_time.is_none());

            // Verify old storage is cleared
            assert!(v2::Ledger::<Test>::get(account1).is_none());
            assert!(v2::Ledger::<Test>::get(account2).is_none());
            assert!(v2::AuctionV1::<Test>::get(0u32).is_none());
            assert!(v2::Oracle::<Test>::get().is_none());
            assert!(v2::Devices::<Test>::get(account1).is_none());
            assert!(v2::AuctionQueue::<Test>::get().is_none());
            assert!(v2::AuctionNext::<Test>::get().is_none());
            assert!(v2::UnspendBondValue::<Test>::get().is_none());
        });
    }

    #[test]
    fn test_migration_idempotency() {
        new_test_ext().execute_with(|| {
            // Set storage version to v1
            StorageVersion::new(1).put::<Pallet<Test>>();

            // Insert v1 data
            let account = 1u64;
            let ledger = v2::SubscriptionLedgerV1 {
                free_weight: 1000,
                issue_time: 100u64,
                last_update: 100u64,
                kind: v2::SubscriptionV1::Lifetime { tps: 500 },
            };
            v2::Ledger::<Test>::insert(account, ledger);

            // Run migration once
            let weight1 = v2::MigrateToV2::<Test>::on_runtime_upgrade();
            assert!(weight1 != Weight::zero());
            assert_eq!(Pallet::<Test>::on_chain_storage_version(), 2);

            // Verify data migrated
            assert!(crate::Subscription::<Test>::get(account, 0u32).is_some());

            // Run migration again - should be no-op
            let weight2 = v2::MigrateToV2::<Test>::on_runtime_upgrade();
            // Should be minimal weight (only reading storage version)
            assert!(weight2 != Weight::zero());

            // Data should still be there
            assert!(crate::Subscription::<Test>::get(account, 0u32).is_some());
        });
    }

    #[test]
    fn test_migration_empty_storage() {
        new_test_ext().execute_with(|| {
            // Set storage version to v1 but no data
            StorageVersion::new(1).put::<Pallet<Test>>();

            // Run migration
            let weight = v2::MigrateToV2::<Test>::on_runtime_upgrade();
            assert!(weight != Weight::zero());

            // Should update version
            assert_eq!(Pallet::<Test>::on_chain_storage_version(), 2);
        });
    }
}

/// Migration from v2 to v3 storage format.
pub mod v3 {
    use super::*;

    /// V2 SubscriptionLedger structure (without new transaction tracking fields).
    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
    pub struct SubscriptionLedgerV2<Moment: HasCompact + MaxEncodedLen> {
        /// Free execution weights accumulator.
        #[codec(compact)]
        pub free_weight: u64,
        /// Subscription creation timestamp.
        #[codec(compact)]
        pub issue_time: Moment,
        /// Moment of time for last subscription update (used for TPS estimation).
        #[codec(compact)]
        pub last_update: Moment,
        /// Type of subscription (lifetime, daily, etc).
        pub mode: SubscriptionMode,
        /// Expiration timestamp for Daily subscriptions.
        pub expiration_time: Option<Moment>,
    }

    pub struct MigrateToV3<T>(PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = Pallet::<T>::on_chain_storage_version();
            let current_version = Pallet::<T>::current_storage_version();

            log::info!(
                "🔧 RWS pallet migration: on-chain version: {:?}, current version: {:?}",
                on_chain_version,
                current_version
            );

            if on_chain_version < 3 {
                log::info!("🔧 Migrating RWS pallet from v{:?} to v3", on_chain_version);

                let mut migrated_count = 0u32;
                let mut reads = 0u64;
                let mut writes = 0u64;

                // Migrate all subscriptions to add new fields
                crate::Subscription::<T>::translate::<SubscriptionLedgerV2<<T::Time as Time>::Moment>, _>(
                    |owner, subscription_id, old_sub| {
                        reads += 1;
                        writes += 1;
                        migrated_count += 1;

                        log::info!(
                            "🔧 Migrating subscription for owner {:?}, id: {}",
                            owner,
                            subscription_id
                        );

                        Some(SubscriptionLedger {
                            free_weight: old_sub.free_weight,
                            issue_time: old_sub.issue_time,
                            last_update: old_sub.last_update,
                            mode: old_sub.mode,
                            expiration_time: old_sub.expiration_time,
                            transactions_used: 0,
                            transactions_limit: None,
                            is_active: true,
                        })
                    },
                );

                log::info!(
                    "✅ RWS pallet migration complete: migrated {} subscriptions",
                    migrated_count
                );

                // Update storage version
                current_version.put::<Pallet<T>>();
                writes += 1;

                T::DbWeight::get().reads_writes(reads, writes)
            } else {
                log::info!("✅ RWS pallet already at v3, no migration needed");
                T::DbWeight::get().reads(1)
            }
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
            use parity_scale_codec::Encode;

            let on_chain_version = Pallet::<T>::on_chain_storage_version();
            ensure!(on_chain_version < 3, "Migration not needed");

            let mut count = 0u32;
            for (owner, subscription_id, _) in crate::Subscription::<T>::iter() {
                count += 1;
                log::info!(
                    "Pre-upgrade: subscription for {:?}, id: {}",
                    owner,
                    subscription_id
                );
            }

            log::info!("Pre-upgrade: found {} subscriptions to migrate", count);
            Ok(count.encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
            use parity_scale_codec::Decode;

            let pre_count: u32 = Decode::decode(&mut &state[..])
                .map_err(|_| "Failed to decode pre-upgrade state")?;

            let mut post_count = 0u32;
            for (owner, subscription_id, sub) in crate::Subscription::<T>::iter() {
                post_count += 1;

                // Verify new fields have default values
                ensure!(
                    sub.transactions_used == 0,
                    "transactions_used not initialized"
                );
                ensure!(
                    sub.transactions_limit.is_none(),
                    "transactions_limit not initialized"
                );
                ensure!(sub.is_active, "is_active not initialized");

                log::info!(
                    "Post-upgrade: subscription for {:?}, id: {}, active: {}",
                    owner,
                    subscription_id,
                    sub.is_active
                );
            }

            ensure!(
                pre_count == post_count,
                "Subscription count mismatch after migration"
            );

            let new_version = Pallet::<T>::on_chain_storage_version();
            ensure!(new_version == 3, "Storage version not updated");

            log::info!(
                "Post-upgrade: verified {} subscriptions migrated to v3",
                post_count
            );

            Ok(())
        }
    }
}
