use uom::si::{f64::*, ratio::percent};

use crate::simulator::{
    SimulatorReadState, SimulatorReadWritable, SimulatorVisitable, SimulatorVisitor, UpdateContext,
};

pub struct Engine {
    index: usize,
    pub n2: Ratio,
}
impl Engine {
    pub fn new(index: usize) -> Engine {
        Engine {
            index,
            n2: Ratio::new::<percent>(0.),
        }
    }

    pub fn update(&mut self, _: &UpdateContext) {}
}
impl SimulatorVisitable for Engine {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorReadWritable for Engine {
    fn read(&mut self, state: &SimulatorReadState) {
        self.n2 = state.engine_n2[self.index];
    }
}
