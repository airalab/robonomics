use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub trait WeightInfo {
    fn record() -> Weight;
    fn erase(win: u64) -> Weight;
}

#[allow(clippy::unnecessary_cast)]
impl WeightInfo for () {
    fn record() -> Weight {
        (500_000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(2_u64 as Weight))
            .saturating_add(DbWeight::get().writes(3_u64 as Weight))
    }

    fn erase(win: u64) -> Weight {
        (100_000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(1_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 + win as Weight))
    }
}