use crate::{
    overhead::OnOffPushButton,
    simulator::{SimulatorReadState, SimulatorReadWritable, SimulatorVisitable, SimulatorVisitor},
};

pub struct A320PneumaticOverheadPanel {
    apu_bleed: OnOffPushButton,
}
impl A320PneumaticOverheadPanel {
    pub fn new() -> Self {
        A320PneumaticOverheadPanel {
            apu_bleed: OnOffPushButton::new_on(),
        }
    }

    pub fn apu_bleed_is_on(&self) -> bool {
        self.apu_bleed.is_on()
    }

    #[cfg(test)]
    pub fn turn_apu_bleed_on(&mut self) {
        self.apu_bleed.turn_on();
    }

    #[cfg(test)]
    pub fn turn_apu_bleed_off(&mut self) {
        self.apu_bleed.turn_off();
    }
}
impl SimulatorVisitable for A320PneumaticOverheadPanel {
    fn accept<T: SimulatorVisitor>(&mut self, visitor: &mut T) {
        visitor.visit(self);
    }
}
impl SimulatorReadWritable for A320PneumaticOverheadPanel {
    fn read(&mut self, state: &SimulatorReadState) {
        self.apu_bleed.set(state.apu_bleed_sw_on);
    }
}
