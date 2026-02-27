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
//! A set of common values used in robonomics runtime.

pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
pub use sp_runtime::{
    generic,
    traits::{IdentifyAccount, Verify},
};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = sp_runtime::MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Money matters.
pub mod currency {
    use super::*;
    use frame_support::PalletId;

    /// XRT is Robonomics native currency.
    ///
    /// Note: decimals is 9.
    pub const XRT: Balance = 1_000 * GLUSHKOV;

    /// Glushkov is unit name for milli XRT.
    pub const GLUSHKOV: Balance = 1_000 * COASE;

    /// Coase is unit name for micro XRT.
    pub const COASE: Balance = 1_000;

    /// Helper function for storage cost estimation.
    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 150 * GLUSHKOV / 100 + (bytes as Balance) * 60 * GLUSHKOV
    }

    /// Robonomics Network Treasury PalletId.
    pub const TREASURY_PALLET_ID: PalletId = PalletId(*b"py/trsry");

    #[cfg(test)]
    mod tests {
        use super::{AccountId, TREASURY_PALLET_ID};
        use hex_literal::hex;
        use sp_runtime::traits::AccountIdConversion;

        #[test]
        fn treasury_account_matched() {
            let treasury_account: AccountId = TREASURY_PALLET_ID.into_account_truncating();
            assert_eq!(
                treasury_account,
                AccountId::from(hex![
                    "6d6f646c70792f74727372790000000000000000000000000000000000000000"
                ]),
            );
        }
    }
}

/// Fee-related.
pub mod fee {
    use super::*;
    use frame_support::{
        pallet_prelude::Weight,
        weights::{
            constants::ExtrinsicBaseWeight, FeePolynomial, WeightToFeeCoefficient,
            WeightToFeeCoefficients, WeightToFeePolynomial,
        },
    };
    use smallvec::smallvec;
    use sp_runtime::Perbill;

    /// The block saturation level. Fees will be updated based on this value.
    pub const TARGET_BLOCK_FULLNESS: Perbill = Perbill::from_percent(25);

    /// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
    /// node's balance type.
    ///
    /// This should typically create a mapping between the following ranges:
    ///   - [0, MAXIMUM_BLOCK_WEIGHT]
    ///   - [Balance::min, Balance::max]
    ///
    /// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
    ///   - Setting it to `0` will essentially disable the weight fee.
    ///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
    pub struct WeightToFee;
    impl frame_support::weights::WeightToFee for WeightToFee {
        type Balance = Balance;

        fn weight_to_fee(weight: &Weight) -> Self::Balance {
            let time_poly: FeePolynomial<Balance> = RefTimeToFee::polynomial().into();
            let proof_poly: FeePolynomial<Balance> = ProofSizeToFee::polynomial().into();

            // Take the maximum instead of the sum to charge by the more scarce resource.
            time_poly
                .eval(weight.ref_time())
                .max(proof_poly.eval(weight.proof_size()))
        }
    }

    /// Maps the reference time component of `Weight` to a fee.
    pub struct RefTimeToFee;
    impl WeightToFeePolynomial for RefTimeToFee {
        type Balance = Balance;
        fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
            // In Westend, extrinsic base weight (smallest non-zero weight) is mapped to 1/10 COASE:
            // The standard system parachain configuration is 1/10 of that, as in 1/100 COASE.
            let p = super::currency::COASE;
            let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());

            smallvec![WeightToFeeCoefficient {
                degree: 1,
                negative: false,
                coeff_frac: Perbill::from_rational(p % q, q),
                coeff_integer: p / q,
            }]
        }
    }

    /// Maps the proof size component of `Weight` to a fee.
    pub struct ProofSizeToFee;
    impl WeightToFeePolynomial for ProofSizeToFee {
        type Balance = Balance;
        fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
            // Map 10kb proof to 1 COASE.
            let p = super::currency::COASE;
            let q = 10_000;

            smallvec![WeightToFeeCoefficient {
                degree: 1,
                negative: false,
                coeff_frac: Perbill::from_rational(p % q, q),
                coeff_integer: p / q,
            }]
        }
    }
}

/// Consensus-related.
pub mod consensus {
    use super::BlockNumber;
    use frame_support::weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight};
    use sp_runtime::Perbill;

    /// Maximum number of blocks simultaneously accepted by the Runtime, not yet included into the
    /// relay chain.
    pub const UNINCLUDED_SEGMENT_CAPACITY: u32 = 3;
    /// How many parachain blocks are processed by the relay chain per parent. Limits the number of
    /// blocks authored per slot.
    pub const BLOCK_PROCESSING_VELOCITY: u32 = 1;
    /// Relay chain slot duration, in milliseconds.
    pub const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

    /// We allow for 2 seconds of compute with a 6 second average block.
    pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
        WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
        cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
    );

    /// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
    /// used to limit the maximal weight of a single extrinsic.
    pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

    /// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
    /// Operational  extrinsics.
    pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

    /// This determines the average expected block time that we are targeting.
    /// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
    /// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
    /// up by `pallet_aura` to implement `fn slot_duration()`.
    ///
    /// Change this to adjust the block time.
    pub const MILLISECS_PER_BLOCK: u64 = 6000;
    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

    /// How many blocks in one minute.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);

    /// How many blocks in one hour.
    pub const HOURS: BlockNumber = MINUTES * 60;

    /// How many blocks in one day.
    pub const DAYS: BlockNumber = HOURS * 24;
}

/// XCM-related.
pub mod xcm_version {
    /// The default XCM version to set in genesis config.
    ///
    /// Note: depend of current `xcm` dependency version.
    pub const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
}
