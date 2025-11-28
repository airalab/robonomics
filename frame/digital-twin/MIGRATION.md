# Digital Twin Pallet v2 - Migration Guide

## Overview

Digital Twin v2 is a unified pallet that consolidates functionality from three separate pallets:
- **DigitalTwin v1** - Source mappings for digital twins
- **Datalog** - Timestamped data records  
- **Launch** - Robot launch commands

The v2 design provides a clean, type-safe API through a generic `TopicData` enum that supports multiple topic types.

## What Changed

### Storage Structure

**v1 Storage:**
```rust
Total<T>: StorageValue<u32>
Owner<T>: StorageMap<Twox64Concat, u32, AccountId>
DigitalTwin<T>: StorageMap<Twox64Concat, u32, BTreeMap<H256, AccountId>>  // Unbounded!
```

**v2 Storage:**
```rust
TwinCount<T>: StorageValue<u32>
Owner<T>: StorageMap<Blake2_128Concat, u32, AccountId>
Topics<T>: StorageDoubleMap<Blake2_128Concat, u32, Blake2_128Concat, H256, TopicData<T>>
TopicList<T>: StorageMap<Blake2_128Concat, u32, BoundedVec<H256, MaxTopicsPerTwin>>
```

### Key Improvements

1. **Fully Bounded Storage** - Removed `#[pallet::without_storage_info]` attribute
2. **Better Hashing** - Upgraded from `Twox64Concat` to `Blake2_128Concat` for improved security
3. **Efficient Lookups** - `StorageDoubleMap` allows single-read access to topic data
4. **Topic Enumeration** - `TopicList` enables UI-friendly iteration over all topics
5. **Type Safety** - `TopicData` enum prevents invalid topic type combinations

### TopicData Enum

```rust
pub enum TopicData<T: Config> {
    /// Data source mapping (from DigitalTwin v1)
    Source(T::AccountId),
    
    /// Timestamped data record (from Datalog)
    Data {
        timestamp: <T::Time as Time>::Moment,
        data: BoundedVec<u8, T::MaxDataSize>,
    },
    
    /// Robot launch command (from Launch)
    Command {
        target: T::AccountId,
        params: BoundedVec<u8, T::MaxDataSize>,
    },
}
```

### API Changes

**v1 API:**
```rust
fn create(origin) -> DispatchResult
fn set_source(origin, id, topic, source) -> DispatchResult
fn remove_source(origin, id, topic, source) -> DispatchResult
```

**v2 API:**
```rust
fn create(origin) -> DispatchResult
fn set_source(origin, id, topic, source) -> DispatchResult
fn set_data(origin, id, topic, data) -> DispatchResult
fn set_command(origin, id, topic, target, params) -> DispatchResult
fn remove_topic(origin, id, topic) -> DispatchResult
```

### Error Handling

**v1:** String errors
```rust
ensure!(condition, "sender should be a twin owner")
```

**v2:** Proper error enum
```rust
#[pallet::error]
pub enum Error<T> {
    NotOwner,
    TwinNotFound,
    TopicNotFound,
    TooManyTopics,
}
```

## Migration from v1 to v2

The pallet includes automatic migration logic that runs on runtime upgrade:

1. Migrates `Total` → `TwinCount`
2. Converts `DigitalTwin` BTreeMap → `Topics` StorageDoubleMap
3. Creates `TopicList` entries for each twin
4. Updates storage version from 1 → 2

### Testing Migration

```rust
// Set up v1 storage
Total::<T>::put(1);
let mut map = BTreeMap::new();
map.insert(topic, source_account);
DigitalTwin::<T>::insert(0, map);

// Trigger migration (happens automatically on runtime upgrade)
migrations::migrate::<T>();

// Verify v2 storage
assert_eq!(TwinCount::<T>::get(), 1);
assert_eq!(Topics::<T>::get(0, topic), Some(TopicData::Source(source_account)));
```

## Usage Examples

### Creating a Digital Twin
```rust
DigitalTwin::create(RuntimeOrigin::signed(owner))?;
```

### Setting a Source (v1 compatibility)
```rust
let topic = H256::from([1u8; 32]);
DigitalTwin::set_source(
    RuntimeOrigin::signed(owner),
    twin_id,
    topic,
    source_account
)?;
```

### Recording Data (Datalog functionality)
```rust
let data = BoundedVec::try_from(b"sensor reading: 42".to_vec())?;
DigitalTwin::set_data(
    RuntimeOrigin::signed(owner),
    twin_id,
    topic,
    data
)?;
```

### Launching a Command (Launch functionality)
```rust
let params = BoundedVec::try_from(b"start mission".to_vec())?;
DigitalTwin::set_command(
    RuntimeOrigin::signed(owner),
    twin_id,
    topic,
    robot_account,
    params
)?;
```

### Removing a Topic
```rust
DigitalTwin::remove_topic(
    RuntimeOrigin::signed(owner),
    twin_id,
    topic
)?;
```

## Configuration

Add these constants to your runtime configuration:

```rust
impl pallet_robonomics_digital_twin::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Time = Timestamp;
    type MaxDataSize = ConstU32<512>;        // Max bytes for data/params
    type MaxTopicsPerTwin = ConstU32<100>;   // Max topics per twin
    type WeightInfo = ();
}
```

## Future Extensibility

The enum-based design makes it easy to add new topic types:

```rust
pub enum TopicData<T: Config> {
    Source(T::AccountId),
    Data { timestamp: Moment, data: BoundedVec },
    Command { target: AccountId, params: BoundedVec },
    
    // Future additions:
    XcmCallback { dest: MultiLocation, message: Xcm<()> },
    ZkProof { proof: BoundedVec<u8>, public_inputs: BoundedVec<u8> },
    // ... etc
}
```

## Breaking Changes

⚠️ **This is a breaking change from v1:**

- Storage structure is completely different
- `set_source` and `remove_source` have different signatures
- Migration is required for existing chains

## Benefits Summary

✅ No more unbounded storage  
✅ Better security with Blake2_128Concat hashing  
✅ Single storage read for topic access  
✅ Easy topic enumeration for UIs  
✅ Type-safe topic data  
✅ Extensible for future topic types  
✅ Unified API for related functionality  
✅ Proper error handling  
✅ Comprehensive test coverage  
✅ Automatic migration from v1  
