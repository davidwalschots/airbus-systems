//! Provides all the necessary types for integrating the
//! crate into a Microsoft Flight Simulator aircraft.
use std::time::Duration;
use uom::si::{f64::*, length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

mod update_context;
#[cfg(test)]
pub use update_context::test_helpers;
pub use update_context::UpdateContext;

use crate::electrical::{PowerConsumptionState, PowerSupply};

/// Trait for reading data from and writing data to the simulator.
pub trait SimulatorReadWriter {
    fn read(&mut self, name: &str) -> f64;
    fn write(&mut self, name: &str, value: f64);
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
        let mut reader = SimulatorReader::new(&mut self.simulator_read_writer);
        let context = reader.get_context(delta);

        let mut visitor = SimulatorToModelVisitor::new(&mut reader);
        self.aircraft.accept(&mut Box::new(&mut visitor));

        self.aircraft.update(&context);

        let mut writer = SimulatorWriter::new(&mut self.simulator_read_writer);
        let mut visitor = ModelToSimulatorVisitor::new(&mut writer);
        self.aircraft.accept(&mut Box::new(&mut visitor));
    }
}

/// Visits aircraft components in order to pass data coming
/// from the simulator into the aircraft system simulation.
struct SimulatorToModelVisitor<'a> {
    reader: &'a mut SimulatorReader<'a>,
}
impl<'a> SimulatorToModelVisitor<'a> {
    pub fn new(reader: &'a mut SimulatorReader<'a>) -> Self {
        SimulatorToModelVisitor { reader }
    }
}
impl SimulatorElementVisitor for SimulatorToModelVisitor<'_> {
    fn visit(&mut self, visited: &mut Box<&mut dyn SimulatorElement>) {
        visited.read(&mut self.reader);
    }
}

/// Visits aircraft components in order to pass data from
/// the aircraft system simulation to the simulator.
pub struct ModelToSimulatorVisitor<'a> {
    writer: &'a mut SimulatorWriter<'a>,
}
impl<'a> ModelToSimulatorVisitor<'a> {
    pub fn new(writer: &'a mut SimulatorWriter<'a>) -> Self {
        ModelToSimulatorVisitor { writer }
    }
}
impl<'a> SimulatorElementVisitor for ModelToSimulatorVisitor<'a> {
    fn visit(&mut self, visited: &mut Box<&mut dyn SimulatorElement>) {
        visited.write(&mut self.writer);
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
    fn read(&mut self, _state: &mut SimulatorReader) {}

    /// Writes data from the aircraft system simulation to a model which can be passed to the simulator.
    fn write(&self, _state: &mut SimulatorWriter) {}

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

/// The data which is read from the simulator and can
/// be passed into the aircraft system simulation.
pub struct SimulatorReader<'a> {
    simulator_read_writer: &'a mut dyn SimulatorReadWriter,
}
impl<'a> SimulatorReader<'a> {
    /// Creates a context based on the data that was read from the simulator.
    pub fn get_context(&mut self, delta_time: Duration) -> UpdateContext {
        UpdateContext {
            ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(
                self.get_f64("AMBIENT_TEMPERATURE"),
            ),
            indicated_airspeed: Velocity::new::<knot>(self.get_f64("INDICATED_AIRSPEED")),
            indicated_altitude: Length::new::<foot>(self.get_f64("INDICATED_ALTITUDE")),
            delta: delta_time,
        }
    }

    pub fn new(simulator_read_writer: &'a mut dyn SimulatorReadWriter) -> Self {
        Self {
            simulator_read_writer,
        }
    }

    pub fn get_f64(&mut self, name: &str) -> f64 {
        self.simulator_read_writer.read(name)
    }

    pub fn get_bool(&mut self, name: &str) -> bool {
        to_bool(self.get_f64(name))
    }
}

#[derive(Default)]
pub struct SimulatorApuReadState {
    pub master_sw_pb_on: bool,
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
    pub external_power_pb_available: bool,
    pub external_power_pb_on: bool,
}

/// The data which is written from the aircraft system simulation
/// into the the simulator.
pub struct SimulatorWriter<'a> {
    pub variables: Vec<(String, f64)>,
    simulator_read_writer: Option<&'a mut dyn SimulatorReadWriter>,
}
impl<'a> SimulatorWriter<'a> {
    pub fn new(simulator_read_writer: &'a mut dyn SimulatorReadWriter) -> Self {
        Self {
            variables: Default::default(),
            simulator_read_writer: Some(simulator_read_writer),
        }
    }

    pub fn new_for_test() -> Self {
        Self {
            variables: Default::default(),
            simulator_read_writer: None,
        }
    }

    #[cfg(not(test))]
    pub fn write_f64(&mut self, name: &str, value: f64) {
        self.simulator_read_writer
            .as_mut()
            .unwrap()
            .write(name, value);
    }

    #[cfg(not(test))]
    pub fn write_bool(&mut self, name: &str, value: bool) {
        self.simulator_read_writer
            .as_mut()
            .unwrap()
            .write(name, from_bool(value))
    }

    #[cfg(test)]
    pub fn write_f64(&mut self, name: &str, value: f64) {
        self.variables.push((name.to_owned(), value));
    }

    #[cfg(test)]
    pub fn write_bool(&mut self, name: &str, value: bool) {
        self.variables.push((name.to_owned(), from_bool(value)));
    }

    pub fn contains_f64(&self, name: &str, value: f64) -> bool {
        self.variables.iter().any(|x| x.0 == name && x.1 == value)
    }

    pub fn contains_bool(&self, name: &str, value: bool) -> bool {
        self.contains_f64(name, from_bool(value))
    }

    pub fn len_is(&self, length: usize) -> bool {
        self.variables.len() == length
    }
}
