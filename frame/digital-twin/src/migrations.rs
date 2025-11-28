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
//! Storage migration from DigitalTwin v1 to v2

use frame_support::{pallet_prelude::*, traits::StorageVersion};
use sp_core::H256;
use sp_std::collections::btree_map::BTreeMap;

use crate::{pallet::*, Config};

/// Old v1 storage structure
#[frame_support::storage_alias]
pub type Total<T: Config> = StorageValue<Pallet<T>, u32>;

#[frame_support::storage_alias]
pub type DigitalTwinV1<T: Config> =
    StorageMap<Pallet<T>, Twox64Concat, u32, BTreeMap<H256, <T as frame_system::Config>::AccountId>>;

/// Migrate from v1 to v2
pub fn migrate<T: Config>() -> Weight {
    let on_chain_version = StorageVersion::get::<Pallet<T>>();
    let current_version = Pallet::<T>::in_code_storage_version();

    #[cfg(feature = "std")]
    println!(
        "Running migration from {:?} to {:?}",
        on_chain_version,
        current_version
    );

    if on_chain_version < 2 {
        migrate_v1_to_v2::<T>()
    } else {
        #[cfg(feature = "std")]
        println!("No migration needed");
        T::DbWeight::get().reads(1)
    }
}

/// Migrate v1 storage to v2
fn migrate_v1_to_v2<T: Config>() -> Weight {
    let mut reads = 0u64;
    let mut writes = 0u64;

    #[cfg(feature = "std")]
    println!("Migrating DigitalTwin from v1 to v2");

    // Migrate Total to TwinCount
    if let Some(total) = Total::<T>::take() {
        TwinCount::<T>::put(total);
        reads += 1;
        writes += 1;
        #[cfg(feature = "std")]
        println!("Migrated Total to TwinCount: {}", total);
    }

    // Migrate DigitalTwin BTreeMap storage to new Topics structure
    let _ = DigitalTwinV1::<T>::drain().for_each(|(id, btree_map)| {
        reads += 1;
        #[cfg(feature = "std")]
        println!("Migrating digital twin {}, {} topics", id, btree_map.len());

        let mut topic_list = BoundedVec::new();
        for (topic, source) in btree_map.iter() {
            // Convert old Source mapping to new TopicData::Source
            Topics::<T>::insert(id, topic, TopicData::Source(source.clone()));
            
            // Add to topic list (ignore if max topics exceeded)
            let _ = topic_list.try_push(*topic);
            
            writes += 1;
        }

        TopicList::<T>::insert(id, topic_list);
        writes += 1;
    });

    // Update storage version
    StorageVersion::new(2).put::<Pallet<T>>();
    writes += 1;

    #[cfg(feature = "std")]
    println!("Migration complete");

    T::DbWeight::get().reads_writes(reads, writes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn test_migration_v1_to_v2() {
        new_test_ext().execute_with(|| {
            // Set up v1 storage
            Total::<Runtime>::put(2);
            
            let mut map1 = BTreeMap::new();
            let topic1 = H256::from_low_u64_be(1);
            map1.insert(topic1, 100u64);
            DigitalTwinV1::<Runtime>::insert(0, map1);
            
            let mut map2 = BTreeMap::new();
            let topic2 = H256::from_low_u64_be(2);
            let topic3 = H256::from_low_u64_be(3);
            map2.insert(topic2, 200u64);
            map2.insert(topic3, 300u64);
            DigitalTwinV1::<Runtime>::insert(1, map2);

            // Run migration
            migrate_v1_to_v2::<Runtime>();

            // Verify migration
            assert_eq!(TwinCount::<Runtime>::get(), 2);
            
            // Check stored values manually
            let stored1 = Topics::<Runtime>::get(0, topic1);
            assert!(stored1.is_some());
            if let Some(TopicData::Source(s)) = stored1 {
                assert_eq!(s, 100u64);
            } else {
                panic!("Expected Source variant for topic1");
            }

            let stored2 = Topics::<Runtime>::get(1, topic2);
            assert!(stored2.is_some());
            if let Some(TopicData::Source(s)) = stored2 {
                assert_eq!(s, 200u64);
            } else {
                panic!("Expected Source variant for topic2");
            }

            let stored3 = Topics::<Runtime>::get(1, topic3);
            assert!(stored3.is_some());
            if let Some(TopicData::Source(s)) = stored3 {
                assert_eq!(s, 300u64);
            } else {
                panic!("Expected Source variant for topic3");
            }

            let list1 = TopicList::<Runtime>::get(0);
            assert_eq!(list1.len(), 1);
            assert!(list1.contains(&topic1));

            let list2 = TopicList::<Runtime>::get(1);
            assert_eq!(list2.len(), 2);
            assert!(list2.contains(&topic2));
            assert!(list2.contains(&topic3));
        });
    }
}
