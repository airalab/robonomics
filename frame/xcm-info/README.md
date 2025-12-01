# XCM Info Pallet

The XCM Info pallet provides essential on-chain storage and management capabilities for Cross-Consensus Messaging (XCM) configuration and asset information within the Robonomics parachain ecosystem.

## Overview

This pallet enables privileged accounts (root origin) to configure and maintain critical XCM-related data:

- **Relay Network Configuration**: Store the relay chain network identifier for proper XCM routing
- **Asset-Location Mapping**: Create bidirectional mappings between local asset IDs and XCM locations

These capabilities are fundamental for enabling cross-chain asset transfers and proper XCM message routing in a parachain environment.

## Key Features

### 1. Relay Network Management

The pallet stores the network identifier (e.g., Polkadot, Kusama) of the relay chain that the parachain is connected to. This information is crucial for:
- Proper XCM message construction and routing
- Validating incoming XCM messages
- Ensuring compatibility with the relay chain's XCM configuration

### 2. Bidirectional Asset-Location Mapping

The pallet maintains two-way mappings between:
- **Local Asset IDs**: Integer identifiers used within the parachain's runtime
- **XCM Locations**: Multi-location structures that uniquely identify assets across chains

This enables seamless conversion between local and cross-chain asset representations.

### 3. MaybeEquivalence Implementation

The pallet implements the `MaybeEquivalence<Location, AssetId>` trait, which provides:
- `convert()`: Convert XCM location to local asset ID
- `convert_back()`: Convert local asset ID to XCM location

This trait implementation can be used by XCM executors and other pallets that need to resolve asset identities.

## Usage Examples

### Setting the Relay Network

```rust
use xcm::latest::prelude::*;

// Configure Kusama as the relay network
XcmInfo::set_relay_network(RuntimeOrigin::root(), NetworkId::Kusama)?;
```

### Creating Asset-Location Links

```rust
use xcm::latest::prelude::*;

// Link local asset ID 1 to a location on parachain 2000
let location = Location::new(1, [Parachain(2000)]);
XcmInfo::set_asset_link(RuntimeOrigin::root(), 1u32, location)?;

// Link local asset ID 10 to a multi-hop location
let complex_location = Location::new(
    1, 
    [Parachain(2000), GeneralIndex(42)]
);
XcmInfo::set_asset_link(RuntimeOrigin::root(), 10u32, complex_location)?;
```

### Querying Mappings

```rust
// Get the XCM location for a local asset ID
if let Some(location) = XcmInfo::location_of(asset_id) {
    // Use the location in XCM messages
    println!("Asset {} is located at {:?}", asset_id, location);
}

// Get the local asset ID for an XCM location
if let Some(asset_id) = XcmInfo::assetid_of(&location) {
    // Use the asset ID in local operations
    println!("Location {:?} corresponds to asset {}", location, asset_id);
}

// Query relay network
if let Some(network) = XcmInfo::relay_network() {
    println!("Connected to relay network: {:?}", network);
}
```

### Using MaybeEquivalence Trait

```rust
use sp_runtime::traits::MaybeEquivalence;

// Convert location to asset ID
let asset_id = <Pallet<T> as MaybeEquivalence<Location, AssetId>>::convert(&location);

// Convert asset ID back to location
let location = <Pallet<T> as MaybeEquivalence<Location, AssetId>>::convert_back(&asset_id);
```

## Storage Items

### RelayNetwork

```rust
pub type RelayNetwork<T> = StorageValue<_, NetworkId>
```

Stores the network identifier of the relay chain (e.g., `NetworkId::Polkadot`, `NetworkId::Kusama`).

**Getter**: `relay_network() -> Option<NetworkId>`

### LocationOf

```rust
pub type LocationOf<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, Location>
```

Maps local asset IDs to their corresponding XCM locations. Part of the bidirectional mapping system.

**Getter**: `location_of(asset_id: T::AssetId) -> Option<Location>`

### AssetIdOf

```rust
pub type AssetIdOf<T: Config> = StorageMap<_, Blake2_128Concat, Location, T::AssetId>
```

Maps XCM locations to their corresponding local asset IDs. Part of the bidirectional mapping system.

**Getter**: `assetid_of(location: &Location) -> Option<T::AssetId>`

## Extrinsics

### set_relay_network

```rust
pub fn set_relay_network(origin: OriginFor<T>, network_id: NetworkId) -> DispatchResult
```

Sets or updates the relay chain network identifier.

**Parameters**:
- `origin`: Must be root origin (governance or sudo)
- `network_id`: The network identifier (e.g., `NetworkId::Polkadot`, `NetworkId::Kusama`)

**Events**: 
- `RelayNetworkChanged(NetworkId)`: Emitted when successfully updated

**Errors**:
- `BadOrigin`: If caller is not root

### set_asset_link

```rust
pub fn set_asset_link(
    origin: OriginFor<T>, 
    asset_id: T::AssetId, 
    location: Location
) -> DispatchResult
```

Creates a bidirectional link between a local asset ID and an XCM location.

**Parameters**:
- `origin`: Must be root origin (governance or sudo)
- `asset_id`: The local asset identifier to link
- `location`: The XCM location to associate with the asset

**Events**:
- `AssetLinkAdded(AssetId, Location)`: Emitted when successfully created

**Errors**:
- `BadOrigin`: If caller is not root

## Configuration

To integrate the XCM Info pallet into your runtime:

```rust
impl pallet_xcm_info::Config for Runtime {
    type AssetId = u32;  // Or your preferred asset ID type
    type RuntimeEvent = RuntimeEvent;
}
```

### Configuration Parameters

- **AssetId**: The type used to identify assets locally. Should implement `Parameter`, `Copy`, `Default`, and `MaxEncodedLen`. Common choices are `u32` or `u64`.
- **RuntimeEvent**: The overarching runtime event type.

## Testing

### Running Unit Tests

```bash
# Run all tests for the pallet
cargo test -p pallet-xcm-info

# Run tests with output
cargo test -p pallet-xcm-info -- --nocapture

# Run a specific test
cargo test -p pallet-xcm-info test_name
```

### Running Benchmarks

```bash
# Compile with benchmarking features
cargo test -p pallet-xcm-info --features runtime-benchmarks

# Run benchmarks (requires full runtime context)
cargo test -p pallet-xcm-info --features runtime-benchmarks -- --nocapture
```

## Integration Guide

### Step 1: Add Dependency

Add the pallet to your runtime's `Cargo.toml`:

```toml
[dependencies]
pallet-xcm-info = { path = "../../frame/xcm-info", default-features = false }

[features]
std = [
    # ... other pallets
    "pallet-xcm-info/std",
]

runtime-benchmarks = [
    # ... other pallets  
    "pallet-xcm-info/runtime-benchmarks",
]
```

### Step 2: Implement Config

Add the configuration to your runtime:

```rust
impl pallet_xcm_info::Config for Runtime {
    type AssetId = u32;
    type RuntimeEvent = RuntimeEvent;
}
```

### Step 3: Add to construct_runtime!

Include the pallet in your runtime construction:

```rust
construct_runtime!(
    pub enum Runtime {
        // ... other pallets
        XcmInfo: pallet_xcm_info,
    }
);
```

### Step 4: Use in XCM Configuration

Integrate with your XCM executor configuration:

```rust
pub type AssetTransactor = (
    // ... other transactors
    FungiblesAdapter<
        Assets,
        ConvertedConcreteId<
            AssetId,
            Balance,
            XcmInfo,  // Use as the converter
            JustTry,
        >,
        // ... other parameters
    >,
);
```

### Step 5: Initialize via Governance

Set up initial configuration through governance proposals:

```rust
// Proposal 1: Set relay network
XcmInfo::set_relay_network(RuntimeOrigin::root(), NetworkId::Kusama)?;

// Proposal 2: Link your native token
let native_location = Location::parent();
XcmInfo::set_asset_link(RuntimeOrigin::root(), 0u32, native_location)?;

// Proposal 3: Link relay chain native token
let relay_native = Location::new(1, []);
XcmInfo::set_asset_link(RuntimeOrigin::root(), 1u32, relay_native)?;
```

## Security Considerations

1. **Root Origin Required**: All state-changing extrinsics require root origin. Ensure proper governance controls are in place.

2. **Mapping Overwrites**: Calling `set_asset_link` with an existing asset ID will overwrite the previous location. The reverse mapping from the old location will persist in storage.

3. **No Deletion**: The pallet does not provide functionality to remove mappings. Consider this when managing asset lifecycles.

4. **XCM Version Compatibility**: Ensure Location structures are compatible with the XCM version used across your chain ecosystem.

## License

Licensed under the Apache License, Version 2.0. See the [LICENSE](../../LICENSE) file for details.

## Support

For issues, questions, or contributions related to the XCM Info pallet:

- GitHub Issues: [https://github.com/airalab/robonomics/issues](https://github.com/airalab/robonomics/issues)
- Documentation: [https://wiki.robonomics.network](https://wiki.robonomics.network)
