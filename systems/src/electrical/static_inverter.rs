use super::{
    consumption::{PowerConsumption, PowerConsumptionReport},
    ElectricalStateWriter, Potential, PotentialSource, PotentialTarget, ProvideFrequency,
    ProvidePotential,
};
use crate::simulation::{SimulationElement, SimulatorWriter, UpdateContext};
use uom::si::{electric_potential::volt, f64::*, frequency::hertz};

pub struct StaticInverter {
    writer: ElectricalStateWriter,
    input: Potential,
    potential: ElectricPotential,
    frequency: Frequency,
}
impl StaticInverter {
    pub fn new() -> StaticInverter {
        StaticInverter {
            writer: ElectricalStateWriter::new("STAT_INV"),
            input: Potential::None,
            potential: ElectricPotential::new::<volt>(0.),
            frequency: Frequency::new::<hertz>(0.),
        }
    }

    pub fn input_potential(&self) -> Potential {
        self.input
    }
}
potential_target!(StaticInverter);
impl PotentialSource for StaticInverter {
    fn output_potential(&self) -> Potential {
        if self.input.is_powered() {
            Potential::StaticInverter
        } else {
            Potential::None
        }
    }
}
provide_potential!(StaticInverter, (110.0..=120.0));
provide_frequency!(StaticInverter, (390.0..=410.0));
impl SimulationElement for StaticInverter {
    fn write(&self, writer: &mut SimulatorWriter) {
        self.writer.write_alternating(self, writer);
    }

    fn consume_power_in_converters(&mut self, consumption: &mut PowerConsumption) {
        let ac_consumption = consumption.total_consumption_of(&self.output_potential());

        // Add the AC consumption to the STAT INVs input (DC) consumption.
        // Currently static inverter inefficiency isn't modelled.
        // It is to be expected that DC consumption should actually be somewhat
        // higher than AC consumption.
        consumption.add(&self.input, ac_consumption);
    }

    fn process_power_consumption_report<T: PowerConsumptionReport>(
        &mut self,
        _: &T,
        _: &UpdateContext,
    ) {
        self.potential = if self.output_potential().is_powered() {
            ElectricPotential::new::<volt>(115.)
        } else {
            ElectricPotential::new::<volt>(0.)
        };

        self.frequency = if self.output_potential().is_powered() {
            Frequency::new::<hertz>(400.)
        } else {
            Frequency::new::<hertz>(0.)
        };
    }
}
impl Default for StaticInverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod static_inverter_tests {
    use uom::si::power::watt;

    use super::*;
    use crate::{
        electrical::{
            consumption::{PowerConsumer, SuppliedPower},
            ElectricalBusType,
        },
        simulation::{test::SimulationTestBed, Aircraft, SimulationElementVisitor},
    };

    struct Powerless {}
    impl PotentialSource for Powerless {
        fn output_potential(&self) -> Potential {
            Potential::None
        }
    }

    struct Powered {}
    impl PotentialSource for Powered {
        fn output_potential(&self) -> Potential {
            Potential::Battery(1)
        }
    }

    struct StaticInverterTestBed {
        test_bed: SimulationTestBed,
    }
    impl StaticInverterTestBed {
        fn new() -> Self {
            Self {
                test_bed: SimulationTestBed::new(),
            }
        }

        fn run_aircraft<T: Aircraft>(&mut self, aircraft: &mut T) {
            self.test_bed.run_aircraft(aircraft);
        }

        fn frequency_is_normal(&mut self) -> bool {
            self.test_bed.read_bool("ELEC_STAT_INV_FREQUENCY_NORMAL")
        }

        fn potential_is_normal(&mut self) -> bool {
            self.test_bed.read_bool("ELEC_STAT_INV_POTENTIAL_NORMAL")
        }
    }

    struct TestAircraft {
        static_inverter: StaticInverter,
        consumer: PowerConsumer,
        static_inverter_consumption: Power,
    }
    impl TestAircraft {
        fn new() -> Self {
            Self {
                static_inverter: StaticInverter::new(),
                consumer: PowerConsumer::from(ElectricalBusType::AlternatingCurrentEssential),
                static_inverter_consumption: Power::new::<watt>(0.),
            }
        }

        fn with_powered_static_inverter(mut self) -> Self {
            self.static_inverter.powered_by(&Powered {});
            self
        }

        fn with_unpowered_static_inverter(mut self) -> Self {
            self.static_inverter.powered_by(&Powerless {});
            self
        }

        fn static_inverter_is_powered(&self) -> bool {
            self.static_inverter.is_powered()
        }

        fn power_demand(&mut self, power: Power) {
            self.consumer.demand(power);
        }

        fn static_inverter_consumption(&self) -> Power {
            self.static_inverter_consumption
        }
    }
    impl Aircraft for TestAircraft {
        fn get_supplied_power(&mut self) -> SuppliedPower {
            let mut supplied_power = SuppliedPower::new();
            if self.static_inverter.is_powered() {
                supplied_power.add(
                    ElectricalBusType::AlternatingCurrentEssential,
                    Potential::StaticInverter,
                );
            }

            supplied_power
        }
    }
    impl SimulationElement for TestAircraft {
        fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
            self.static_inverter.accept(visitor);
            self.consumer.accept(visitor);

            visitor.visit(self);
        }

        fn process_power_consumption_report<T: PowerConsumptionReport>(
            &mut self,
            report: &T,
            _: &UpdateContext,
        ) {
            self.static_inverter_consumption =
                report.total_consumption_of(&Potential::StaticInverter);
        }
    }

    #[test]
    fn when_unpowered_has_no_output() {
        let mut aircraft = TestAircraft::new().with_unpowered_static_inverter();
        let mut test_bed = StaticInverterTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!aircraft.static_inverter_is_powered());
    }

    #[test]
    fn when_powered_has_output() {
        let mut aircraft = TestAircraft::new().with_powered_static_inverter();
        let mut test_bed = StaticInverterTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(aircraft.static_inverter_is_powered());
    }

    #[test]
    fn when_unpowered_frequency_is_not_normal() {
        let mut aircraft = TestAircraft::new().with_unpowered_static_inverter();
        let mut test_bed = StaticInverterTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!test_bed.frequency_is_normal());
    }

    #[test]
    fn when_powered_frequency_is_normal() {
        let mut aircraft = TestAircraft::new().with_powered_static_inverter();
        let mut test_bed = StaticInverterTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.frequency_is_normal());
    }

    #[test]
    fn when_unpowered_potential_is_not_normal() {
        let mut aircraft = TestAircraft::new().with_unpowered_static_inverter();
        let mut test_bed = StaticInverterTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!test_bed.potential_is_normal());
    }

    #[test]
    fn when_powered_potential_is_normal() {
        let mut aircraft = TestAircraft::new().with_powered_static_inverter();
        let mut test_bed = StaticInverterTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.potential_is_normal());
    }

    #[test]
    fn when_unpowered_has_no_consumption() {
        let mut aircraft = TestAircraft::new().with_unpowered_static_inverter();
        let mut test_bed = StaticInverterTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(
            aircraft.static_inverter_consumption(),
            Power::new::<watt>(0.)
        );
    }

    #[test]
    fn when_powered_without_demand_has_no_consumption() {
        let mut aircraft = TestAircraft::new().with_powered_static_inverter();
        let mut test_bed = StaticInverterTestBed::new();

        aircraft.power_demand(Power::new::<watt>(0.));
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(
            aircraft.static_inverter_consumption(),
            Power::new::<watt>(0.)
        );
    }

    #[test]
    fn when_powered_with_demand_has_consumption() {
        let mut aircraft = TestAircraft::new().with_powered_static_inverter();
        let mut test_bed = StaticInverterTestBed::new();

        aircraft.power_demand(Power::new::<watt>(200.));
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(
            aircraft.static_inverter_consumption(),
            Power::new::<watt>(200.)
        );
    }

    #[test]
    fn writes_its_state() {
        let mut aircraft = TestAircraft::new();
        let mut test_bed = SimulationTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.contains_key("ELEC_STAT_INV_POTENTIAL"));
        assert!(test_bed.contains_key("ELEC_STAT_INV_POTENTIAL_NORMAL"));
        assert!(test_bed.contains_key("ELEC_STAT_INV_FREQUENCY"));
        assert!(test_bed.contains_key("ELEC_STAT_INV_FREQUENCY_NORMAL"));
    }
}
