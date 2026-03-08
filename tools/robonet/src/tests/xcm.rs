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
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//
///////////////////////////////////////////////////////////////////////////////
//! XCM (Cross-Consensus Messaging) integration tests.
//!
//! Tests verify XCM functionality:
//! - Upward XCM (parachain -> relay chain)
//! - Downward XCM (relay chain -> parachain)
//! - Native asset teleportation between Robonomics and AssetHub
//!
//! # Important Notes
//!
//! The Robonomics runtime supports teleportation of native assets (XRT) with AssetHub.
//! Foreign asset registration is not supported as the runtime does not include pallet_assets.

use anyhow::{anyhow, bail, ensure, Context, Result};
use robonomics_runtime_subxt_api::{api, AccountId32, MultiAddress};
use std::time::Duration;
use subxt::{
    client::OnlineClientT,
    config::HashFor,
    tx::{Payload, TxProgress, TxStatus},
    Config,
};
use subxt_signer::sr25519::dev;
use zombienet_sdk::{LocalFileSystem, Network};

use crate::network::{NetworkClient, ASSET_HUB_PARA_ID, PARA_ID, PARA_SIB_ACCOUNT};

// Local rococo relay API
#[subxt::subxt(
    runtime_metadata_path = "artifacts/relay.scale",
    derive_for_all_types = "Eq, PartialEq, Clone"
)]
pub mod relay {}

// Local AssetHub API
#[subxt::subxt(
    runtime_metadata_path = "artifacts/assethub.scale",
    derive_for_all_types = "Eq, PartialEq, Clone"
)]
pub mod assethub {}

/// Robonomics XCM helpers
mod robonomics_xcm {
    use super::{api, ASSET_HUB_PARA_ID};
    use subxt::tx::DefaultPayload;

    use api::runtime_types::robonomics_runtime::RuntimeCall;
    pub use api::runtime_types::staging_xcm::v5::{
        asset::{Asset, AssetId, Assets, Fungibility},
        junction::Junction,
        junctions::Junctions,
        location::Location,
        Instruction, Xcm,
    };
    pub use api::runtime_types::xcm::{
        double_encoded::DoubleEncoded,
        v3::{OriginKind, WeightLimit},
        VersionedLocation, VersionedXcm,
    };
    use api::sudo::calls::types::Sudo;

    /// Make XCM transaction message
    pub fn transact(origin_kind: OriginKind, fees: Asset, encoded: Vec<u8>) -> Xcm {
        Xcm(vec![
            Instruction::WithdrawAsset(Assets(vec![fees.clone()])),
            Instruction::BuyExecution {
                fees,
                weight_limit: WeightLimit::Unlimited,
            },
            Instruction::Transact {
                origin_kind,
                fallback_max_weight: None,
                call: DoubleEncoded { encoded },
            },
            Instruction::RefundSurplus,
        ])
    }

    /// Send XCM using Sudo call
    pub fn send(unboxed_dest: Location, unboxed_message: Xcm) -> DefaultPayload<Sudo> {
        let dest = Box::new(VersionedLocation::V5(unboxed_dest));
        let message = Box::new(VersionedXcm::V5(unboxed_message));
        let send_tx = RuntimeCall::XcmPallet(api::xcm_pallet::Call::send { dest, message });
        api::tx().sudo().sudo(send_tx)
    }

    /// Send version notify request using Sudo call
    pub fn force_version_notify() -> DefaultPayload<Sudo> {
        let location = Box::new(VersionedLocation::V5(ASSET_HUB_LOCATION));
        let notify_call =
            RuntimeCall::XcmPallet(api::xcm_pallet::Call::force_subscribe_version_notify {
                location,
            });
        api::tx().sudo().sudo(notify_call)
    }

    pub fn account_location(id: [u8; 32]) -> Location {
        Location {
            parents: 0,
            interior: Junctions::X1([Junction::AccountId32 { network: None, id }]),
        }
    }

    pub const RELAY_LOCATION: Location = Location {
        parents: 1,
        interior: Junctions::Here,
    };

    pub const ASSET_HUB_LOCATION: Location = Location {
        parents: 1,
        interior: Junctions::X1([Junction::Parachain(ASSET_HUB_PARA_ID)]),
    };

    pub const RELAY_ASSET: Asset = Asset {
        id: AssetId(Location {
            parents: 0,
            interior: Junctions::Here,
        }),
        fun: Fungibility::Fungible(1_000_000_000u128),
    };

    pub const AH_RELAY_ASSET: Asset = Asset {
        id: AssetId(Location {
            parents: 1,
            interior: Junctions::Here,
        }),
        fun: Fungibility::Fungible(1_000_000_000u128),
    };
}

/// Relay XCM helpers
mod relay_xcm {
    use super::relay as api;
    use subxt::tx::DefaultPayload;

    use api::runtime_types::rococo_runtime::RuntimeCall;
    pub use api::runtime_types::staging_xcm::v5::{
        junction::Junction, junctions::Junctions, location::Location, Instruction, Xcm,
    };
    pub use api::runtime_types::xcm::{
        double_encoded::DoubleEncoded,
        v3::{OriginKind, WeightLimit},
        VersionedLocation, VersionedXcm,
    };
    use api::sudo::calls::types::Sudo;

    /// Make XCM unpaid transaction message
    pub fn unpaid_transact(origin_kind: OriginKind, encoded: Vec<u8>) -> Xcm {
        Xcm(vec![
            Instruction::UnpaidExecution {
                weight_limit: WeightLimit::Unlimited,
                check_origin: None,
            },
            Instruction::Transact {
                origin_kind,
                fallback_max_weight: None,
                call: DoubleEncoded { encoded },
            },
        ])
    }

    /// Send XCM using Sudo call
    pub fn send(unboxed_dest: Location, unboxed_message: Xcm) -> DefaultPayload<Sudo> {
        let dest = Box::new(VersionedLocation::V5(unboxed_dest));
        let message = Box::new(VersionedXcm::V5(unboxed_message));
        let send_tx = RuntimeCall::XcmPallet(api::xcm_pallet::Call::send { dest, message });
        api::tx().sudo().sudo(send_tx)
    }
}

mod assethub_xcm {
    use super::assethub as api;

    pub use api::runtime_types::staging_xcm::v5::{
        junction::Junction, junctions::Junctions, location::Location,
    };

    pub use api::runtime_types::assets_common::local_and_foreign_assets::ForeignAssetReserveData;
    pub use api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
}

/// Wait for transaction appears in best block
pub async fn wait_for_best<T: Config, C: OnlineClientT<T>>(
    mut tx_process: TxProgress<T, C>,
) -> Result<HashFor<T>> {
    while let Some(status) = tx_process.next().await {
        match status? {
            TxStatus::Error { message } => bail!(message),
            TxStatus::Invalid { message } => bail!(message),
            TxStatus::Dropped { message } => bail!(message),
            TxStatus::InBestBlock(block) => return Ok(block.extrinsic_hash()),
            _ => continue,
        }
    }
    Err(anyhow!("Subscription dropped"))
}

/// Test: XCM upward message (parachain -> relay)
///
/// This test verifies that the parachain can send XCM messages to the relay chain
/// via the Upward Message Passing (UMP) queue. It sends a simple Remark instruction
/// wrapped in an XCM message.
pub async fn test_xcm_upward_message(network: Option<&Network<LocalFileSystem>>) -> Result<()> {
    log::info!("=== Test: XCM Upward Message (Parachain -> Relay) ===");

    // Connect to parachain and relay
    let para_client = NetworkClient::robonomics(network).await?;
    let relay_client = NetworkClient::relay(network).await?;

    log::info!("✓ Connected to parachain and relay chain");

    // Verify both chains are producing blocks
    let para_block = para_client.blocks().at_latest().await?;
    log::info!("  Parachain block: #{}", para_block.number());

    let relay_block = relay_client.blocks().at_latest().await?;
    log::info!("  Relay chain block: #{}", relay_block.number());

    // Get Alice's account for signing transactions
    let alice = dev::alice();
    log::info!("  Using account: Alice");

    // Simple System::remark() call for test
    let remark_call = relay::tx()
        .system()
        .remark(b"Hello from parachain".to_vec());
    let remark_call_encoded = remark_call
        .encode_call_data(&relay_client.metadata())
        .context("Unable to encode remark call")?;

    // Create a remark transaction XCM
    let message = robonomics_xcm::transact(
        robonomics_xcm::OriginKind::SovereignAccount,
        robonomics_xcm::RELAY_ASSET,
        remark_call_encoded,
    );
    let send_tx = robonomics_xcm::send(robonomics_xcm::RELAY_LOCATION, message);

    log::info!("  Sending XCM message via UMP...");

    // Submit transaction and wait for it to be included in a block
    let tx_process = para_client
        .tx()
        .sign_and_submit_then_watch_default(&send_tx, &alice)
        .await
        .context("Failed to submit sudo transaction")?;

    let tx_hash = wait_for_best(tx_process).await?;
    log::info!("XCM sent by sudo with tx-hash {}", tx_hash);

    // Wait for MessageQueue event on relay chain
    log::info!("  Waiting for MessageQueue::Processed event on relay chain...");

    let mut blocks_sub = relay_client
        .blocks()
        .subscribe_finalized()
        .await
        .context("Failed to subscribe to relay blocks")?;

    let timeout = Duration::from_secs(120);
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Some(block_result) = blocks_sub.next().await {
            let block = block_result.context("Failed to get block")?;
            let events = block.events().await.context("Failed to get events")?;

            for event in events.iter() {
                let event = event.context("Failed to parse event")?;

                // Check for MessageQueue::Processed event
                if event.pallet_name() == "MessageQueue" && event.variant_name() == "Processed" {
                    // Try to decode the event to check success field
                    if let Some(decoded) = event
                        .as_event::<relay::message_queue::events::Processed>()
                        .context(
                            "  event.as_event::<relay::message_queue::events::Processed> fails",
                        )?
                    {
                        log::info!("  Event found: {:?}", decoded);
                        if decoded.success {
                            log::info!(
                                "  ✓ MessageQueue event processed successfully on relay chain"
                            );
                            return Ok(());
                        } else {
                            log::error!("  ⚠ MessageQueue event is not success");
                            bail!("Transaction failed on destination chain")
                        }
                    }
                }
            }
        }
    }

    if start.elapsed() > timeout {
        log::warn!("  ⚠ MessageQueue success event not found within timeout");
        bail!("XCM not processed")
    }

    Ok(())
}

/// Test: XCM downward message (relay -> parachain)
///
/// This test verifies that the relay chain can send XCM messages to parachains
/// via the Downward Message Passing (DMP) queue. In a real scenario, this would
/// require sudo access on the relay chain.
pub async fn test_xcm_downward_message(network: Option<&Network<LocalFileSystem>>) -> Result<()> {
    log::info!("=== Test: XCM Downward Message (Relay -> Parachain) ===");

    // Connect to parachain and relay
    let para_client = NetworkClient::robonomics(network).await?;
    let relay_client = NetworkClient::relay(network).await?;

    log::info!("✓ Connected to relay and parachain");

    // Verify both chains are producing blocks
    let para_block = para_client.blocks().at_latest().await?;
    log::info!("  Parachain block: #{}", para_block.number());

    let relay_block = relay_client.blocks().at_latest().await?;
    log::info!("  Relay chain block: #{}", relay_block.number());

    // Get Alice's account for signing transactions
    let alice = dev::alice();
    log::info!("  Using account: Alice");

    let para_location = relay_xcm::Location {
        parents: 0,
        interior: relay_xcm::Junctions::X1([relay_xcm::Junction::Parachain(PARA_ID)]),
    };

    // Simple System::remark() call for test
    let remark_call = api::tx().system().remark(b"Hello from relay".to_vec());
    let remark_call_encoded = remark_call
        .encode_call_data(&para_client.metadata())
        .context("Unable to encode remark call")?;

    // Create a remark transaction XCM
    let message =
        relay_xcm::unpaid_transact(relay_xcm::OriginKind::SovereignAccount, remark_call_encoded);
    let send_tx = relay_xcm::send(para_location, message);

    log::info!("  Sending XCM message via DMP...");

    // Submit transaction and wait for it to be included in a block
    let tx_process = relay_client
        .tx()
        .sign_and_submit_then_watch_default(&send_tx, &alice)
        .await
        .context("Failed to submit sudo transaction")?;

    let tx_hash = wait_for_best(tx_process).await?;
    log::info!("XCM sent by sudo with tx-hash {}", tx_hash);

    // Wait for MessageQueue event on parachain
    log::info!("  Waiting for MessageQueue::Processed event on parachain...");

    let mut blocks_sub = para_client
        .blocks()
        .subscribe_finalized()
        .await
        .context("Failed to subscribe to parachain blocks")?;

    let timeout = Duration::from_secs(120);
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Some(block_result) = blocks_sub.next().await {
            let block = block_result.context("Failed to get block")?;
            let events = block.events().await.context("Failed to get events")?;

            for event in events.iter() {
                let event = event.context("Failed to parse event")?;

                // Check for MessageQueue::Processed event
                if event.pallet_name() == "MessageQueue" && event.variant_name() == "Processed" {
                    // Try to decode the event to check success field
                    if let Some(decoded) = event
                        .as_event::<api::message_queue::events::Processed>()
                        .context(
                            "  event.as_event::<api::message_queue::events::Processed> fails",
                        )?
                    {
                        log::info!(
                            "  MessageQueue::Processed event found: success={:?}",
                            decoded.success
                        );
                        if decoded.success {
                            log::info!(
                                "  ✓ MessageQueue event processed successfully on parachain"
                            );
                            return Ok(());
                        } else {
                            log::error!("  ⚠ MessageQueue event is not success");
                            bail!("Transaction failed on destination chain")
                        }
                    }
                }
            }
        }
    }

    if start.elapsed() > timeout {
        log::warn!("  ⚠ MessageQueue success event not found within timeout");
        bail!("XCM message not processed")
    }

    Ok(())
}

pub async fn test_hrmp_create_channel(network: Option<&Network<LocalFileSystem>>) -> Result<()> {
    log::info!("=== Test: XCM Create HRMP with system parachain ===");

    // Connect to parachain and relay
    let para_client = NetworkClient::robonomics(network).await?;
    let relay_client = NetworkClient::relay(network).await?;

    log::info!("✓ Connected to parachain and relay chain");

    // Verify both chains are producing blocks
    let para_block = para_client.blocks().at_latest().await?;
    log::info!("  Parachain block: #{}", para_block.number());

    let relay_block = relay_client.blocks().at_latest().await?;
    log::info!("  Relay chain block: #{}", relay_block.number());

    // Get Alice's account for signing transactions
    let alice = dev::alice();
    log::info!("  Using account: Alice");

    use relay::runtime_types::polkadot_parachain_primitives::primitives::Id;

    let hrmp_call = relay::tx()
        .hrmp()
        .establish_channel_with_system(Id(ASSET_HUB_PARA_ID));
    let hrmp_call_encoded = hrmp_call
        .encode_call_data(&relay_client.metadata())
        .context("Unable to encode hrmp call")?;

    // Create a remark transaction XCM
    let message = robonomics_xcm::transact(
        robonomics_xcm::OriginKind::Native,
        robonomics_xcm::RELAY_ASSET,
        hrmp_call_encoded,
    );
    let send_tx = robonomics_xcm::send(robonomics_xcm::RELAY_LOCATION, message);

    log::info!("  Sending XCM message via UMP...");

    // Submit transaction and wait for it to be included in a block
    let tx_process = para_client
        .tx()
        .sign_and_submit_then_watch_default(&send_tx, &alice)
        .await
        .context("Failed to submit sudo transaction")?;

    let tx_hash = wait_for_best(tx_process).await?;
    log::info!("  XCM sent by sudo with tx-hash {}", tx_hash);

    // Wait for Hrmp event on relay chain
    log::info!("  Waiting for Hrmp.HrmpSystemChannelOpened event on relay chain...");

    let mut blocks_sub = relay_client
        .blocks()
        .subscribe_finalized()
        .await
        .context("Failed to subscribe to relay blocks")?;

    let mut event_found = false;
    let timeout = Duration::from_secs(120);
    let start = std::time::Instant::now();
    while start.elapsed() < timeout && !event_found {
        if let Some(block_result) = blocks_sub.next().await {
            let block = block_result.context("Failed to get block")?;
            let events = block.events().await.context("Failed to get events")?;

            for event in events.iter() {
                let event = event.context("Failed to parse event")?;
                log::trace!(
                    "  New Event(name: {}, variant: {})",
                    event.pallet_name(),
                    event.variant_name(),
                );

                // Check for Hrmp::HrmpSystemChannelOpened event
                if event.pallet_name() == "Hrmp"
                    && event.variant_name() == "HrmpSystemChannelOpened"
                {
                    // Try to decode the event to check success field
                    if let Some(decoded) = event.as_event::<relay::hrmp::events::HrmpSystemChannelOpened>()
                        .context("  event.as_event::<relay::hrmp::events::HrmpSystemChannelOpened> fails")? {
                        log::info!("  Event found: {:?}", decoded);

                        if decoded.sender.0 == PARA_ID && decoded.recipient.0 == ASSET_HUB_PARA_ID {
                            log::info!("  Event confirmed");
                            event_found = true;
                            break;
                        }
                    } else {
                        bail!("Unable to decode HrmpSystemChannelOpened event");
                    }
                }
            }
        }
    }

    if start.elapsed() > timeout {
        log::warn!("  ⚠ HrmpSystemChannelOpened event not found within timeout");
        bail!("XCM failed")
    }

    let version_query = api::storage().xcm_pallet().supported_version(
        5,
        robonomics_xcm::VersionedLocation::V5(robonomics_xcm::RELAY_LOCATION),
    );
    if let Some(5) = para_client
        .storage()
        .at_latest()
        .await?
        .fetch(&version_query)
        .await?
    {
        log::info!("  Parachain already known relay XCM version, test finish");
        return Ok(());
    }

    // Wait for relay version event on parachain chain
    log::info!("  Waiting for XcmPallet::SupportedVersionChanged event on parachain...");

    let mut blocks_sub = para_client
        .blocks()
        .subscribe_finalized()
        .await
        .context("Failed to subscribe to parachain blocks")?;

    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Some(block_result) = blocks_sub.next().await {
            let block = block_result.context("Failed to get block")?;
            let events = block.events().await.context("Failed to get events")?;

            for event in events.iter() {
                let event = event.context("Failed to parse event")?;
                log::trace!(
                    "  New Event(name: {}, variant: {})",
                    event.pallet_name(),
                    event.variant_name(),
                );

                // Check for XcmPallet::SupportedversionChanged event
                if event.pallet_name() == "XcmPallet"
                    && event.variant_name() == "SupportedVersionChanged"
                {
                    // Try to decode the event to check success field
                    if let Some(decoded) = event.as_event::<api::xcm_pallet::events::SupportedVersionChanged>()
                        .context("  event.as_event::<robonomics::xcm_pallet::events::SupportedVersionChanged> fails")? {
                        log::info!("  Event found: {:?}", decoded);

                        if decoded.version == 5u32 && decoded.location == robonomics_xcm::RELAY_LOCATION {
                            log::info!("  Event confirmed, test success");
                            return Ok(())
                        }
                    } else {
                        bail!("Unable to decode SupportedVersionChanged event");
                    }
                }
            }
        }
    }

    if start.elapsed() > timeout {
        log::warn!("  ⚠ SupportedVersionChanged event not found within timeout");
        bail!("XCM failed")
    }

    Ok(())
}

pub async fn test_subscribe_version_notify(
    network: Option<&Network<LocalFileSystem>>,
) -> Result<()> {
    log::info!("=== Test: Start XCM version notify with AssetHub ===");

    // Connect to parachain
    let para_client = NetworkClient::robonomics(network).await?;

    log::info!("✓ Connected to parachain");

    // Verify both chains are producing blocks
    let para_block = para_client.blocks().at_latest().await?;
    log::info!("  Parachain block: #{}", para_block.number());

    // Get Alice's account for signing transactions
    let alice = dev::alice();
    log::info!("  Using account: Alice");

    let subscribe_tx = robonomics_xcm::force_version_notify();

    log::info!("  Sending XCM message via HRMP...");

    // Submit transaction and wait for it to be included in a block
    let tx_process = para_client
        .tx()
        .sign_and_submit_then_watch_default(&subscribe_tx, &alice)
        .await
        .context("Failed to submit sudo transaction")?;

    let tx_hash = wait_for_best(tx_process).await?;
    log::info!("  XCM sent by sudo with tx-hash {}", tx_hash);

    // Wait for Hrmp event on relay chain
    log::info!("  Waiting for XcmPallet::SupportedVersionChanged event on parachain...");

    let mut blocks_sub = para_client
        .blocks()
        .subscribe_finalized()
        .await
        .context("Failed to subscribe to parachain blocks")?;

    let timeout = Duration::from_secs(120);
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Some(block_result) = blocks_sub.next().await {
            let block = block_result.context("Failed to get block")?;
            let events = block.events().await.context("Failed to get events")?;

            for event in events.iter() {
                let event = event.context("Failed to parse event")?;
                log::trace!(
                    "  New Event(name: {}, variant: {})",
                    event.pallet_name(),
                    event.variant_name(),
                );

                // Check for XcmPallet::SupportedversionChanged event
                if event.pallet_name() == "XcmPallet"
                    && event.variant_name() == "SupportedVersionChanged"
                {
                    // Try to decode the event to check success field
                    if let Some(decoded) = event.as_event::<api::xcm_pallet::events::SupportedVersionChanged>()
                        .context("  event.as_event::<robonomics::xcm_pallet::events::SupportedVersionChanged> fails")? {
                        log::info!("  Event found: {:?}", decoded);

                        if decoded.version == 5u32 && decoded.location == robonomics_xcm::ASSET_HUB_LOCATION {
                            log::info!("  Event confirmed, test success");
                            return Ok(())
                        }
                    } else {
                        bail!("Unable to decode SupportedVersionChanged event");
                    }
                }
            }
        }
    }

    if start.elapsed() > timeout {
        log::warn!("  ⚠ XcmPallet event not found within timeout");
        bail!("XCM failed")
    }

    Ok(())
}

async fn test_create_foreign_asset(network: Option<&Network<LocalFileSystem>>) -> Result<()> {
    log::info!("=== Test: Create Foreign Asset ===");

    // Connect to parachain and asset-hub
    let para_client = NetworkClient::robonomics(network).await?;
    let assethub_client = NetworkClient::assethub(network).await?;

    // Verify both chains are producing blocks
    let para_block = para_client.blocks().at_latest().await?;
    log::info!("  Parachain block: #{}", para_block.number());

    let assethub_block = assethub_client.blocks().at_latest().await?;
    log::info!("  AssetHub chain block: #{}", assethub_block.number());

    // Get Alice's account for signing transactions
    let alice = dev::alice();
    log::info!("  Using account: Alice");

    let admin: AccountId32 = PARA_SIB_ACCOUNT
        .parse()
        .context("Unable to parse para_address")?;

    let id = assethub_xcm::Location {
        parents: 1,
        interior: assethub_xcm::Junctions::X1([assethub_xcm::Junction::Parachain(PARA_ID)]),
    };
    let create_asset = assethub::tx()
        .foreign_assets()
        .create(id.clone(), MultiAddress::Id(admin.clone()), 1_000)
        .encode_call_data(&assethub_client.metadata())
        .context("Unable to encode remark call")?;

    // Create transaction XCM
    let message = robonomics_xcm::transact(
        robonomics_xcm::OriginKind::Xcm,
        robonomics_xcm::AH_RELAY_ASSET,
        create_asset,
    );
    let send_tx = robonomics_xcm::send(robonomics_xcm::ASSET_HUB_LOCATION, message);

    log::info!("  Sending XCM message via XCMP...");

    // Submit transaction and wait for it to be included in a block
    let tx_process = para_client
        .tx()
        .sign_and_submit_then_watch_default(&send_tx, &alice)
        .await
        .context("Failed to submit sudo transaction")?;

    let tx_hash = wait_for_best(tx_process).await?;
    log::info!("  XCM sent by sudo with tx-hash {}", tx_hash);

    // Wait for MessageQueue event on AssetHub chain
    log::info!("  Waiting for ForeignAssets::Created event on AssetHub chain...");

    let mut blocks_sub = assethub_client
        .blocks()
        .subscribe_finalized()
        .await
        .context("Failed to subscribe to relay blocks")?;

    let timeout = Duration::from_secs(120);
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Some(block_result) = blocks_sub.next().await {
            let block = block_result.context("Failed to get block")?;
            let events = block.events().await.context("Failed to get events")?;

            for event in events.iter() {
                let event = event.context("Failed to parse event")?;
                log::trace!(
                    "  New Event(name: {}, variant: {})",
                    event.pallet_name(),
                    event.variant_name(),
                );

                // Check for foreign_assets::Created event
                if event.pallet_name() == "ForeignAssets" && event.variant_name() == "Created" {
                    // Try to decode the event to check success field
                    if let Some(decoded) = event
                        .as_event::<assethub::foreign_assets::events::Created>()
                        .context(
                            "  event.as_event::<assethub::foreign_assets::events::Created> fails",
                        )?
                    {
                        log::info!("  Event found: {:?}", decoded);

                        ensure!(
                            decoded.asset_id == id,
                            "AssetId should be {:?} but was {:?}",
                            id,
                            decoded.asset_id,
                        );
                        ensure!(
                            decoded.creator == admin,
                            "Creator should be {} but was {:?}",
                            admin,
                            decoded.creator,
                        );
                        ensure!(
                            decoded.owner == admin,
                            "Creator should be {} but was {:?}",
                            admin,
                            decoded.owner,
                        );

                        return Ok(());
                    } else {
                        bail!("Unable to decode Created event");
                    }
                }
            }
        }
    }

    if start.elapsed() > timeout {
        log::warn!("  ⚠ Created event not found within timeout");
        bail!("XCM failed")
    }

    Ok(())
}

async fn test_set_asset_trusted_reserve(network: Option<&Network<LocalFileSystem>>) -> Result<()> {
    log::info!("=== Test: Set Foreign Asset Trusted Reserves ===");

    // Connect to parachain and asset-hub
    let para_client = NetworkClient::robonomics(network).await?;
    let assethub_client = NetworkClient::assethub(network).await?;

    // Verify both chains are producing blocks
    let para_block = para_client.blocks().at_latest().await?;
    log::info!("  Parachain block: #{}", para_block.number());

    let assethub_block = assethub_client.blocks().at_latest().await?;
    log::info!("  AssetHub chain block: #{}", assethub_block.number());

    // Get Alice's account for signing transactions
    let alice = dev::alice();
    log::info!("  Using account: Alice");

    let id = assethub_xcm::Location {
        parents: 1,
        interior: assethub_xcm::Junctions::X1([assethub_xcm::Junction::Parachain(PARA_ID)]),
    };
    let reserves = vec![assethub_xcm::ForeignAssetReserveData {
        reserve: id.clone(),
        teleportable: true,
    }];
    let set_reserves = assethub::tx()
        .foreign_assets()
        .set_reserves(id.clone(), assethub_xcm::BoundedVec(reserves.clone()))
        .encode_call_data(&assethub_client.metadata())
        .context("Unable to encode foreign_assets.set_reserves call")?;

    // Create transaction XCM
    let message = robonomics_xcm::transact(
        robonomics_xcm::OriginKind::Xcm,
        robonomics_xcm::AH_RELAY_ASSET,
        set_reserves,
    );
    let send_tx = robonomics_xcm::send(robonomics_xcm::ASSET_HUB_LOCATION, message);

    log::info!("  Sending XCM message via XCMP...");

    // Submit transaction and wait for it to be included in a block
    let tx_process = para_client
        .tx()
        .sign_and_submit_then_watch_default(&send_tx, &alice)
        .await
        .context("Failed to submit sudo transaction")?;

    let tx_hash = wait_for_best(tx_process).await?;
    log::info!("  XCM sent by sudo with tx-hash {}", tx_hash);

    // Wait for MessageQueue event on AssetHub chain
    log::info!("  Waiting for ForeignAssets::ReservesUpdated event on AssetHub chain...");

    let mut blocks_sub = assethub_client
        .blocks()
        .subscribe_finalized()
        .await
        .context("Failed to subscribe to AssetHub blocks")?;

    let timeout = Duration::from_secs(120);
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Some(block_result) = blocks_sub.next().await {
            let block = block_result.context("Failed to get block")?;
            let events = block.events().await.context("Failed to get events")?;

            for event in events.iter() {
                let event = event.context("Failed to parse event")?;
                log::trace!(
                    "  New Event(name: {}, variant: {})",
                    event.pallet_name(),
                    event.variant_name(),
                );

                // Check for foreign_assets::ReservesUpdated event
                if event.pallet_name() == "ForeignAssets"
                    && event.variant_name() == "ReservesUpdated"
                {
                    // Try to decode the event to check success field
                    if let Some(decoded) = event
                        .as_event::<assethub::foreign_assets::events::ReservesUpdated>()
                        .context(
                            "  event.as_event::<assethub::foreign_assets::events::ReservesUpdated> fails",
                        )?
                    {
                        log::info!("  ForeignAssets::ReservesUpdated event found: {:?}", decoded);

                        ensure!(
                            decoded.asset_id == id,
                            "AssetId should be {:?} but was {:?}",
                            id,
                            decoded.asset_id,
                        );
                        ensure!(
                            decoded.reserves == reserves,
                            "Creator should be {:?} but was {:?}",
                            reserves,
                            decoded.reserves,
                        );

                        return Ok(())
                    } else {
                        bail!("Unable to decode ReservesUpdated event");
                    }
                }
            }
        }
    }

    if start.elapsed() > timeout {
        log::warn!("  ⚠ ReservesUpdated event not found within timeout");
        bail!("XCM failed")
    }

    Ok(())
}

async fn test_teleport_to_assethub(network: Option<&Network<LocalFileSystem>>) -> Result<()> {
    log::info!("=== Test: Teleport Assets (Robonomics -> AssetHub) ===");

    // Connect to both chains
    let para_client = NetworkClient::robonomics(network).await?;
    let assethub_client = NetworkClient::assethub(network).await?;

    log::info!("✓ Connected to Robonomics and AssetHub");

    // Use Alice as the sender
    let alice = dev::alice();
    let alice_account_id: subxt::utils::AccountId32 = alice.public_key().into();

    log::info!(
        "  Sender account: Alice ({})",
        hex::encode(&alice_account_id)
    );

    // Get initial balance on Robonomics parachain
    let para_balance_query = api::storage().system().account(alice_account_id.clone());
    let para_block = para_client.blocks().at_latest().await?;

    let initial_para_balance = match para_client
        .storage()
        .at(para_block.hash())
        .fetch(&para_balance_query)
        .await?
    {
        Some(account_info) => {
            let free_balance = account_info.data.free;
            log::info!("  Initial Robonomics balance: {} COASE", free_balance);
            free_balance
        }
        None => {
            log::warn!("  No balance found for Alice on Robonomics");
            0u128
        }
    };

    // Check initial AssetHub balance (if exists)
    // Note: AssetHub uses a similar account structure
    let assethub_block = assethub_client.blocks().at_latest().await?;
    log::info!("  AssetHub block: #{}", assethub_block.number());
    log::info!("  Robonomics block: #{}", para_block.number());

    // Amount to teleport: 1 XRT = 1_000_000_000 COASE (9 decimals)
    let amount: u128 = 1_000_000_000;
    log::info!("  Amount to teleport: {} Wn (1 XRT)", amount);

    // Construct beneficiary: Alice's account on destination
    let beneficiary = robonomics_xcm::account_location(alice_account_id.0.clone());

    // Construct assets: native asset with amount

    log::info!("  Constructing teleport transaction...");

    // Create teleport transaction using static API
    let teleport_tx = api::tx().teleport_xrt().send(beneficiary, amount);

    log::info!("  Submitting teleport transaction...");

    // Submit and watch transaction
    let _events = para_client
        .tx()
        .sign_and_submit_then_watch_default(&teleport_tx, &alice)
        .await
        .context("Failed to submit teleport transaction")?
        .wait_for_finalized_success()
        .await
        .context("Teleport transaction fails")?;

    let final_para_balance = match para_client
        .storage()
        .at_latest()
        .await?
        .fetch(&para_balance_query)
        .await?
    {
        Some(account_info) => {
            let free_balance = account_info.data.free;
            log::info!("  Final Robonomics balance: {} Wn", free_balance);
            free_balance
        }
        None => {
            log::warn!("  No balance found for Alice on Robonomics");
            0u128
        }
    };

    ensure!(
        final_para_balance + amount < initial_para_balance,
        "Wrong final Alice balance on parachain"
    );

    Ok(())
}

/// Test: XCM token teleport between parachains
///
/// This is the main test entry point that orchestrates all XCM teleportation tests.
/// It runs different test scenarios based on the network topology.
///
/// # Note on Foreign Assets
///
/// The Robonomics runtime does not include `pallet_assets`, so foreign asset
/// registration is not supported. These tests focus on native token (XRT)
/// teleportation, which is fully supported via the trusted teleporter
/// configuration with AssetHub.
pub async fn test_xcm_token_teleport(network: Option<&Network<LocalFileSystem>>) -> Result<()> {
    log::info!("==================================================");
    log::info!("  XCM Token Teleportation Tests");
    log::info!("==================================================");
    log::info!("");

    log::info!("[ 1/5 ] Register foreign asset on AssetHub");
    test_hrmp_create_channel(network).await?;

    log::info!("[ 2/5 ] Subscribe XCM version for AssetHub");
    test_subscribe_version_notify(network).await?;

    log::info!("[ 3/5 ] Register foreign asset on AssetHub");
    test_create_foreign_asset(network).await?;

    log::info!("[ 4/5 ] Set asset trusted reserve information");
    test_set_asset_trusted_reserve(network).await?;

    log::info!("[ 5/5 ] Test Teleport: Robonomics -> AssetHub");
    test_teleport_to_assethub(network).await?;

    log::info!("");
    log::info!("==================================================");
    log::info!("  All XCM Token Teleport Tests Completed Successfully");
    log::info!("==================================================");

    Ok(())
}
