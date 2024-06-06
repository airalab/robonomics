use frame_support::weights::Weight;

pub trait WeightInfo {
    fn record() -> Weight;
    fn erase(win: u64) -> Weight;
}

impl WeightInfo for () {
    fn record() -> Weight {
        Default::default()
    }

    fn erase(win: u64) -> Weight {
        Default::default()
    }
}
