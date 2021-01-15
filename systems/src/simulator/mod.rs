use std::time::Duration;

use uom::si::f64::*;

mod update_context;
#[cfg(test)]
pub use update_context::test_helpers;
pub use update_context::UpdateContext;

pub struct SimulatorToModelVisitor<'a> {
    state: &'a SimulatorReadState,
}
impl<'a> SimulatorToModelVisitor<'a> {
    pub fn new(state: &'a SimulatorReadState) -> Self {
        SimulatorToModelVisitor { state }
    }
}
impl SimulatorVisitor for SimulatorToModelVisitor<'_> {
    fn visit<T: SimulatorReadWritable>(&mut self, visited: &mut T) {
        visited.read(&self.state);
    }
}

pub struct ModelToSimulatorVisitor {
    state: SimulatorWriteState,
}
impl ModelToSimulatorVisitor {
    pub fn new() -> Self {
        ModelToSimulatorVisitor {
            state: Default::default(),
        }
    }

    pub fn get_state(self) -> SimulatorWriteState {
        self.state
    }
}
impl SimulatorVisitor for ModelToSimulatorVisitor {
    fn visit<T: SimulatorReadWritable>(&mut self, visited: &mut T) {
        visited.write(&mut self.state);
    }
}

pub fn to_bool(value: f64) -> bool {
    value == 1.
}

pub fn from_bool(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

pub trait SimulatorReadWritable {
    /// Reads simulator state data into the struct.
    fn read(&mut self, state: &SimulatorReadState) {}

    /// Writes struct data into the simulator's state.
    fn write(&self, state: &mut SimulatorWriteState) {}
}

pub trait SimulatorVisitable {
    fn accept<T: SimulatorVisitor>(&mut self, visitor: &mut T);
}

pub trait SimulatorVisitor {
    fn visit<T: SimulatorReadWritable>(&mut self, visited: &mut T);
}

#[derive(Default)]
pub struct SimulatorReadState {
    pub ambient_temperature: ThermodynamicTemperature,
    pub apu_master_sw_on: bool,
    pub apu_start_sw_on: bool,
    pub apu_bleed_sw_on: bool,
    pub indicated_airspeed: Velocity,
    pub indicated_altitude: Length,
}
impl SimulatorReadState {
    pub fn to_context(&self, delta_time: Duration) -> UpdateContext {
        UpdateContext {
            ambient_temperature: self.ambient_temperature,
            indicated_airspeed: self.indicated_airspeed,
            indicated_altitude: self.indicated_altitude,
            delta: delta_time,
        }
    }
}

#[derive(Default)]
pub struct SimulatorWriteState {
    pub apu_caution_egt: ThermodynamicTemperature,
    pub apu_egt: ThermodynamicTemperature,
    pub apu_gen_current: ElectricCurrent,
    pub apu_gen_frequency: Frequency,
    pub apu_gen_potential: ElectricPotential,
    pub apu_n: Ratio,
    pub apu_start_contactor_energized: bool,
    pub apu_start_sw_available: bool,
    pub apu_start_sw_on: bool,
    pub apu_warning_egt: ThermodynamicTemperature,
    pub apu_air_intake_flap_opened_for: Ratio,
}
