use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorReadState,
};
use uom::si::{f64::*, mass::kilogram};

pub struct A320Fuel {
    unlimited_fuel: bool,
    left_inner_tank_fuel_quantity: Mass,
}
impl A320Fuel {
    pub fn new() -> Self {
        A320Fuel {
            unlimited_fuel: false,
            left_inner_tank_fuel_quantity: Mass::new::<kilogram>(0.),
        }
    }

    pub fn update(&mut self) {}

    pub fn left_inner_tank_has_fuel_remaining(&self) -> bool {
        self.unlimited_fuel || self.left_inner_tank_fuel_quantity > Mass::new::<kilogram>(0.)
    }
}
impl SimulatorElementVisitable for A320Fuel {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for A320Fuel {
    fn read(&mut self, state: &SimulatorReadState) {
        self.unlimited_fuel = state.unlimited_fuel;
        self.left_inner_tank_fuel_quantity = state.left_inner_tank_fuel_quantity;
    }
}
