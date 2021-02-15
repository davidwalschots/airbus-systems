use super::{from_bool, SimulatorReaderWriter, UpdateContext};
use std::time::Duration;
use uom::si::{f64::*, length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

pub struct TestReaderWriter {
    variables: Vec<(String, f64)>,
}
impl TestReaderWriter {
    pub fn new() -> Self {
        Self { variables: vec![] }
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
impl SimulatorReaderWriter for TestReaderWriter {
    fn read(&mut self, name: &str) -> f64 {
        match self.variables.iter().find(|x| x.0 == name).map(|x| x.1) {
            Some(value) => value,
            None => 0.,
        }
    }

    fn write(&mut self, name: &str, value: f64) {
        self.variables.push((name.to_owned(), value));
    }
}

pub fn context_with() -> UpdateContextBuilder {
    UpdateContextBuilder::new()
}

pub fn context() -> UpdateContext {
    context_with().build()
}

pub struct UpdateContextBuilder {
    delta: Duration,
    indicated_airspeed: Velocity,
    indicated_altitude: Length,
    ambient_temperature: ThermodynamicTemperature,
}
impl UpdateContextBuilder {
    fn new() -> UpdateContextBuilder {
        UpdateContextBuilder {
            delta: Duration::from_secs(1),
            indicated_airspeed: Velocity::new::<knot>(250.),
            indicated_altitude: Length::new::<foot>(5000.),
            ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(0.),
        }
    }

    pub fn build(&self) -> UpdateContext {
        UpdateContext::new(
            self.delta,
            self.indicated_airspeed,
            self.indicated_altitude,
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
}
