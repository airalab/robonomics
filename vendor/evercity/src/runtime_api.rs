use frame_support::dispatch::Vec;

sp_api::decl_runtime_apis! {
    pub trait BondApi {
        /// delegate call to the pallet get_impact_reports()
        fn get_impact_reports(bond: crate::BondId)->Vec<crate::PeriodDataStruct>;
    }
}
