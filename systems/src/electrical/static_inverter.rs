use super::{Current, ElectricPowerSource, ElectricSource, PowerConsumptionState, Powerable};
use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorWriteState,
};
use uom::si::{electric_potential::volt, f64::*, frequency::hertz};

pub struct StaticInverter {
    input: Current,
}
impl StaticInverter {
    pub fn new() -> StaticInverter {
        StaticInverter {
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
        // TODO: Replace with actual values once calculated.
        state.electrical.static_inverter.frequency = if self.output().is_powered() {
            Frequency::new::<hertz>(400.)
        } else {
            Frequency::new::<hertz>(0.)
        };
        state
            .electrical
            .static_inverter
            .frequency_within_normal_range = if self.output().is_powered() {
            true
        } else {
            false
        };
        state.electrical.static_inverter.potential = if self.output().is_powered() {
            ElectricPotential::new::<volt>(115.)
        } else {
            ElectricPotential::new::<volt>(0.)
        };
        state
            .electrical
            .static_inverter
            .potential_within_normal_range = if self.output().is_powered() {
            true
        } else {
            false
        };
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

    fn static_inverter() -> StaticInverter {
        StaticInverter::new()
    }

    fn powered() -> Powered {
        Powered {}
    }
}
