use crate::simulation::{SimulationElement, SimulatorReader, SimulatorWriter, UpdateContext};
use uom::si::{electric_potential::volt, f64::*, frequency::hertz};

use super::{
    ElectricalStateWriter, Potential, PotentialOrigin, PotentialSource, ProvideFrequency,
    ProvidePotential,
};

pub struct ExternalPowerSource {
    writer: ElectricalStateWriter,
    is_connected: bool,
    output_frequency: Frequency,
    output_potential: ElectricPotential,
}
impl ExternalPowerSource {
    pub fn new() -> ExternalPowerSource {
        ExternalPowerSource {
            writer: ElectricalStateWriter::new("EXT_PWR"),
            is_connected: false,
            output_frequency: Frequency::new::<hertz>(0.),
            output_potential: ElectricPotential::new::<volt>(0.),
        }
    }

    pub fn update(&mut self, _: &UpdateContext) {}

    /// Indicates if the provided electricity's potential and frequency
    /// are within normal parameters. Use this to decide if the
    /// external power contactor should close.
    pub fn output_within_normal_parameters(&self) -> bool {
        self.potential_normal() && self.frequency_normal()
    }

    fn should_provide_output(&self) -> bool {
        self.is_connected
    }
}
impl PotentialSource for ExternalPowerSource {
    fn output(&self) -> Potential {
        if self.should_provide_output() {
            Potential::single(PotentialOrigin::External, self.output_potential)
        } else {
            Potential::none()
        }
    }
}
provide_potential!(ExternalPowerSource, (110.0..=120.0));
provide_frequency!(ExternalPowerSource, (390.0..=410.0));
impl SimulationElement for ExternalPowerSource {
    fn read(&mut self, reader: &mut SimulatorReader) {
        self.is_connected = reader.read_bool("EXTERNAL POWER AVAILABLE:1");
    }

    fn write(&self, writer: &mut SimulatorWriter) {
        self.writer.write_alternating(self, writer);
    }

    fn process_power_consumption_report<T: super::consumption::PowerConsumptionReport>(
        &mut self,
        _: &T,
    ) {
        self.output_frequency = if self.should_provide_output() {
            Frequency::new::<hertz>(400.)
        } else {
            Frequency::new::<hertz>(0.)
        };

        self.output_potential = if self.should_provide_output() {
            ElectricPotential::new::<volt>(115.)
        } else {
            ElectricPotential::new::<volt>(0.)
        };
    }
}
impl Default for ExternalPowerSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod external_power_source_tests {
    use super::*;
    use crate::simulation::{test::SimulationTestBed, Aircraft, SimulationElementVisitor};

    struct ExternalPowerTestBed {
        test_bed: SimulationTestBed,
    }
    impl ExternalPowerTestBed {
        fn new() -> Self {
            Self {
                test_bed: SimulationTestBed::new(),
            }
        }

        fn with_disconnected_external_power(mut self) -> Self {
            self.test_bed
                .write_bool("EXTERNAL POWER AVAILABLE:1", false);
            self
        }

        fn with_connected_external_power(mut self) -> Self {
            self.test_bed.write_bool("EXTERNAL POWER AVAILABLE:1", true);
            self
        }

        fn run_aircraft<T: Aircraft>(&mut self, aircraft: &mut T) {
            self.test_bed.run_aircraft(aircraft);
        }

        fn frequency_is_normal(&mut self) -> bool {
            self.test_bed.read_bool("ELEC_EXT_PWR_FREQUENCY_NORMAL")
        }

        fn potential_is_normal(&mut self) -> bool {
            self.test_bed.read_bool("ELEC_EXT_PWR_POTENTIAL_NORMAL")
        }
    }

    struct TestAircraft {
        ext_pwr: ExternalPowerSource,
    }
    impl TestAircraft {
        fn new() -> Self {
            Self {
                ext_pwr: ExternalPowerSource::new(),
            }
        }

        fn ext_pwr_is_powered(&self) -> bool {
            self.ext_pwr.is_powered()
        }

        fn ext_pwr_output_within_normal_parameters(&self) -> bool {
            self.ext_pwr.output_within_normal_parameters()
        }
    }
    impl Aircraft for TestAircraft {}
    impl SimulationElement for TestAircraft {
        fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
            self.ext_pwr.accept(visitor);
            visitor.visit(self);
        }
    }

    #[test]
    fn when_disconnected_provides_no_output() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = ExternalPowerTestBed::new().with_disconnected_external_power();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!aircraft.ext_pwr_is_powered());
    }

    #[test]
    fn when_connected_provides_output() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = ExternalPowerTestBed::new().with_connected_external_power();

        test_bed.run_aircraft(&mut aircraft);

        assert!(aircraft.ext_pwr_is_powered());
    }

    #[test]
    fn when_disconnected_frequency_not_normal() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = ExternalPowerTestBed::new().with_disconnected_external_power();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!test_bed.frequency_is_normal());
    }

    #[test]
    fn when_connected_frequency_normal() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = ExternalPowerTestBed::new().with_connected_external_power();

        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.frequency_is_normal());
    }

    #[test]
    fn when_disconnected_potential_not_normal() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = ExternalPowerTestBed::new().with_disconnected_external_power();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!test_bed.potential_is_normal());
    }

    #[test]
    fn when_connected_potential_normal() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = ExternalPowerTestBed::new().with_connected_external_power();

        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.potential_is_normal());
    }

    #[test]
    fn output_not_within_normal_parameters_when_disconnected() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = ExternalPowerTestBed::new().with_disconnected_external_power();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!aircraft.ext_pwr_output_within_normal_parameters());
    }

    #[test]
    fn output_within_normal_parameters_when_connected() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = ExternalPowerTestBed::new().with_connected_external_power();

        test_bed.run_aircraft(&mut aircraft);

        assert!(aircraft.ext_pwr_output_within_normal_parameters());
    }

    #[test]
    fn writes_its_state() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = SimulationTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.contains_key("ELEC_EXT_PWR_POTENTIAL"));
        assert!(test_bed.contains_key("ELEC_EXT_PWR_POTENTIAL_NORMAL"));
        assert!(test_bed.contains_key("ELEC_EXT_PWR_FREQUENCY"));
        assert!(test_bed.contains_key("ELEC_EXT_PWR_FREQUENCY_NORMAL"));
    }
}
