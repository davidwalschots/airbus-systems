use super::{
    consumption::PowerConsumptionReport, ElectricalStateWriter, Potential, PotentialSource,
    PotentialTarget, ProvideCurrent, ProvidePotential,
};
use crate::simulation::{SimulationElement, SimulatorWriter, UpdateContext};
use uom::si::{
    electric_charge::ampere_hour, electric_current::ampere, electric_potential::volt, f64::*,
};

pub struct Battery {
    writer: ElectricalStateWriter,
    number: usize,
    input: Potential,
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

    pub fn empty(number: usize) -> Battery {
        Battery::new(number, ElectricCharge::new::<ampere_hour>(0.))
    }

    fn new(number: usize, charge: ElectricCharge) -> Self {
        Self {
            writer: ElectricalStateWriter::new(&format!("BAT_{}", number)),
            number,
            input: Potential::None,
            charge,
        }
    }

    pub fn is_full(&self) -> bool {
        self.charge >= ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS)
    }

    pub fn input_potential(&self) -> Potential {
        self.input
    }
}
potential_target!(Battery);
impl PotentialSource for Battery {
    fn output_potential(&self) -> Potential {
        if self.input.is_unpowered() && self.charge > ElectricCharge::new::<ampere_hour>(0.) {
            Potential::Battery(self.number)
        } else {
            Potential::None
        }
    }
}
impl ProvideCurrent for Battery {
    fn current(&self) -> ElectricCurrent {
        // TODO: Replace with actual values once calculated.
        ElectricCurrent::new::<ampere>(0.)
    }

    fn current_normal(&self) -> bool {
        // TODO: Replace with actual values once calculated.
        self.output_potential().is_powered()
    }
}
impl ProvidePotential for Battery {
    fn potential(&self) -> ElectricPotential {
        // TODO: Replace with actual values once calculated.
        if self.output_potential().is_powered() {
            ElectricPotential::new::<volt>(28.)
        } else {
            ElectricPotential::new::<volt>(0.)
        }
    }

    fn potential_normal(&self) -> bool {
        // TODO: Replace with actual values once calculated.
        self.output_potential().is_powered()
    }
}
impl SimulationElement for Battery {
    fn process_power_consumption_report<T: PowerConsumptionReport>(
        &mut self,
        _report: &T,
        _context: &UpdateContext,
    ) {
        // TODO charging and depleting battery.
    }

    fn write(&self, writer: &mut SimulatorWriter) {
        self.writer.write_direct(self, writer);
    }
}

#[cfg(test)]
mod battery_tests {
    use super::*;
    use crate::simulation::test::SimulationTestBed;

    struct Powerless {}
    impl PotentialSource for Powerless {
        fn output_potential(&self) -> Potential {
            Potential::None
        }
    }

    struct StubApuGenerator {}
    impl PotentialSource for StubApuGenerator {
        fn output_potential(&self) -> Potential {
            Potential::ApuGenerator(1)
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

    #[test]
    fn writes_its_state() {
        let mut battery = full_battery();
        let mut test_bed = SimulationTestBed::new();
        test_bed.run_without_update(&mut battery);

        assert!(test_bed.contains_key("ELEC_BAT_1_CURRENT"));
        assert!(test_bed.contains_key("ELEC_BAT_1_CURRENT_NORMAL"));
        assert!(test_bed.contains_key("ELEC_BAT_1_POTENTIAL"));
        assert!(test_bed.contains_key("ELEC_BAT_1_POTENTIAL_NORMAL"));
    }

    fn full_battery() -> Battery {
        Battery::full(1)
    }

    fn empty_battery() -> Battery {
        Battery::empty(1)
    }
}
