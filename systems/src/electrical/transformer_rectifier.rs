use super::{
    consumption::{PowerConsumption, PowerConsumptionReport},
    ElectricalStateWriter, Potential, PotentialSource, PotentialTarget, ProvideCurrent,
    ProvidePotential,
};
use crate::simulation::{SimulationElement, SimulatorWriter, UpdateContext};
use uom::si::{electric_current::ampere, electric_potential::volt, f64::*};

pub struct TransformerRectifier {
    writer: ElectricalStateWriter,
    number: usize,
    input: Potential,
    failed: bool,
    potential: ElectricPotential,
    current: ElectricCurrent,
}
impl TransformerRectifier {
    pub fn new(number: usize) -> TransformerRectifier {
        TransformerRectifier {
            writer: ElectricalStateWriter::new(&format!("TR_{}", number)),
            number,
            input: Potential::None,
            failed: false,
            potential: ElectricPotential::new::<volt>(0.),
            current: ElectricCurrent::new::<ampere>(0.),
        }
    }

    pub fn fail(&mut self) {
        self.failed = true;
    }

    pub fn input_potential(&self) -> Potential {
        self.input
    }
}
potential_target!(TransformerRectifier);
impl PotentialSource for TransformerRectifier {
    fn output_potential(&self) -> Potential {
        if self.failed {
            Potential::None
        } else if self.input.is_powered() {
            Potential::TransformerRectifier(self.number)
        } else {
            Potential::None
        }
    }
}
impl ProvideCurrent for TransformerRectifier {
    fn current(&self) -> ElectricCurrent {
        self.current
    }

    fn current_normal(&self) -> bool {
        self.current > ElectricCurrent::new::<ampere>(5.)
    }
}
impl ProvidePotential for TransformerRectifier {
    fn potential(&self) -> ElectricPotential {
        self.potential
    }

    fn potential_normal(&self) -> bool {
        let volts = self.potential.get::<volt>();
        (25.0..=31.0).contains(&volts)
    }
}
impl SimulationElement for TransformerRectifier {
    fn write(&self, writer: &mut SimulatorWriter) {
        self.writer.write_direct(self, writer);
    }

    fn consume_power_in_converters(&mut self, consumption: &mut PowerConsumption) {
        let dc_consumption = consumption.total_consumption_of(&self.output_potential());

        // Add the DC consumption to the TRs input (AC) consumption.
        // Currently transformer rectifier inefficiency isn't modelled.
        // It is to be expected that AC consumption should actually be somewhat
        // higher than DC consumption.
        consumption.add(&self.input, dc_consumption);
    }

    fn process_power_consumption_report<T: PowerConsumptionReport>(
        &mut self,
        report: &T,
        _: &UpdateContext,
    ) {
        self.potential = if self.output_potential().is_powered() {
            ElectricPotential::new::<volt>(28.)
        } else {
            ElectricPotential::new::<volt>(0.)
        };

        let consumption = report.total_consumption_of(&self.output_potential());
        self.current = consumption / self.potential;
    }
}

#[cfg(test)]
mod transformer_rectifier_tests {
    use uom::si::power::watt;

    use super::*;
    use crate::{
        electrical::{
            consumption::{PowerConsumer, SuppliedPower},
            ElectricalBusType,
        },
        simulation::{test::SimulationTestBed, Aircraft, SimulationElementVisitor},
    };

    struct TransformerRectifierTestBed {
        test_bed: SimulationTestBed,
    }
    impl TransformerRectifierTestBed {
        fn new() -> Self {
            Self {
                test_bed: SimulationTestBed::new(),
            }
        }

        fn run_aircraft<T: Aircraft>(&mut self, aircraft: &mut T) {
            self.test_bed.run_aircraft(aircraft);
        }

        fn current_is_normal(&mut self) -> bool {
            self.test_bed.read_bool("ELEC_TR_1_CURRENT_NORMAL")
        }

        fn potential_is_normal(&mut self) -> bool {
            self.test_bed.read_bool("ELEC_TR_1_POTENTIAL_NORMAL")
        }

        fn current(&mut self) -> ElectricCurrent {
            ElectricCurrent::new::<ampere>(self.test_bed.read_f64("ELEC_TR_1_CURRENT"))
        }
    }

    struct TestAircraft {
        transformer_rectifier: TransformerRectifier,
        consumer: PowerConsumer,
        transformer_rectifier_consumption: Power,
    }
    impl TestAircraft {
        fn new() -> Self {
            Self {
                transformer_rectifier: TransformerRectifier::new(1),
                consumer: PowerConsumer::from(ElectricalBusType::DirectCurrent(1)),
                transformer_rectifier_consumption: Power::new::<watt>(0.),
            }
        }

        fn with_powered_transformer_rectifier(mut self) -> Self {
            self.transformer_rectifier.powered_by(&Powered {});
            self
        }

        fn with_unpowered_transformer_rectifier(mut self) -> Self {
            self.transformer_rectifier.powered_by(&Powerless {});
            self
        }

        fn fail_transformer_rectifier(&mut self) {
            self.transformer_rectifier.fail();
        }

        fn transformer_rectifier_is_powered(&self) -> bool {
            self.transformer_rectifier.is_powered()
        }

        fn power_demand(&mut self, power: Power) {
            self.consumer.demand(power);
        }

        fn transformer_rectifier_consumption(&self) -> Power {
            self.transformer_rectifier_consumption
        }
    }
    impl Aircraft for TestAircraft {
        fn get_supplied_power(&mut self) -> SuppliedPower {
            let mut supplied_power = SuppliedPower::new();
            if self.transformer_rectifier.is_powered() {
                supplied_power.add(
                    ElectricalBusType::DirectCurrent(1),
                    Potential::TransformerRectifier(1),
                );
            }

            supplied_power
        }
    }
    impl SimulationElement for TestAircraft {
        fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
            self.transformer_rectifier.accept(visitor);
            self.consumer.accept(visitor);

            visitor.visit(self);
        }

        fn process_power_consumption_report<T: PowerConsumptionReport>(
            &mut self,
            report: &T,
            _: &UpdateContext,
        ) {
            self.transformer_rectifier_consumption =
                report.total_consumption_of(&Potential::TransformerRectifier(1));
        }
    }

    struct Powerless {}
    impl PotentialSource for Powerless {
        fn output_potential(&self) -> Potential {
            Potential::None
        }
    }

    struct Powered {}
    impl PotentialSource for Powered {
        fn output_potential(&self) -> Potential {
            Potential::ApuGenerator(1)
        }
    }

    #[test]
    fn when_unpowered_has_no_output() {
        let mut aircraft = TestAircraft::new().with_unpowered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!aircraft.transformer_rectifier_is_powered());
    }

    #[test]
    fn when_powered_has_output() {
        let mut aircraft = TestAircraft::new().with_powered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(aircraft.transformer_rectifier_is_powered());
    }

    #[test]
    fn when_powered_but_failed_has_no_output() {
        let mut aircraft = TestAircraft::new().with_powered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        aircraft.fail_transformer_rectifier();
        test_bed.run_aircraft(&mut aircraft);

        assert!(!aircraft.transformer_rectifier_is_powered());
    }

    #[test]
    fn when_unpowered_current_is_not_normal() {
        let mut aircraft = TestAircraft::new().with_unpowered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!test_bed.current_is_normal());
    }

    #[test]
    fn when_powered_with_too_little_demand_current_is_not_normal() {
        let mut aircraft = TestAircraft::new().with_powered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        aircraft.power_demand(Power::new::<watt>(5. * 28.));
        test_bed.run_aircraft(&mut aircraft);

        assert!(!test_bed.current_is_normal());
    }

    #[test]
    fn when_powered_with_enough_demand_current_is_normal() {
        let mut aircraft = TestAircraft::new().with_powered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        aircraft.power_demand(Power::new::<watt>((5. * 28.) + 1.));
        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.current_is_normal());
    }

    #[test]
    fn when_unpowered_potential_is_not_normal() {
        let mut aircraft = TestAircraft::new().with_unpowered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!test_bed.potential_is_normal());
    }

    #[test]
    fn when_powered_potential_is_normal() {
        let mut aircraft = TestAircraft::new().with_powered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.potential_is_normal());
    }

    #[test]
    fn when_unpowered_has_no_consumption() {
        let mut aircraft = TestAircraft::new().with_unpowered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(
            aircraft.transformer_rectifier_consumption(),
            Power::new::<watt>(0.)
        );
    }

    #[test]
    fn when_powered_without_demand_has_no_consumption() {
        let mut aircraft = TestAircraft::new().with_powered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        aircraft.power_demand(Power::new::<watt>(0.));
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(
            aircraft.transformer_rectifier_consumption(),
            Power::new::<watt>(0.)
        );
    }

    #[test]
    fn when_powered_with_demand_has_consumption() {
        let mut aircraft = TestAircraft::new().with_powered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        aircraft.power_demand(Power::new::<watt>(200.));
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(
            aircraft.transformer_rectifier_consumption(),
            Power::new::<watt>(200.)
        );
    }

    #[test]
    fn when_powered_with_demand_current_is_based_on_demand() {
        let mut aircraft = TestAircraft::new().with_powered_transformer_rectifier();
        let mut test_bed = TransformerRectifierTestBed::new();

        aircraft.power_demand(Power::new::<watt>(200.));
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(
            test_bed.current(),
            ElectricCurrent::new::<ampere>(200. / 28.)
        );
    }

    #[test]
    fn writes_its_state() {
        let mut aircraft = TestAircraft::new().with_powered_transformer_rectifier();
        let mut test_bed = SimulationTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.contains_key("ELEC_TR_1_CURRENT"));
        assert!(test_bed.contains_key("ELEC_TR_1_CURRENT_NORMAL"));
        assert!(test_bed.contains_key("ELEC_TR_1_POTENTIAL"));
        assert!(test_bed.contains_key("ELEC_TR_1_POTENTIAL_NORMAL"));
    }
}
