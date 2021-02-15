//! Provides all the necessary types for integrating the
//! crate into a Microsoft Flight Simulator aircraft.
use std::time::Duration;
use uom::si::{f64::*, length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

mod update_context;
pub use update_context::*;

#[cfg(test)]
mod test;
#[cfg(test)]
pub use test::*;

use crate::electrical::{PowerConsumptionState, PowerSupply};

/// Trait for reading data from and writing data to the simulator.
pub trait SimulatorReaderWriter {
    fn read(&mut self, name: &str) -> f64;
    fn write(&mut self, name: &str, value: f64);
}

pub trait Aircraft: SimulatorElement {
    fn update(&mut self, context: &UpdateContext);
}

/// Orchestrates the:
/// 1. Reading of data from the simulator into the aircraft state.
/// 2. Updating of the aircraft state for each tick.
/// 3. Writing of aircraft state data to the simulator.
pub struct Simulation<T: Aircraft, U: SimulatorReaderWriter> {
    aircraft: T,
    simulator_read_writer: U,
}
impl<T: Aircraft, U: SimulatorReaderWriter> Simulation<T, U> {
    pub fn new(aircraft: T, simulator_read_writer: U) -> Self {
        Simulation {
            aircraft,
            simulator_read_writer,
        }
    }

    pub fn tick(&mut self, delta: Duration) {
        let mut reader = SimulatorElementReader::new(&mut self.simulator_read_writer);
        let context = reader.get_context(delta);

        let mut visitor = SimulatorToModelVisitor::new(&mut reader);
        self.aircraft.accept(&mut visitor);

        self.aircraft.update(&context);

        let mut writer = SimulatorElementWriter::new(&mut self.simulator_read_writer);
        let mut visitor = ModelToSimulatorVisitor::new(&mut writer);
        self.aircraft.accept(&mut visitor);
    }
}

/// Visits aircraft components in order to pass data coming
/// from the simulator into the aircraft system simulation.
struct SimulatorToModelVisitor<'a> {
    reader: &'a mut SimulatorElementReader<'a>,
}
impl<'a> SimulatorToModelVisitor<'a> {
    pub fn new(reader: &'a mut SimulatorElementReader<'a>) -> Self {
        SimulatorToModelVisitor { reader }
    }
}
impl SimulatorElementVisitor for SimulatorToModelVisitor<'_> {
    fn visit<T: SimulatorElement>(&mut self, visited: &mut T) {
        visited.read(&mut self.reader);
    }
}

/// Visits aircraft components in order to pass data from
/// the aircraft system simulation to the simulator.
pub struct ModelToSimulatorVisitor<'a> {
    writer: &'a mut SimulatorElementWriter<'a>,
}
impl<'a> ModelToSimulatorVisitor<'a> {
    pub fn new(writer: &'a mut SimulatorElementWriter<'a>) -> Self {
        ModelToSimulatorVisitor { writer }
    }
}
impl<'a> SimulatorElementVisitor for ModelToSimulatorVisitor<'a> {
    fn visit<T: SimulatorElement>(&mut self, visited: &mut T) {
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
    fn accept<T: SimulatorElementVisitor>(&mut self, visitor: &mut T)
    where
        Self: Sized,
    {
        visitor.visit(self);
    }

    /// Reads data representing the current state of the simulator into the aircraft system simulation.
    fn read(&mut self, _reader: &mut SimulatorElementReader) {}

    /// Writes data from the aircraft system simulation to a model which can be passed to the simulator.
    fn write(&self, _writer: &mut SimulatorElementWriter) {}

    /// Supplies the element with power when available.
    fn supply_power(&mut self, _supply: &PowerSupply) {}

    /// Determines the electrical demand of the element at this time.
    fn determine_power_consumption(&mut self, _state: &mut PowerConsumptionState) {}

    /// Writes electrical consumption to elements that can cater to such demand.
    fn write_power_consumption(&mut self, _state: &PowerConsumptionState) {}
}

/// Trait for visitors that visit the aircraft's system simulation.
pub trait SimulatorElementVisitor {
    fn visit<T: SimulatorElement>(&mut self, visited: &mut T);
}

/// The data which is read from the simulator and can
/// be passed into the aircraft system simulation.
pub struct SimulatorElementReader<'a> {
    simulator_read_writer: &'a mut dyn SimulatorReaderWriter,
}
impl<'a> SimulatorElementReader<'a> {
    /// Creates a context based on the data that was read from the simulator.
    pub fn get_context(&mut self, delta_time: Duration) -> UpdateContext {
        UpdateContext {
            ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(
                self.read_f64("AMBIENT TEMPERATURE"),
            ),
            indicated_airspeed: Velocity::new::<knot>(self.read_f64("AIRSPEED INDICATED")),
            indicated_altitude: Length::new::<foot>(self.read_f64("INDICATED ALTITUDE")),
            delta: delta_time,
        }
    }

    pub fn new(simulator_read_writer: &'a mut dyn SimulatorReaderWriter) -> Self {
        Self {
            simulator_read_writer,
        }
    }

    pub fn read_f64(&mut self, name: &str) -> f64 {
        self.simulator_read_writer.read(name)
    }

    pub fn read_bool(&mut self, name: &str) -> bool {
        to_bool(self.read_f64(name))
    }
}

/// The data which is written from the aircraft system simulation
/// into the the simulator.
pub struct SimulatorElementWriter<'a> {
    simulator_read_writer: &'a mut dyn SimulatorReaderWriter,
}
impl<'a> SimulatorElementWriter<'a> {
    pub fn new(simulator_read_writer: &'a mut dyn SimulatorReaderWriter) -> Self {
        Self {
            simulator_read_writer: simulator_read_writer,
        }
    }

    pub fn write_f64(&mut self, name: &str, value: f64) {
        self.simulator_read_writer.write(name, value);
    }

    pub fn write_bool(&mut self, name: &str, value: bool) {
        self.simulator_read_writer.write(name, from_bool(value));
    }
}
