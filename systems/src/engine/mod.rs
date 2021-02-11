use uom::si::{f64::*, ratio::percent};

use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorReader,
    UpdateContext,
};

pub struct Engine {
    n2_id: String,
    pub n2: Ratio,
}
impl Engine {
    pub fn new(number: usize) -> Engine {
        Engine {
            n2_id: format!("ENG_{}_N2", number),
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
    fn read(&mut self, state: &mut SimulatorReader) {
        self.n2 = Ratio::new::<percent>(state.get_f64(&self.n2_id));
    }
}
