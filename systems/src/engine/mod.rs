use uom::si::{f64::*, ratio::percent};

use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorReadState,
    UpdateContext,
};

pub struct Engine {
    number: usize,
    pub n2: Ratio,
}
impl Engine {
    pub fn new(number: usize) -> Engine {
        Engine {
            number,
            n2: Ratio::new::<percent>(0.),
        }
    }

    pub fn update(&mut self, _: &UpdateContext) {}
}
impl SimulatorElementVisitable for Engine {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for Engine {
    fn read(&mut self, state: &SimulatorReadState) {
        self.n2 = state.engine_n2[self.number - 1];
    }
}
