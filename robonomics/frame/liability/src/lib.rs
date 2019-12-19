///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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
//! The Robonomics runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use codec::{Encode, Decode};
use system::ensure_none;
use support::{
    StorageValue, ensure, decl_module, decl_storage, decl_event,
    dispatch::Result,
};
use sp_runtime::transaction_validity::{
    TransactionValidity, ValidTransaction, InvalidTransaction, TransactionPriority,
};

/// Import module traits.
pub mod traits;
use traits::*;

pub mod signed;
pub mod technics;
pub mod economics;

/// Type synonym for technical trait parameter.
pub type TechnicalParam<T> = <<T as Trait>::Technics as Technical>::Parameter;

/// Type synonym for technical report trait parameter.
pub type TechnicalReport<T> = <<T as Trait>::Technics as Technical>::Report;

/// Type synonym for economical trait parameter.
pub type EconomicalParam<T> = <<T as Trait>::Economics as Economical>::Parameter;

/// Type synonym for liability proof parameter.
pub type ProofParam<T> = <<T as Trait>::Liability as Agreement<<T as Trait>::Technics, <T as Trait>::Economics>>::Proof;

/// Type synonym for liability account parameter.
pub type AccountId<T> = <<T as Trait>::Liability as Agreement<<T as Trait>::Technics, <T as Trait>::Economics>>::AccountId;


/// Liability module main trait.
pub trait Trait: system::Trait {
    /// Technical aspects of agreement.
    type Technics: Technical;

    /// Economical aspects of agreement.
    type Economics: Economical;

    /// How to make and process agreement between two parties. 
    type Liability: Agreement<Self::Technics, Self::Economics> + Processing;

    /// The overarching event type.
    type Event: From<Event> + Into<<Self as system::Trait>::Event>;
}

decl_event! {
    pub enum Event {
        /// Yay! New liability created.
        NewLiability(u64),

        /// Liability report published.
        NewReport(u64),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Liability {
        /// Latest liability index.
        LatestIndex get(fn latest_index): u64;
        /// Encoded liability parameters list.
        LiabilityOf get(fn liability_of): map u64 => Vec<u8>;
        /// Encoded liability report.
        ReportOf get(fn report_of): map u64 => Vec<u8>;
        /// Liability finalization flag.
        IsFinalized get(fn is_finalized): map u64 => bool;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Create agreement between two parties.
        fn create(
            origin,
            technics: TechnicalParam<T>,
            economics: EconomicalParam<T>,
            promisee: AccountId<T>,
            promisor: AccountId<T>,
            promisee_proof: ProofParam<T>,
            promisor_proof: ProofParam<T>,
        ) -> Result {
            ensure_none(origin)?;

            // Create liability
            let liability = T::Liability::new(technics, economics, promisee, promisor);

            // Check promisee proof
            if !liability.verify(ProofTarget::Promisee, &promisee_proof) {
                return Err("Bad promisee proof"); 
            }

            // Check promisor proof
            if !liability.verify(ProofTarget::Promisor, &promisor_proof) {
                return Err("Bad promisor proof");
            }

            // Run economical processing
            liability.on_start()?;

            // Store liability params as bytestring 
            let latest_index = <LatestIndex>::get() + 1;
            liability.using_encoded(|params|
                <LiabilityOf>::insert(latest_index, Vec::from(params))
            );
            <LatestIndex>::put(latest_index);

            Ok(())
        }

        /// Publish technical report of complite works.
        fn finalize(
            origin,
            index: u64,
            report: TechnicalReport<T>,
            proof: ProofParam<T>,
        ) -> Result {
            ensure_none(origin)?;

            // Is liability already finalized? 
            ensure!(!<IsFinalized>::get(index), "already finalized");

            // Decode liability from storage
            let liability = T::Liability::decode(&mut &<LiabilityOf>::get(index)[..])
                .map_err(|_| "unable decode liability params")?;

            // Check report proof
            if !liability.verify(ProofTarget::Report(report.clone()), &proof) {
                return Err("Bad report proof");
            }

            // Run economical processing
            // TODO: switch to oracle
            liability.on_finish(true)?;

            // Store report as bytestring
            report.using_encoded(|bytes| <ReportOf>::insert(index, Vec::from(bytes)));

            Ok(())
        }
    }
}

#[allow(deprecated)]
impl<T: Trait> support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
        match call {
            Call::create(technics, economics, promisee, promisor, promisee_proof, promisor_proof) => {
                let liability = T::Liability::new(
                    technics.clone(),
                    economics.clone(),
                    promisee.clone(),
                    promisor.clone(),
                );

                if !liability.verify(ProofTarget::Promisee, promisee_proof) {
                    return InvalidTransaction::BadProof.into();
                }

                if !liability.verify(ProofTarget::Promisor, promisor_proof) {
                    return InvalidTransaction::BadProof.into();
                }
            },

            Call::finalize(index, report, proof) =>
                match T::Liability::decode(&mut &<LiabilityOf>::get(*index)[..]) {
                    Ok(liability) =>
                        if !liability.verify(ProofTarget::Report(report.clone()), proof) {
                            return InvalidTransaction::BadProof.into();
                        },
                    _ => return InvalidTransaction::Call.into()
                },

		    _ => return InvalidTransaction::Call.into()
        };

		Ok(ValidTransaction {
			priority: TransactionPriority::max_value(),
			requires: vec![],
			provides: vec![],
			longevity: 64_u64,
			propagate: true,
		})
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::technics::PureIPFS;
    use super::economics::Communism;
    use super::signed::SignedLiability;
    use sp_runtime::{Perbill, traits::{Verify, IdentifyAccount}, testing::Header};
    use node_primitives::{AccountIndex, AccountId, Signature};
    use support::{impl_outer_origin, parameter_types, weights::Weight};
    use sp_core::{H256, sr25519, crypto::Pair};

    impl_outer_origin!{ pub enum Origin for Runtime {} }
    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct Runtime;
    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaximumBlockWeight: Weight = 1024;
        pub const MaximumBlockLength: u32 = 2 * 1024;
        pub const AvailableBlockRatio: Perbill = Perbill::one();
    }
    impl system::Trait for Runtime {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Call = ();
        type Hash = H256;
        type Hashing = ::sp_runtime::traits::BlakeTwo256;
        type AccountId = AccountId;
        type Lookup = Indices;
        type Header = Header;
        type Event = ();
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
    }
    impl indices::Trait for Runtime {
        type AccountIndex = AccountIndex;
        type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
        type IsDeadAccount = ();
        type Event = ();
    }
    impl Trait for Runtime {
        type Technics = PureIPFS;
        type Economics = Communism;
        type Liability = SignedLiability<Self::Technics, Self::Economics, <Signature as Verify>::Signer, Signature>;
        type Event = ();
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
        storage.into()
    }

    type System = system::Module<Runtime>;
    type Indices = indices::Module<Runtime>;
    type Liability = Module<Runtime>;

    #[test]
    fn test_setup_works() {
        new_test_ext().execute_with(|| {
            assert_eq!(Liability::latest_index(), 0);
        });
    }

    #[test]
    fn test_liability_proofs() {
        let pair: sr25519::Pair = Pair::from_string("//Alice", None).unwrap();
        let technics = vec![1,2,3];
        let economics = ();
        let order = (technics.clone(), economics.clone());
        let sender = <Signature as Verify>::Signer::from(pair.public()).into_account();
        let params_proof = order.using_encoded(|params| pair.sign(params)).into();
        let liability = <Runtime as Trait>::Liability::new(
            technics,
            economics,
            sender.clone(),
            sender,
        );
        assert_eq!(liability.verify(ProofTarget::Promisor, &params_proof), true);
        assert_eq!(liability.verify(ProofTarget::Promisee, &params_proof), true);

        let report = vec![3,2,1];
        let report_proof = report.using_encoded(|params| pair.sign(params)).into();
        assert_eq!(liability.verify(ProofTarget::Report(report), &report_proof), true);
    }
}
