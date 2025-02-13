//////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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
//! Generic Robonomics runtime inerface.

use frame_support::weights::Weight;
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
use robonomics_primitives::{AccountId, Balance, Block, Nonce};
use sp_api::impl_runtime_apis;
use sp_core::OpaqueMetadata;
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
    traits::Block as BlockT,
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, ExtrinsicInclusionMode,
};
use sp_version::RuntimeVersion;

pub struct Runtime;

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            unimplemented!()
        }

        fn execute_block(_block: Block) {
            unimplemented!()
        }

        fn initialize_block(_header: &<Block as BlockT>::Header) -> ExtrinsicInclusionMode {
            unimplemented!()
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            unimplemented!()
        }

        fn metadata_at_version(_version: u32) -> Option<OpaqueMetadata> {
            unimplemented!()
        }

        fn metadata_versions() -> sp_std::vec::Vec<u32> {
            unimplemented!()
        }
    }

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info(_header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
            unimplemented!()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(_extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            unimplemented!()
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            unimplemented!()
        }

        fn inherent_extrinsics(_data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            unimplemented!()
        }

        fn check_inherents(_block: Block, _data: InherentData) -> CheckInherentsResult {
            unimplemented!()
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(_account: AccountId) -> Nonce {
            unimplemented!()
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(_seed: Option<Vec<u8>>) -> Vec<u8> {
            unimplemented!()
        }

        fn decode_session_keys(_encoded: Vec<u8>) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
            unimplemented!()
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
        Block,
        Balance,
    > for Runtime {
        fn query_info(_uxt: <Block as BlockT>::Extrinsic, _len: u32) -> RuntimeDispatchInfo<Balance> {
            unimplemented!()
        }
        fn query_fee_details(_uxt: <Block as BlockT>::Extrinsic, _len: u32) -> FeeDetails<Balance> {
            unimplemented!()
        }
        fn query_weight_to_fee(_weight: Weight) -> Balance {
            unimplemented!()
        }
        fn query_length_to_fee(_length: u32) -> Balance {
            unimplemented!()
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(_source: TransactionSource, _tx: <Block as BlockT>::Extrinsic, _block_hash: <Block as BlockT>::Hash) -> TransactionValidity {
            unimplemented!()
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(_header: &<Block as BlockT>::Header) {
            unimplemented!()
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(_extra: bool) -> (Vec<frame_benchmarking::BenchmarkList>, Vec<frame_support::traits::StorageInfo>) {
            unimplemented!()
        }

        fn dispatch_benchmark(_config: frame_benchmarking::BenchmarkConfig) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            unimplemented!()
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(_checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            unimplemented!()
        }

        fn execute_block(_block: Block, _state_root_check: bool, _signature_check: bool, _select: frame_try_runtime::TryStateSelect) -> Weight {
            unimplemented!()
        }
    }
}
