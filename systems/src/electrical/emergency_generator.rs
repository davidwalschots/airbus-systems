use super::{
    consumption::PowerConsumptionReport, ElectricalStateWriter, Potential, PotentialSource,
    ProvideFrequency, ProvidePotential,
};
use crate::simulation::{SimulationElement, SimulatorWriter, UpdateContext};
use uom::si::{electric_potential::volt, f64::*, frequency::hertz};

pub struct EmergencyGenerator {
    writer: ElectricalStateWriter,
    running: bool,
    is_blue_pressurised: bool,
    frequency: Frequency,
    potential: ElectricPotential,
}
impl EmergencyGenerator {
    pub fn new() -> EmergencyGenerator {
        EmergencyGenerator {
            writer: ElectricalStateWriter::new("EMER_GEN"),
            running: false,
            is_blue_pressurised: false,
            frequency: Frequency::new::<hertz>(0.),
            potential: ElectricPotential::new::<volt>(0.),
        }
    }

    pub fn update(&mut self, is_blue_pressurised: bool) {
        // TODO: The emergency generator is driven by the blue hydraulic circuit. Still to be implemented.
        self.is_blue_pressurised = is_blue_pressurised;
    }

    pub fn attempt_start(&mut self) {
        self.running = true;
    }

    pub fn is_running(&self) -> bool {
        self.is_blue_pressurised && self.running
    }
}
impl PotentialSource for EmergencyGenerator {
    fn output_potential(&self) -> Potential {
        if self.is_running() {
            Potential::EmergencyGenerator
        } else {
            Potential::None
        }
    }
}
provide_frequency!(EmergencyGenerator, (390.0..=410.0));
provide_potential!(EmergencyGenerator, (110.0..=120.0));
impl SimulationElement for EmergencyGenerator {
    fn process_power_consumption_report<T: PowerConsumptionReport>(
        &mut self,
        _report: &T,
        _context: &UpdateContext,
    ) {
        self.frequency = if self.output_potential().is_powered() {
            Frequency::new::<hertz>(400.)
        } else {
            Frequency::new::<hertz>(0.)
        };

        self.potential = if self.output_potential().is_powered() {
            ElectricPotential::new::<volt>(115.)
        } else {
            ElectricPotential::new::<volt>(0.)
        };
    }

    fn write(&self, writer: &mut SimulatorWriter) {
        self.writer.write_alternating(self, writer);
    }
}
impl Default for EmergencyGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod emergency_generator_tests {
    use super::*;
    use crate::simulation::{test::SimulationTestBed, Aircraft, SimulationElementVisitor};

    struct EmergencyGeneratorTestBed {
        test_bed: SimulationTestBed,
    }
    impl EmergencyGeneratorTestBed {
        fn new() -> Self {
            Self {
                test_bed: SimulationTestBed::new(),
            }
        }

        fn run_aircraft<T: Aircraft>(&mut self, aircraft: &mut T) {
            self.test_bed.run_aircraft(aircraft);
        }

        fn frequency_is_normal(&mut self) -> bool {
            self.test_bed.read_bool("ELEC_EMER_GEN_FREQUENCY_NORMAL")
        }

        fn potential_is_normal(&mut self) -> bool {
            self.test_bed.read_bool("ELEC_EMER_GEN_POTENTIAL_NORMAL")
        }
    }

    struct TestAircraft {
        emer_gen: EmergencyGenerator,
        is_blue_pressurised: bool,
    }
    impl TestAircraft {
        fn new() -> Self {
            Self {
                emer_gen: EmergencyGenerator::new(),
                is_blue_pressurised: true,
            }
        }

        fn emer_gen_is_powered(&self) -> bool {
            self.emer_gen.is_powered()
        }

        fn attempt_emer_gen_start(&mut self) {
            self.emer_gen.attempt_start();
        }

        fn set_blue_pressurisation(&mut self, pressurised: bool) {
            self.is_blue_pressurised = pressurised;
        }
    }
    impl Aircraft for TestAircraft {
        fn update_before_power_distribution(&mut self, _: &UpdateContext) {
            self.emer_gen.update(self.is_blue_pressurised);
        }
    }
    impl SimulationElement for TestAircraft {
        fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
            self.emer_gen.accept(visitor);

            visitor.visit(self);
        }
    }

    #[test]
    fn when_shutdown_has_no_output() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = EmergencyGeneratorTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!aircraft.emer_gen_is_powered());
    }

    #[test]
    fn when_started_provides_output() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = EmergencyGeneratorTestBed::new();

        aircraft.attempt_emer_gen_start();
        test_bed.run_aircraft(&mut aircraft);

        assert!(aircraft.emer_gen_is_powered());
    }

    #[test]
    fn when_started_without_hydraulic_pressure_is_unpowered() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = EmergencyGeneratorTestBed::new();

        aircraft.attempt_emer_gen_start();
        aircraft.set_blue_pressurisation(false);
        test_bed.run_aircraft(&mut aircraft);

        assert!(!aircraft.emer_gen_is_powered());
    }

    #[test]
    fn when_shutdown_frequency_not_normal() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = EmergencyGeneratorTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!test_bed.frequency_is_normal());
    }

    #[test]
    fn when_started_frequency_normal() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = EmergencyGeneratorTestBed::new();

        aircraft.attempt_emer_gen_start();
        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.frequency_is_normal());
    }

    #[test]
    fn when_shutdown_potential_not_normal() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = EmergencyGeneratorTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!test_bed.potential_is_normal());
    }

    #[test]
    fn when_started_potential_normal() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = EmergencyGeneratorTestBed::new();

        aircraft.attempt_emer_gen_start();
        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.potential_is_normal());
    }

    #[test]
    fn writes_its_state() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = SimulationTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.contains_key("ELEC_EMER_GEN_POTENTIAL"));
        assert!(test_bed.contains_key("ELEC_EMER_GEN_POTENTIAL_NORMAL"));
        assert!(test_bed.contains_key("ELEC_EMER_GEN_FREQUENCY"));
        assert!(test_bed.contains_key("ELEC_EMER_GEN_FREQUENCY_NORMAL"));
    }
}
