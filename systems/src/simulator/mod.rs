use uom::si::f64::*;

mod update_context;
#[cfg(test)]
pub use update_context::test_helpers;
pub use update_context::UpdateContext;

pub struct SimToModelVisitor {
    state: SimulatorReadState,
}
impl SimToModelVisitor {
    pub fn new(state: SimulatorReadState) -> Self {
        SimToModelVisitor { state }
    }
}
impl SimVisitor for SimToModelVisitor {
    fn visit<T: SimulatorReadWritable>(&mut self, visited: &mut T) {
        visited.read(&self.state);
    }
}

pub struct ModelToSimVisitor {
    state: SimulatorWriteState,
}
impl ModelToSimVisitor {
    pub fn new() -> Self {
        ModelToSimVisitor {
            state: Default::default(),
        }
    }

    pub fn get_state(self) -> SimulatorWriteState {
        self.state
    }
}
impl SimVisitor for ModelToSimVisitor {
    fn visit<T: SimulatorReadWritable>(&mut self, visited: &mut T) {
        visited.write(&mut self.state);
    }
}

pub fn to_bool(value: f64) -> bool {
    value == 1.
}

pub trait SimulatorReadWritable {
    /// Reads simulator state data into the struct.
    fn read(&mut self, state: &SimulatorReadState) {}

    /// Writes struct data into the simulator's state.
    fn write(&self, state: &mut SimulatorWriteState) {}
}

pub trait SimulatorVisitable {
    fn accept<T: SimVisitor>(&mut self, visitor: &mut T);
}

pub trait SimVisitor {
    fn visit<T: SimulatorReadWritable>(&mut self, visited: &mut T);
}

#[derive(Default)]
pub struct SimulatorReadState {
    pub apu_master_sw_on: bool,
    pub apu_start_sw_on: bool,
    pub apu_bleed_sw_on: bool,
}

#[derive(Default)]
pub struct SimulatorWriteState {
    pub apu_n: Ratio,
    pub apu_egt: ThermodynamicTemperature,
    pub apu_caution_egt: ThermodynamicTemperature,
    pub apu_warning_egt: ThermodynamicTemperature,
}
