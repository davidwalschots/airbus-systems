use super::{
    consumption::{PowerConsumption, PowerConsumptionReport},
    ElectricalStateWriter, Potential, PotentialSource, PotentialTarget, ProvideCurrent,
    ProvidePotential,
};
use crate::simulation::{SimulationElement, SimulatorWriter};
use uom::si::{
    electric_charge::ampere_hour, electric_current::ampere, electric_potential::volt, f64::*,
    power::watt, time::second,
};

enum ElectricCurrentDirection {
    Charging,
    Discharging,
}

pub struct Battery {
    number: usize,
    writer: ElectricalStateWriter,
    input: Potential,
    charge: ElectricCharge,
    potential: ElectricPotential,
    current: ElectricCurrent,
    current_direction: Option<ElectricCurrentDirection>,
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
            number,
            writer: ElectricalStateWriter::new(&format!("BAT_{}", number)),
            input: Potential::none(),
            charge,
            potential: ElectricPotential::new::<volt>(0.),
            current: ElectricCurrent::new::<ampere>(0.),
            current_direction: None,
        }
    }

    pub fn needs_charging(&self) -> bool {
        // TODO: Get info from komp; from which Ah should we start charging?
        self.charge >= ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS)
    }

    fn has_charge(&self) -> bool {
        self.charge > ElectricCharge::new::<ampere_hour>(0.)
    }

    fn is_charging(&self) -> bool {
        self.input.is_powered()
    }

    pub fn input_potential(&self) -> Potential {
        self.input
    }

    #[cfg(test)]
    fn charge(&self) -> ElectricCharge {
        self.charge
    }

    fn should_provide_output(&self) -> bool {
        !self.is_charging() && self.has_charge()
    }
}
potential_target!(Battery);
impl PotentialSource for Battery {
    fn output(&self) -> Potential {
        if self.should_provide_output() {
            Potential::battery(self.number).with_raw(self.potential)
        } else {
            Potential::none()
        }
    }
}
impl ProvideCurrent for Battery {
    fn current(&self) -> ElectricCurrent {
        self.current
    }

    fn current_normal(&self) -> bool {
        let current = self.current.get::<ampere>();
        self.is_charging() || (0.0..=5.0).contains(&current)
    }
}
provide_potential!(Battery, (25.0..=31.0));
impl SimulationElement for Battery {
    fn write(&self, writer: &mut SimulatorWriter) {
        self.writer.write_direct(self, writer);
    }

    fn consume_power(&mut self, consumption: &mut PowerConsumption) {
        if self.is_charging() {
            consumption.add(&self.input, self.potential * self.current);
        }
    }

    fn process_power_consumption_report<T: PowerConsumptionReport>(&mut self, report: &T) {
        self.potential = if self.has_charge() {
            ElectricPotential::new::<volt>(28.)
        } else {
            ElectricPotential::new::<volt>(0.)
        };

        let time = Time::new::<second>(report.delta().as_secs_f64());
        if self.should_provide_output() {
            let consumption = report.total_consumption_of(&self.output());
            self.current = consumption / self.potential;

            if consumption > Power::new::<watt>(0.) {
                self.current_direction = Some(ElectricCurrentDirection::Discharging);
                self.charge -= (consumption * time) / self.potential;
            }
        } else if self.is_charging() {
            self.current = ElectricCurrent::new::<ampere>(9.); // TODO Should be replaced with a function that takes into account battery internals.
            self.current_direction = Some(ElectricCurrentDirection::Charging);

            let time = Time::new::<second>(report.delta().as_secs_f64());
            let incoming_potential = ElectricPotential::new::<volt>(28.); // TODO Replace with actual potential coming from origin.

            self.charge += ((incoming_potential * self.current) * time) / incoming_potential;
        } else {
            self.current = ElectricCurrent::new::<ampere>(0.);
            self.current_direction = None;
        }
    }
}

#[cfg(test)]
mod battery_tests {
    use std::time::Duration;

    use uom::si::power::watt;

    use super::*;
    use crate::{
        electrical::{
            consumption::{PowerConsumer, SuppliedPower},
            ElectricalBusType,
        },
        simulation::{test::SimulationTestBed, Aircraft, SimulationElementVisitor},
    };

    struct BatteryTestBed {
        test_bed: SimulationTestBed,
    }
    impl BatteryTestBed {
        fn new() -> Self {
            Self::new_with_delta(Duration::from_secs(1))
        }

        fn new_with_delta(delta: Duration) -> Self {
            Self {
                test_bed: SimulationTestBed::new_with_delta(delta),
            }
        }

        fn run_aircraft<T: Aircraft>(&mut self, aircraft: &mut T) {
            self.test_bed.run_aircraft(aircraft);
        }

        fn current_is_normal(&mut self) -> bool {
            self.test_bed.read_bool("ELEC_BAT_1_CURRENT_NORMAL")
        }

        fn current(&mut self) -> ElectricCurrent {
            ElectricCurrent::new::<ampere>(self.test_bed.read_f64("ELEC_BAT_1_CURRENT"))
        }

        fn potential_is_normal(&mut self) -> bool {
            self.test_bed.read_bool("ELEC_BAT_1_POTENTIAL_NORMAL")
        }

        fn potential(&mut self) -> ElectricPotential {
            ElectricPotential::new::<volt>(self.test_bed.read_f64("ELEC_BAT_1_POTENTIAL"))
        }
    }

    struct TestAircraft {
        battery: Battery,
        consumer: PowerConsumer,
        battery_consumption: Power,
    }
    impl TestAircraft {
        fn new(battery: Battery) -> Self {
            Self {
                battery: battery,
                consumer: PowerConsumer::from(ElectricalBusType::DirectCurrentBattery),
                battery_consumption: Power::new::<watt>(0.),
            }
        }

        fn with_full_battery() -> Self {
            Self::new(Battery::full(1))
        }

        fn with_empty_battery() -> Self {
            Self::new(Battery::empty(1))
        }

        fn supply_input_potential(&mut self) {
            self.battery.powered_by(&Powered {});
        }

        fn battery_is_powered(&self) -> bool {
            self.battery.is_powered()
        }

        fn power_demand(&mut self, power: Power) {
            self.consumer.demand(power);
        }

        fn battery_charge(&self) -> ElectricCharge {
            self.battery.charge()
        }
    }
    impl Aircraft for TestAircraft {
        fn get_supplied_power(&mut self) -> SuppliedPower {
            let mut supplied_power = SuppliedPower::new();
            if self.battery.is_powered() {
                supplied_power.add(
                    ElectricalBusType::DirectCurrentBattery,
                    Potential::battery(1),
                );
            }

            supplied_power
        }
    }
    impl SimulationElement for TestAircraft {
        fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
            self.battery.accept(visitor);
            self.consumer.accept(visitor);

            visitor.visit(self);
        }

        fn process_power_consumption_report<T: PowerConsumptionReport>(&mut self, report: &T) {
            self.battery_consumption = report.total_consumption_of(&Potential::battery(1));
        }
    }

    struct Powerless {}
    impl PotentialSource for Powerless {
        fn output(&self) -> Potential {
            Potential::none()
        }
    }

    struct Powered {}
    impl PotentialSource for Powered {
        fn output(&self) -> Potential {
            Potential::transformer_rectifier(1).with_raw(ElectricPotential::new::<volt>(28.))
        }
    }

    #[test]
    fn when_full_without_input_has_output() {
        let mut aircraft = TestAircraft::with_full_battery();
        let mut test_bed = BatteryTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(aircraft.battery_is_powered());
    }

    #[test]
    fn when_full_and_has_input_doesnt_have_output() {
        let mut aircraft = TestAircraft::with_full_battery();
        let mut test_bed = BatteryTestBed::new();

        aircraft.supply_input_potential();
        test_bed.run_aircraft(&mut aircraft);

        assert!(!aircraft.battery_is_powered());
    }

    #[test]
    fn when_empty_without_input_has_no_output() {
        let mut aircraft = TestAircraft::with_empty_battery();
        let mut test_bed = BatteryTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!aircraft.battery_is_powered());
    }

    #[test]
    fn when_empty_and_has_input_doesnt_have_output() {
        let mut aircraft = TestAircraft::with_empty_battery();
        let mut test_bed = BatteryTestBed::new();

        aircraft.supply_input_potential();
        test_bed.run_aircraft(&mut aircraft);

        assert!(!aircraft.battery_is_powered());
    }

    #[test]
    fn when_full_has_potential() {
        let mut aircraft = TestAircraft::with_full_battery();
        let mut test_bed = BatteryTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(test_bed.potential(), ElectricPotential::new::<volt>(28.));
    }

    #[test]
    fn when_full_potential_is_normal() {
        let mut aircraft = TestAircraft::with_full_battery();
        let mut test_bed = BatteryTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.potential_is_normal());
    }

    #[test]
    fn when_empty_has_no_potential() {
        let mut aircraft = TestAircraft::with_empty_battery();
        let mut test_bed = BatteryTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(test_bed.potential(), ElectricPotential::new::<volt>(0.));
    }

    #[test]
    fn when_empty_potential_is_abnormal() {
        let mut aircraft = TestAircraft::with_empty_battery();
        let mut test_bed = BatteryTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(!test_bed.potential_is_normal());
    }

    #[test]
    fn when_charging_current_is_normal() {
        let mut aircraft = TestAircraft::with_empty_battery();
        let mut test_bed = BatteryTestBed::new();

        aircraft.supply_input_potential();
        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.current_is_normal());
    }

    #[test]
    fn when_charging_battery_current_is_charge_current() {
        let mut aircraft = TestAircraft::with_empty_battery();
        let mut test_bed = BatteryTestBed::new();

        aircraft.supply_input_potential();
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(test_bed.current(), ElectricCurrent::new::<ampere>(9.));
    }

    #[test]
    fn when_discharging_slowly_current_is_normal() {
        let mut aircraft = TestAircraft::with_full_battery();
        let mut test_bed = BatteryTestBed::new();

        aircraft.power_demand(Power::new::<watt>(28. * 5.));
        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.current_is_normal());
    }

    #[test]
    fn when_discharging_quickly_current_is_abnormal() {
        let mut aircraft = TestAircraft::with_full_battery();
        let mut test_bed = BatteryTestBed::new();

        aircraft.power_demand(Power::new::<watt>((28. * 5.) + 1.));
        test_bed.run_aircraft(&mut aircraft);

        assert!(!test_bed.current_is_normal());
    }

    #[test]
    fn when_discharging_battery_current_is_discharge_current() {
        let mut aircraft = TestAircraft::with_full_battery();
        let mut test_bed = BatteryTestBed::new();

        aircraft.power_demand(Power::new::<watt>(28. * 5.));
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(test_bed.current(), ElectricCurrent::new::<ampere>(5.));
    }

    #[test]
    fn when_discharging_loses_charge() {
        let mut aircraft = TestAircraft::with_full_battery();
        let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(60));

        let charge_prior_to_run = aircraft.battery_charge();

        aircraft.power_demand(Power::new::<watt>(28. * 5.));
        test_bed.run_aircraft(&mut aircraft);

        assert!(aircraft.battery_charge() < charge_prior_to_run);
    }

    #[test]
    fn when_charging_gains_charge() {
        let mut aircraft = TestAircraft::with_empty_battery();
        let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(60));

        let charge_prior_to_run = aircraft.battery_charge();

        aircraft.supply_input_potential();
        test_bed.run_aircraft(&mut aircraft);

        assert!(aircraft.battery_charge() > charge_prior_to_run);
    }

    #[test]
    fn writes_its_state() {
        let mut aircraft = TestAircraft::with_full_battery();
        let mut test_bed = SimulationTestBed::new();

        test_bed.run_aircraft(&mut aircraft);

        assert!(test_bed.contains_key("ELEC_BAT_1_CURRENT"));
        assert!(test_bed.contains_key("ELEC_BAT_1_CURRENT_NORMAL"));
        assert!(test_bed.contains_key("ELEC_BAT_1_POTENTIAL"));
        assert!(test_bed.contains_key("ELEC_BAT_1_POTENTIAL_NORMAL"));
    }
}
