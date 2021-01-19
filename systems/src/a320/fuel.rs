use crate::simulator::{
    SimulatorReadState, SimulatorReadWritable, SimulatorVisitable, SimulatorVisitor,
};
use uom::si::{f64::*, mass::kilogram};

pub struct A320Fuel {
    unlimited_fuel: bool,
    total_fuel_weight: Mass,
}
impl A320Fuel {
    pub fn new() -> Self {
        A320Fuel {
            unlimited_fuel: false,
            total_fuel_weight: Mass::new::<kilogram>(0.),
        }
    }

    pub fn update(&mut self) {}

    pub fn has_fuel_remaining(&self) -> bool {
        self.unlimited_fuel || self.total_fuel_weight > Mass::new::<kilogram>(0.)
    }
}
impl SimulatorVisitable for A320Fuel {
    fn accept<T: SimulatorVisitor>(&mut self, visitor: &mut T) {
        visitor.visit(self);
    }
}
impl SimulatorReadWritable for A320Fuel {
    fn read(&mut self, state: &SimulatorReadState) {
        self.unlimited_fuel = state.unlimited_fuel;
        self.total_fuel_weight = state.total_fuel_weight;
    }
}
