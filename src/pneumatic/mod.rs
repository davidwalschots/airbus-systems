//! As we've not yet modelled pneumatic systems and some pneumatic things are needed for the APU, for now this implementation will be very simple.

use crate::overhead::OnOffPushButton;

#[derive(Debug)]
pub struct PneumaticOverheadPanel {
    apu_bleed: OnOffPushButton,
}
impl PneumaticOverheadPanel {
    pub fn new() -> Self {
        PneumaticOverheadPanel {
            apu_bleed: OnOffPushButton::new_on(),
        }
    }

    pub fn apu_bleed_is_on(&self) -> bool {
        self.apu_bleed.is_on()
    }

    pub fn turn_apu_bleed_on(&mut self) {
        self.apu_bleed.turn_on();
    }

    pub fn turn_apu_bleed_off(&mut self) {
        self.apu_bleed.turn_off();
    }
}

#[derive(Debug)]
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
