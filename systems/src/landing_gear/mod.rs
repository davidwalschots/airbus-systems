use crate::simulation::{SimulationElement, SimulatorReader};

pub struct LandingGear {
    is_down: bool,
}
impl LandingGear {
    pub fn new() -> Self {
        Self { is_down: false }
    }

    pub fn is_down(&self) -> bool {
        self.is_down
    }
}
impl SimulationElement for LandingGear {
    fn read(&mut self, reader: &mut SimulatorReader) {
        self.is_down = (reader.read_f64("GEAR POSITION") - 2.).abs() < f64::EPSILON;
    }
}
