//! Provides all the necessary types for integrating the
//! crate into a Microsoft Flight Simulator aircraft.
use std::{collections::HashMap, time::Duration};
use uom::si::f64::*;

mod update_context;
#[cfg(test)]
pub use update_context::test_helpers;
pub use update_context::UpdateContext;

use crate::electrical::{PowerConsumptionState, PowerSupply};

/// Trait for reading data from and writing data to the simulator.
pub trait SimulatorReadWriter {
    /// Reads data from the simulator into a model representing that state.
    fn read(&mut self) -> SimulatorReadState;
    /// Writes data from a model into the simulator.
    fn write(&mut self, state: &SimulatorWriteState);
}

pub trait Aircraft: SimulatorElementVisitable {
    fn update(&mut self, context: &UpdateContext);
}

/// Orchestrates the:
/// 1. Reading of data from the simulator into the aircraft state.
/// 2. Updating of the aircraft state for each tick.
/// 3. Writing of aircraft state data to the simulator.
pub struct Simulation<T: Aircraft, U: SimulatorReadWriter> {
    aircraft: T,
    simulator_read_writer: U,
}
impl<T: Aircraft, U: SimulatorReadWriter> Simulation<T, U> {
    pub fn new(aircraft: T, simulator_read_writer: U) -> Self {
        Simulation {
            aircraft,
            simulator_read_writer,
        }
    }

    pub fn tick(&mut self, delta: Duration) {
        let state = self.simulator_read_writer.read();
        let mut visitor = SimulatorToModelVisitor::new(&state);
        self.aircraft.accept(&mut Box::new(&mut visitor));

        self.aircraft.update(&state.to_context(delta));

        let mut visitor = ModelToSimulatorVisitor::new();
        self.aircraft.accept(&mut Box::new(&mut visitor));

        self.simulator_read_writer.write(&visitor.get_state());
    }
}

/// Visits aircraft components in order to pass data coming
/// from the simulator into the aircraft system simulation.
struct SimulatorToModelVisitor<'a> {
    state: &'a SimulatorReadState,
}
impl<'a> SimulatorToModelVisitor<'a> {
    pub fn new(state: &'a SimulatorReadState) -> Self {
        SimulatorToModelVisitor { state }
    }
}
impl SimulatorElementVisitor for SimulatorToModelVisitor<'_> {
    fn visit(&mut self, visited: &mut Box<&mut dyn SimulatorElement>) {
        visited.read(&self.state);
    }
}

/// Visits aircraft components in order to pass data from
/// the aircraft system simulation to the simulator.
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
impl SimulatorElementVisitor for ModelToSimulatorVisitor {
    fn visit(&mut self, visited: &mut Box<&mut dyn SimulatorElement>) {
        visited.write(&mut self.state);
    }
}

/// Converts a given `f64` representing a boolean value in the simulator into an actual `bool` value.
pub fn to_bool(value: f64) -> bool {
    (value - 1.).abs() < f64::EPSILON
}

/// Converts a given `bool` value into an `f64` representing that boolean value in the simulator.
pub fn from_bool(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

/// Trait for an element within the aircraft system simulation.
pub trait SimulatorElement {
    /// Reads data representing the current state of the simulator into the aircraft system simulation.
    fn read(&mut self, _state: &SimulatorReadState) {}

    /// Writes data from the aircraft system simulation to a model which can be passed to the simulator.
    fn write(&self, _state: &mut SimulatorWriteState) {}

    /// Supplies the element with power when available.
    fn supply_power(&mut self, supply: &PowerSupply) {}

    /// Determines the electrical demand of the element at this time.
    fn determine_power_consumption(&mut self, state: &mut PowerConsumptionState) {}

    /// Writes electrical consumption to elements that can cater to such demand.
    fn write_power_consumption(&mut self, _state: &PowerConsumptionState) {}
}

/// Trait for making a piece of the aircraft system simulation visitable.
pub trait SimulatorElementVisitable: SimulatorElement {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>);
}

/// Trait for visitors that visit the aircraft's system simulation.
pub trait SimulatorElementVisitor {
    fn visit(&mut self, visited: &mut Box<&mut dyn SimulatorElement>);
}

pub struct SimulatorVariable {
    name: String,
    value: f64,
}
impl SimulatorVariable {
    pub fn from_f64(name: &str, value: f64) -> Self {
        Self {
            name: name.to_owned(),
            value,
        }
    }

    pub fn from_bool(name: &str, value: bool) -> Self {
        Self {
            name: name.to_owned(),
            value: from_bool(value),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name[..]
    }

    pub fn get_value(&self) -> f64 {
        self.value
    }
}

/// The data which is read from the simulator and can
/// be passed into the aircraft system simulation.
#[derive(Default)]
pub struct SimulatorReadState {
    pub ambient_temperature: ThermodynamicTemperature,
    pub apu: SimulatorApuReadState,
    pub electrical: SimulatorElectricalReadState,
    pub fire: SimulatorFireReadState,
    pub indicated_airspeed: Velocity,
    pub indicated_altitude: Length,
    pub left_inner_tank_fuel_quantity: Mass,
    pub pneumatic: SimulatorPneumaticReadState,
    pub unlimited_fuel: bool,
    pub engine_n2: [Ratio; 2],
}
impl SimulatorReadState {
    /// Creates a context based on the data that was read from the simulator.
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
pub struct SimulatorApuReadState {
    pub master_sw_pb_on: bool,
    pub start_pb_on: bool,
}

#[derive(Default)]
pub struct SimulatorPneumaticReadState {
    pub apu_bleed_pb_on: bool,
}

#[derive(Default)]
pub struct SimulatorFireReadState {
    pub apu_fire_button_released: bool,
}

#[derive(Default)]
pub struct SimulatorElectricalReadState {
    pub ac_ess_feed_pb_normal: bool,
    pub apu_generator_pb_on: bool,
    pub battery_pb_auto: [bool; 2],
    pub bus_tie_pb_auto: bool,
    pub commercial_pb_on: bool,
    pub galy_and_cab_pb_auto: bool,
    pub engine_generator_pb_on: [bool; 2],
    pub idg_pb_released: [bool; 2],
    pub external_power_available: bool,
    pub external_power_pb_on: bool,
}

/// The data which is written from the aircraft system simulation
/// into the the simulator.
#[derive(Default)]
pub struct SimulatorWriteState {
    pub variables: Vec<SimulatorVariable>,

    pub apu: SimulatorApuWriteState,
}
impl SimulatorWriteState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, simulator_variable: SimulatorVariable) {
        self.variables.push(simulator_variable);
    }

    #[cfg(test)]
    pub fn contains_f64(&self, name: &str, value: f64) -> bool {
        self.variables
            .iter()
            .any(|x| x.get_name() == name && x.get_value() == value)
    }

    #[cfg(test)]
    pub fn contains_bool(&self, name: &str, value: bool) -> bool {
        self.contains_f64(name, from_bool(value))
    }

    #[cfg(test)]
    pub fn len_is(&self, length: usize) -> bool {
        self.variables.len() == length
    }

    #[cfg(test)]
    pub fn get(&self, name: &str) -> Option<f64> {
        match self.variables.iter().find(|x| x.get_name() == name) {
            Some(variable) => Some(variable.get_value()),
            None => None,
        }
    }
}

#[derive(Default)]
pub struct SimulatorApuWriteState {
    pub available: bool,
    pub air_intake_flap_is_ecam_open: bool,
    pub air_intake_flap_opened_for: Ratio,
    pub bleed_air_valve_open: bool,
    pub caution_egt: ThermodynamicTemperature,
    pub egt: ThermodynamicTemperature,
    pub inoperable: bool,
    pub is_auto_shutdown: bool,
    pub is_emergency_shutdown: bool,
    pub low_fuel_pressure_fault: bool,
    pub n: Ratio,
    pub start_contactor_energized: bool,
    pub warning_egt: ThermodynamicTemperature,
}
