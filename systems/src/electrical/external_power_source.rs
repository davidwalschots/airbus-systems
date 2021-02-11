use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorReader,
    SimulatorWriter, UpdateContext,
};
use uom::si::{electric_potential::volt, f64::*, frequency::hertz};

use super::{
    Current, ElectricPowerSource, ElectricSource, ElectricalStateWriter, ProvideFrequency,
    ProvidePotential,
};

pub struct ExternalPowerSource {
    writer: ElectricalStateWriter,
    pub is_connected: bool,
}
impl ExternalPowerSource {
    pub fn new() -> ExternalPowerSource {
        ExternalPowerSource {
            writer: ElectricalStateWriter::new("EXT_PWR"),
            is_connected: false,
        }
    }

    pub fn update(&mut self, _: &UpdateContext) {}
}
impl ElectricSource for ExternalPowerSource {
    fn output(&self) -> Current {
        if self.is_connected {
            Current::some(ElectricPowerSource::External)
        } else {
            Current::none()
        }
    }
}
impl ProvidePotential for ExternalPowerSource {
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
impl ProvideFrequency for ExternalPowerSource {
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
impl SimulatorElementVisitable for ExternalPowerSource {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for ExternalPowerSource {
    fn read(&mut self, state: &mut SimulatorReader) {
        self.is_connected = state.get_bool("EXT_PWR_IS_AVAILABLE");
    }

    fn write(&self, state: &mut SimulatorWriter) {
        self.writer.write_alternating(self, state);
    }
}

#[cfg(test)]
mod external_power_source_tests {
    use super::*;

    #[test]
    fn starts_without_output() {
        assert!(external_power_source().is_unpowered());
    }

    #[test]
    fn when_plugged_in_provides_output() {
        let mut ext_pwr = external_power_source();
        ext_pwr.is_connected = true;

        assert!(ext_pwr.is_powered());
    }

    #[test]
    fn when_not_plugged_in_provides_no_output() {
        let mut ext_pwr = external_power_source();
        ext_pwr.is_connected = false;

        assert!(ext_pwr.is_unpowered());
    }

    #[test]
    fn writes_its_state() {
        let external_power = external_power_source();
        let mut state = SimulatorWriter::new_for_test();

        external_power.write(&mut state);

        assert!(state.len_is(4));
        assert!(state.contains_f64("ELEC_EXT_PWR_POTENTIAL", 0.));
        assert!(state.contains_bool("ELEC_EXT_PWR_POTENTIAL_NORMAL", false));
        assert!(state.contains_f64("ELEC_EXT_PWR_FREQUENCY", 0.));
        assert!(state.contains_bool("ELEC_EXT_PWR_FREQUENCY_NORMAL", false));
    }

    fn external_power_source() -> ExternalPowerSource {
        ExternalPowerSource::new()
    }
}
