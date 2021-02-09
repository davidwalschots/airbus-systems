use super::{Current, ElectricPowerSource, ElectricSource, PowerConsumptionState};
use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorWriteState,
};
use uom::si::{electric_potential::volt, f64::*, frequency::hertz};

pub struct EmergencyGenerator {
    running: bool,
    is_blue_pressurised: bool,
}
impl EmergencyGenerator {
    pub fn new() -> EmergencyGenerator {
        EmergencyGenerator {
            running: false,
            is_blue_pressurised: false,
        }
    }

    pub fn update(&mut self, is_blue_pressurised: bool) {
        // TODO: The emergency generator is driven by the blue hydraulic circuit. Still to be implemented.
        self.is_blue_pressurised = is_blue_pressurised;
    }

    #[cfg(test)]
    pub fn attempt_start(&mut self) {
        self.running = true;
    }

    pub fn is_running(&self) -> bool {
        self.is_blue_pressurised && self.running
    }
}
impl ElectricSource for EmergencyGenerator {
    fn output(&self) -> Current {
        if self.is_running() {
            Current::some(ElectricPowerSource::EmergencyGenerator)
        } else {
            Current::none()
        }
    }
}
impl SimulatorElementVisitable for EmergencyGenerator {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for EmergencyGenerator {
    fn write_power_consumption(&mut self, state: &PowerConsumptionState) {
        // TODO
    }

    fn write(&self, state: &mut SimulatorWriteState) {
        // TODO: Replace with actual values once calculated.
        state.electrical.emergency_generator.frequency = if self.output().is_powered() {
            Frequency::new::<hertz>(400.)
        } else {
            Frequency::new::<hertz>(0.)
        };
        state
            .electrical
            .emergency_generator
            .frequency_within_normal_range = if self.output().is_powered() {
            true
        } else {
            false
        };
        state.electrical.emergency_generator.potential = if self.output().is_powered() {
            ElectricPotential::new::<volt>(115.)
        } else {
            ElectricPotential::new::<volt>(0.)
        };
        state
            .electrical
            .emergency_generator
            .potential_within_normal_range = if self.output().is_powered() {
            true
        } else {
            false
        };
    }
}

#[cfg(test)]
mod emergency_generator_tests {
    use super::*;

    #[test]
    fn starts_without_output() {
        assert!(emergency_generator().is_unpowered());
    }

    #[test]
    fn when_started_provides_output() {
        let mut emer_gen = emergency_generator();
        emer_gen.attempt_start();
        emer_gen.update(true);

        assert!(emer_gen.is_powered());
    }

    #[test]
    fn when_started_without_hydraulic_pressure_is_unpowered() {
        let mut emer_gen = emergency_generator();
        emer_gen.attempt_start();
        emer_gen.update(false);

        assert!(emer_gen.is_unpowered());
    }

    fn emergency_generator() -> EmergencyGenerator {
        EmergencyGenerator::new()
    }
}
