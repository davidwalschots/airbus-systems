use super::{
    Current, ElectricPowerSource, ElectricSource, ElectricalStateWriter, PowerConsumptionState,
    ProvideFrequency, ProvidePotential,
};
use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorWriter,
};
use uom::si::{electric_potential::volt, f64::*, frequency::hertz};

pub struct EmergencyGenerator {
    writer: ElectricalStateWriter,
    running: bool,
    is_blue_pressurised: bool,
}
impl EmergencyGenerator {
    pub fn new() -> EmergencyGenerator {
        EmergencyGenerator {
            writer: ElectricalStateWriter::new("EMER_GEN"),
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
impl ProvideFrequency for EmergencyGenerator {
    fn get_frequency(&self) -> Frequency {
        // TODO: Replace with actual values once calculated.
        if self.output().is_powered() {
            Frequency::new::<hertz>(400.)
        } else {
            Frequency::new::<hertz>(0.)
        }
    }

    fn get_frequency_normal(&self) -> bool {
        // TODO: Replace with actual values once calculated.
        if self.output().is_powered() {
            true
        } else {
            false
        }
    }
}
impl ProvidePotential for EmergencyGenerator {
    fn get_potential(&self) -> ElectricPotential {
        // TODO: Replace with actual values once calculated.
        if self.output().is_powered() {
            ElectricPotential::new::<volt>(115.)
        } else {
            ElectricPotential::new::<volt>(0.)
        }
    }

    fn get_potential_normal(&self) -> bool {
        // TODO: Replace with actual values once calculated.
        if self.output().is_powered() {
            true
        } else {
            false
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

    fn write(&self, state: &mut SimulatorWriter) {
        self.writer.write_alternating(self, state);
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

    #[test]
    fn writes_its_state() {
        let bus = emergency_generator();
        let mut state = SimulatorWriter::new_for_test();

        bus.write(&mut state);

        assert!(state.len_is(4));
        assert!(state.contains_f64("ELEC_EMER_GEN_POTENTIAL", 0.));
        assert!(state.contains_bool("ELEC_EMER_GEN_POTENTIAL_NORMAL", false));
        assert!(state.contains_f64("ELEC_EMER_GEN_FREQUENCY", 0.));
        assert!(state.contains_bool("ELEC_EMER_GEN_FREQUENCY_NORMAL", false));
    }

    fn emergency_generator() -> EmergencyGenerator {
        EmergencyGenerator::new()
    }
}
