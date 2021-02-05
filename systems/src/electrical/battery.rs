use super::{Current, ElectricPowerSource, ElectricSource, PowerConsumptionState, Powerable};
use crate::simulator::{SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor};
use uom::si::{electric_charge::ampere_hour, f64::*};

pub struct Battery {
    number: u8,
    input: Current,
    charge: ElectricCharge,
}
impl Battery {
    const MAX_ELECTRIC_CHARGE_AMPERE_HOURS: f64 = 23.0;

    pub fn full(number: u8) -> Battery {
        Battery::new(
            number,
            ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS),
        )
    }

    #[cfg(test)]
    pub fn empty(number: u8) -> Battery {
        Battery::new(number, ElectricCharge::new::<ampere_hour>(0.))
    }

    fn new(number: u8, charge: ElectricCharge) -> Battery {
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
