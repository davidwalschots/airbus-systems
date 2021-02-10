use super::{
    Current, ElectricPowerSource, ElectricSource, ElectricalStateWriter, PowerConsumptionState,
    Powerable, ProvideFrequency, ProvidePotential,
};
use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorWriteState,
};
use uom::si::{electric_potential::volt, f64::*, frequency::hertz};

pub struct StaticInverter {
    writer: ElectricalStateWriter,
    input: Current,
}
impl StaticInverter {
    pub fn new() -> StaticInverter {
        StaticInverter {
            writer: ElectricalStateWriter::new("STAT_INV"),
            input: Current::none(),
        }
    }
}
impl Powerable for StaticInverter {
    fn set_input(&mut self, current: Current) {
        self.input = current;
    }

    fn get_input(&self) -> Current {
        self.input
    }
}
impl ElectricSource for StaticInverter {
    fn output(&self) -> Current {
        if self.input.is_powered() {
            Current::some(ElectricPowerSource::StaticInverter)
        } else {
            Current::none()
        }
    }
}
impl ProvidePotential for StaticInverter {
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
impl ProvideFrequency for StaticInverter {
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
impl SimulatorElementVisitable for StaticInverter {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for StaticInverter {
    fn write_power_consumption(&mut self, state: &PowerConsumptionState) {
        // TODO
    }

    fn write(&self, state: &mut SimulatorWriteState) {
        self.writer.write_alternating(self, state);
    }
}

#[cfg(test)]
mod static_inverter_tests {
    use super::*;

    struct Powerless {}
    impl ElectricSource for Powerless {
        fn output(&self) -> Current {
            Current::none()
        }
    }

    struct Powered {}
    impl ElectricSource for Powered {
        fn output(&self) -> Current {
            Current::some(ElectricPowerSource::ApuGenerator)
        }
    }

    #[test]
    fn starts_without_output() {
        assert!(static_inverter().is_unpowered());
    }

    #[test]
    fn when_powered_has_output() {
        let mut static_inv = static_inverter();
        static_inv.powered_by(&powered());

        assert!(static_inv.is_powered());
    }

    #[test]
    fn when_unpowered_has_no_output() {
        let mut static_inv = static_inverter();
        static_inv.powered_by(&Powerless {});

        assert!(static_inv.is_unpowered());
    }

    #[test]
    fn writes_its_state() {
        let static_inverter = static_inverter();
        let mut state = SimulatorWriteState::new();

        static_inverter.write(&mut state);

        assert!(state.len_is(4));
        assert!(state.contains_f64("ELEC_STAT_INV_POTENTIAL", 0.));
        assert!(state.contains_bool("ELEC_STAT_INV_POTENTIAL_NORMAL", false));
        assert!(state.contains_f64("ELEC_STAT_INV_FREQUENCY", 0.));
        assert!(state.contains_bool("ELEC_STAT_INV_FREQUENCY_NORMAL", false));
    }

    fn static_inverter() -> StaticInverter {
        StaticInverter::new()
    }

    fn powered() -> Powered {
        Powered {}
    }
}
