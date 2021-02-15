use std::time::Duration;
use uom::si::f64::*;

/// Provides data unowned by any system in the aircraft system simulation
/// for the purpose of handling an update frame.
#[derive(Debug)]
pub struct UpdateContext {
    pub delta: Duration,
    pub indicated_airspeed: Velocity,
    pub indicated_altitude: Length,
    pub ambient_temperature: ThermodynamicTemperature,
}
impl UpdateContext {
    pub fn new(
        delta: Duration,
        indicated_airspeed: Velocity,
        indicated_altitude: Length,
        ambient_temperature: ThermodynamicTemperature,
    ) -> UpdateContext {
        UpdateContext {
            delta,
            indicated_airspeed,
            indicated_altitude,
            ambient_temperature,
        }
    }
}
