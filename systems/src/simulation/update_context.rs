use std::time::Duration;
use uom::si::{f64::*, length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

use super::SimulatorReader;

/// Provides data unowned by any system in the aircraft system simulation
/// for the purpose of handling a simulation tick.
#[derive(Debug)]
pub struct UpdateContext {
    pub delta: Duration,
    pub indicated_airspeed: Velocity,
    pub indicated_altitude: Length,
    pub ambient_temperature: ThermodynamicTemperature,
    pub is_on_ground: bool,
}
impl UpdateContext {
    pub(crate) const AMBIENT_TEMPERATURE_KEY: &'static str = "AMBIENT TEMPERATURE";
    pub(crate) const INDICATED_AIRSPEED_KEY: &'static str = "AIRSPEED INDICATED";
    pub(crate) const INDICATED_ALTITUDE_KEY: &'static str = "INDICATED ALTITUDE";
    pub(crate) const IS_ON_GROUND_KEY: &'static str = "SIM ON GROUND";

    pub fn new(
        delta: Duration,
        indicated_airspeed: Velocity,
        indicated_altitude: Length,
        ambient_temperature: ThermodynamicTemperature,
        is_on_ground: bool,
    ) -> UpdateContext {
        UpdateContext {
            delta,
            indicated_airspeed,
            indicated_altitude,
            ambient_temperature,
            is_on_ground,
        }
    }

    /// Creates a context based on the data that was read from the simulator.
    pub(super) fn from_reader(reader: &mut SimulatorReader, delta_time: Duration) -> UpdateContext {
        UpdateContext {
            ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(
                reader.read_f64(UpdateContext::AMBIENT_TEMPERATURE_KEY),
            ),
            indicated_airspeed: Velocity::new::<knot>(
                reader.read_f64(UpdateContext::INDICATED_AIRSPEED_KEY),
            ),
            indicated_altitude: Length::new::<foot>(
                reader.read_f64(UpdateContext::INDICATED_ALTITUDE_KEY),
            ),
            is_on_ground: reader.read_bool(UpdateContext::IS_ON_GROUND_KEY),
            delta: delta_time,
        }
    }

    pub fn is_in_flight(&self) -> bool {
        !self.is_on_ground
    }
}

pub fn context_with() -> UpdateContextBuilder {
    UpdateContextBuilder::new()
}

pub struct UpdateContextBuilder {
    delta: Duration,
    indicated_airspeed: Velocity,
    indicated_altitude: Length,
    ambient_temperature: ThermodynamicTemperature,
    on_ground: bool,
}
impl UpdateContextBuilder {
    fn new() -> UpdateContextBuilder {
        UpdateContextBuilder {
            delta: Duration::from_secs(1),
            indicated_airspeed: Velocity::new::<knot>(250.),
            indicated_altitude: Length::new::<foot>(5000.),
            ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(0.),
            on_ground: false,
        }
    }

    pub fn build(&self) -> UpdateContext {
        UpdateContext::new(
            self.delta,
            self.indicated_airspeed,
            self.indicated_altitude,
            self.ambient_temperature,
            self.on_ground,
        )
    }

    pub fn and(self) -> UpdateContextBuilder {
        self
    }

    pub fn delta(mut self, delta: Duration) -> UpdateContextBuilder {
        self.delta = delta;
        self
    }

    pub fn indicated_airspeed(mut self, indicated_airspeed: Velocity) -> UpdateContextBuilder {
        self.indicated_airspeed = indicated_airspeed;
        self
    }

    pub fn indicated_altitude(mut self, indicated_altitude: Length) -> UpdateContextBuilder {
        self.indicated_altitude = indicated_altitude;
        self
    }

    pub fn ambient_temperature(
        mut self,
        ambient_temperature: ThermodynamicTemperature,
    ) -> UpdateContextBuilder {
        self.ambient_temperature = ambient_temperature;
        self
    }

    pub fn on_ground(mut self, on_ground: bool) -> UpdateContextBuilder {
        self.on_ground = on_ground;
        self
    }
}
