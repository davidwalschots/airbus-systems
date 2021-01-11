//! As we've not yet modelled pneumatic systems and some pneumatic things are needed for the APU, for now this implementation will be very simple.

pub struct BleedAirValve {
    open: bool,
}
impl BleedAirValve {
    pub fn new() -> Self {
        BleedAirValve { open: false }
    }

    pub fn open_when(&mut self, condition: bool) {
        self.open = condition;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }
}
