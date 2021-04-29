///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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
//! Lighthouse is a block author in robonomics parachain.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use sp_inherents::{InherentData, ProvideInherentData};
use sp_inherents::{InherentIdentifier, IsFatalError};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Lighthouse: frame_support::dispatch::Parameter;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Lighthouse already set in block.
        LighthouseAlreadySet,
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    #[pallet::getter(fn lighthouse)]
    pub(super) type Lighthouse<T> = StorageValue<_, <T as Config>::Lighthouse>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            <Lighthouse<T>>::kill();
            0
        }

        fn on_finalize(_n: T::BlockNumber) {
            assert!(
                <Lighthouse<T>>::get().is_some(),
                "No valid lighthouse set in block"
            );
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Inherent to set the lighthouse of a block.
        #[pallet::weight((0, DispatchClass::Mandatory))]
        fn set_lighthouse(
            origin: OriginFor<T>,
            lighthouse: T::Lighthouse,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;
            ensure!(
                <Lighthouse<T>>::get().is_none(),
                Error::<T>::LighthouseAlreadySet
            );

            // Update storage
            <Lighthouse<T>>::put(&lighthouse);

            Ok(().into())
        }
    }

    #[pallet::inherent]
    impl<T: Config> ProvideInherent for Pallet<T> {
        type Call = Call<T>;
        type Error = InherentError;

        const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

        fn is_inherent_required(_: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
            Ok(Some(InherentError::Other(
                sp_runtime::RuntimeString::Borrowed("LighthouseInherentRequired"),
            )))
        }

        fn create_inherent(data: &InherentData) -> Option<Self::Call> {
            let lighthouse_raw = data
                .get_data::<InherentType>(&INHERENT_IDENTIFIER)
                .expect("Gets and decodes authorship inherent data")?;
            let lighthouse = T::Lighthouse::decode(&mut &lighthouse_raw[..])
                .expect("Decodes author raw inherent data");
            Some(Call::set_lighthouse(lighthouse))
        }

        fn check_inherent(_call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
            Ok(())
        }
    }
}

/// Lighthouse inherent identifier
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"lgthouse";

#[derive(codec::Encode)]
#[cfg_attr(feature = "std", derive(Debug, codec::Decode))]
pub enum InherentError {
    Other(sp_runtime::RuntimeString),
}

impl IsFatalError for InherentError {
    fn is_fatal_error(&self) -> bool {
        match *self {
            InherentError::Other(_) => true,
        }
    }
}

impl InherentError {
    /// Try to create an instance ouf of the given identifier and data.
    #[cfg(feature = "std")]
    pub fn try_from(id: &InherentIdentifier, data: &[u8]) -> Option<Self> {
        if id == &INHERENT_IDENTIFIER {
            <InherentError as codec::Decode>::decode(&mut &data[..]).ok()
        } else {
            None
        }
    }
}

/// The type of data that the inherent will contain.
/// Just a byte array. It will be decoded to an actual account id later.
pub type InherentType = sp_std::vec::Vec<u8>;

/// The thing that the outer node will use to actually inject the inherent data
#[cfg(feature = "std")]
pub struct InherentDataProvider(pub InherentType);

#[cfg(feature = "std")]
impl ProvideInherentData for InherentDataProvider {
    fn inherent_identifier(&self) -> &'static InherentIdentifier {
        &INHERENT_IDENTIFIER
    }

    fn provide_inherent_data(
        &self,
        inherent_data: &mut InherentData,
    ) -> Result<(), sp_inherents::Error> {
        inherent_data.put_data(INHERENT_IDENTIFIER, &self.0)
    }

    fn error_to_string(&self, error: &[u8]) -> Option<String> {
        InherentError::try_from(&INHERENT_IDENTIFIER, error).map(|e| format!("{:?}", e))
    }
}
