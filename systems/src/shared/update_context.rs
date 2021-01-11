use std::time::Duration;

use uom::si::f64::*;

#[derive(Debug)]
pub struct UpdateContext {
    pub delta: Duration,
    pub airspeed: Velocity,
    pub above_ground_level: Length,
    pub ambient_temperature: ThermodynamicTemperature,
}

impl UpdateContext {
    pub fn new(
        delta: Duration,
        airspeed: Velocity,
        above_ground_level: Length,
        ambient_temperature: ThermodynamicTemperature,
    ) -> UpdateContext {
        UpdateContext {
            delta,
            airspeed,
            above_ground_level,
            ambient_temperature,
        }
    }
}

#[cfg(test)]
pub mod test_helpers {
    use super::*;

    use uom::si::{length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

    pub fn context_with() -> UpdateContextBuilder {
        UpdateContextBuilder::new()
    }

    pub struct UpdateContextBuilder {
        delta: Duration,
        airspeed: Velocity,
        above_ground_level: Length,
        ambient_temperature: ThermodynamicTemperature,
    }
    impl UpdateContextBuilder {
        fn new() -> UpdateContextBuilder {
            UpdateContextBuilder {
                delta: Duration::from_secs(1),
                airspeed: Velocity::new::<knot>(250.),
                above_ground_level: Length::new::<foot>(5000.),
                ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(0.),
            }
        }

        pub fn build(&self) -> UpdateContext {
            UpdateContext::new(
                self.delta,
                self.airspeed,
                self.above_ground_level,
                self.ambient_temperature,
            )
        }

        pub fn and(self) -> UpdateContextBuilder {
            self
        }

        pub fn delta(mut self, delta: Duration) -> UpdateContextBuilder {
            self.delta = delta;
            self
        }

        pub fn airspeed(mut self, airspeed: Velocity) -> UpdateContextBuilder {
            self.airspeed = airspeed;
            self
        }

        pub fn above_ground_level(mut self, above_ground_level: Length) -> UpdateContextBuilder {
            self.above_ground_level = above_ground_level;
            self
        }

        pub fn ambient_temperature(
            mut self,
            ambient_temperature: ThermodynamicTemperature,
        ) -> UpdateContextBuilder {
            self.ambient_temperature = ambient_temperature;
            self
        }
    }
}
