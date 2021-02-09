use super::{Current, ElectricPowerSource, ElectricSource, PowerConsumptionState, Powerable};
use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorWriteState,
};
use uom::si::{
    electric_charge::ampere_hour, electric_current::ampere, electric_potential::volt, f64::*,
};

pub struct Battery {
    number: usize,
    input: Current,
    charge: ElectricCharge,
}
impl Battery {
    const MAX_ELECTRIC_CHARGE_AMPERE_HOURS: f64 = 23.0;

    pub fn full(number: usize) -> Battery {
        Battery::new(
            number,
            ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS),
        )
    }

    #[cfg(test)]
    pub fn empty(number: usize) -> Battery {
        Battery::new(number, ElectricCharge::new::<ampere_hour>(0.))
    }

    fn new(number: usize, charge: ElectricCharge) -> Battery {
        Battery {
            number,
            input: Current::none(),
            charge,
        }
    }

    pub fn is_full(&self) -> bool {
        self.charge >= ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS)
    }
}
impl Powerable for Battery {
    fn set_input(&mut self, current: Current) {
        self.input = current
    }

    fn get_input(&self) -> Current {
        self.input
    }
}
impl ElectricSource for Battery {
    fn output(&self) -> Current {
        if self.input.is_unpowered() && self.charge > ElectricCharge::new::<ampere_hour>(0.) {
            Current::some(ElectricPowerSource::Battery(self.number))
        } else {
            Current::none()
        }
    }
}
impl SimulatorElementVisitable for Battery {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for Battery {
    fn write_power_consumption(&mut self, state: &PowerConsumptionState) {
        // TODO: Charging and depleting battery when used.
    }

    fn write(&self, state: &mut SimulatorWriteState) {
        // TODO: Replace with actual values once calculated.
        state.electrical.batteries[self.number - 1].current = if self.output().is_powered() {
            ElectricCurrent::new::<ampere>(150.)
        } else {
            ElectricCurrent::new::<ampere>(0.)
        };
        state.electrical.batteries[self.number - 1].current_within_normal_range =
            if self.output().is_powered() {
                true
            } else {
                false
            };
        state.electrical.batteries[self.number - 1].potential = if self.output().is_powered() {
            ElectricPotential::new::<volt>(28.)
        } else {
            ElectricPotential::new::<volt>(0.)
        };
        state.electrical.batteries[self.number - 1].potential_within_normal_range =
            if self.output().is_powered() {
                true
            } else {
                false
            };
    }
}

#[cfg(test)]
mod battery_tests {
    use super::*;

    struct Powerless {}
    impl ElectricSource for Powerless {
        fn output(&self) -> Current {
            Current::none()
        }
    }

    struct StubApuGenerator {}
    impl ElectricSource for StubApuGenerator {
        fn output(&self) -> Current {
            Current::some(ElectricPowerSource::ApuGenerator)
        }
    }

    fn apu_generator() -> StubApuGenerator {
        StubApuGenerator {}
    }

    #[test]
    fn full_battery_has_output() {
        assert!(full_battery().is_full());
        assert!(full_battery().is_powered());
    }

    #[test]
    fn empty_battery_has_no_output() {
        assert!(!empty_battery().is_full());
        assert!(empty_battery().is_unpowered());
    }

    #[test]
    fn when_empty_battery_has_input_doesnt_have_output() {
        let mut battery = empty_battery();
        battery.powered_by(&apu_generator());

        assert!(battery.is_unpowered());
    }

    #[test]
    fn when_full_battery_has_doesnt_have_output() {
        // Of course battery input at this stage would result in overcharging. However, for the sake of the test we ignore it.
        let mut battery = full_battery();
        battery.powered_by(&apu_generator());

        assert!(battery.is_unpowered());
    }

    #[test]
    fn charged_battery_without_input_has_output() {
        let mut battery = full_battery();
        battery.powered_by(&Powerless {});

        assert!(battery.is_powered());
    }

    #[test]
    fn empty_battery_without_input_has_no_output() {
        let mut battery = empty_battery();
        battery.powered_by(&Powerless {});

        assert!(battery.is_unpowered());
    }

    fn full_battery() -> Battery {
        Battery::full(1)
    }

    fn empty_battery() -> Battery {
        Battery::empty(1)
    }
}
