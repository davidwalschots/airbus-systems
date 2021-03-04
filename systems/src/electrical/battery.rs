use super::{
    consumption::{PowerConsumption, PowerConsumptionReport},
    ElectricalStateWriter, Potential, PotentialOrigin, PotentialSource, PotentialTarget,
    ProvideCurrent, ProvidePotential,
};
use crate::simulation::{SimulationElement, SimulatorWriter, UpdateContext};
use uom::si::{
    electric_charge::ampere_hour, electric_current::ampere, electric_potential::volt, f64::*,
    time::second, velocity::knot,
};

pub struct BatteryChargeLimiterArguments {
    ac_buses_unpowered: bool,
}
impl BatteryChargeLimiterArguments {
    pub fn new(ac_buses_unpowered: bool) -> Self {
        Self { ac_buses_unpowered }
    }

    fn ac_buses_unpowered(&self) -> bool {
        self.ac_buses_unpowered
    }
}

pub struct BatteryChargeLimiter {
    should_close_contactor: bool,
}
impl BatteryChargeLimiter {
    pub fn new() -> Self {
        Self {
            should_close_contactor: false,
        }
    }

    pub fn update(
        &mut self,
        context: &UpdateContext,
        battery: &Battery,
        arguments: &BatteryChargeLimiterArguments,
    ) {
        self.should_close_contactor = battery.needs_charging()
            || (arguments.ac_buses_unpowered()
                && context.indicated_airspeed() < Velocity::new::<knot>(100.))
    }

    pub fn should_close_contactor(&self) -> bool {
        self.should_close_contactor
    }
}

// trait BatteryChargeLimiterUpdateArguments {
//     fn apu_started(&self) -> bool;
// }

// pub struct BatteryChargeLimiter {
//     should_charge_battery_delayed: DelayedTrueLogicGate,
//     current_less_than_4_for_10_seconds_or_more: DelayedTrueLogicGate,
//     current_less_than_4_for_30_minutes_or_more: DelayedTrueLogicGate,
//     apu_last_started_time_ago: Duration,
//     apu_started_in_last_tick: bool,
//     should_close_battery_contactor: bool,
// }
// impl BatteryChargeLimiter {
//     fn new() -> Self {
//         Self {
//             should_charge_battery_delayed: DelayedTrueLogicGate::new(Duration::from_millis(225)),
//             current_less_than_4_for_10_seconds_or_more: DelayedTrueLogicGate::new(
//                 Duration::from_secs(10),
//             ),
//             current_less_than_4_for_30_minutes_or_more: DelayedTrueLogicGate::new(
//                 Duration::from_secs(1800),
//             ),
//             apu_last_started_time_ago: Duration::from_secs(86400),
//             apu_started_in_last_tick: false,
//             should_close_battery_contactor: false,
//         }
//     }

//     fn update<T: BatteryChargeLimiterUpdateArguments>(
//         &mut self,
//         context: &UpdateContext,
//         battery: &Battery,
//         battery_bus: &ElectricalBus,
//         arguments: &T,
//     ) {
//         if arguments.apu_started() && !self.apu_started_in_last_tick {
//             self.apu_last_started_time_ago = Duration::from_secs(0);
//         }
//         self.apu_started_in_last_tick = arguments.apu_started();
//         self.apu_last_started_time_ago += context.delta;

//         self.should_charge_battery_delayed
//             .update(context, self.should_charge_battery(battery, battery_bus));
//         self.current_less_than_4_for_10_seconds_or_more.update(
//             context,
//             battery.current() < ElectricCurrent::new::<ampere>(4.),
//         );
//         self.current_less_than_4_for_30_minutes_or_more.update(
//             context,
//             battery.current() < ElectricCurrent::new::<ampere>(4.),
//         );

//         if self.should_close_battery_contactor &&
//             (self.charging_current_is_less_than_4_ampere_for_10_seconds_or_more_on_ground_without_apu_start_in_last_30_minutes(context) ||
//             self.charging_current_is_less_than_4_ampere_for_30_minutes_or_more_with_apu_start_more_than_30_minutes_ago_or_airspeed_above_100_knots(context)) {
//             self.should_close_battery_contactor = false;
//         } else if !self.should_close_battery_contactor && self.should_charge_battery_delayed.output() {
//             self.should_close_battery_contactor = true;
//         }
//     }

//     fn should_charge_battery(&self, battery: &Battery, battery_bus: &ElectricalBus) -> bool {
//         battery.output().raw() < ElectricPotential::new::<volt>(26.5)
//             && battery_bus.output().raw() > ElectricPotential::new::<volt>(27.)
//     }

//     fn should_close_battery_contactor(&self) -> bool {
//         self.should_close_battery_contactor
//     }

//     fn charging_current_is_less_than_4_ampere_for_10_seconds_or_more_on_ground_without_apu_start_in_last_30_minutes(
//         &self,
//         context: &UpdateContext,
//     ) -> bool {
//         self.current_less_than_4_for_10_seconds_or_more.output()
//             && self.apu_last_started_time_ago >= Duration::from_secs(1800)
//             && context.is_on_ground
//     }

//     fn charging_current_is_less_than_4_ampere_for_30_minutes_or_more_with_apu_start_more_than_30_minutes_ago_or_airspeed_above_100_knots(
//         &self,
//         context: &UpdateContext,
//     ) -> bool {
//         self.current_less_than_4_for_30_minutes_or_more.output()
//             && (self.apu_last_started_time_ago >= Duration::from_secs(1800)
//                 || context.indicated_airspeed > Velocity::new::<knot>(100.))
//     }
// }

pub struct Battery {
    number: usize,
    writer: ElectricalStateWriter,
    input_potential: Potential,
    charge: ElectricCharge,
    output_potential: ElectricPotential,
    charge_current: ElectricCurrent,
    discharge_current: ElectricCurrent,
}
impl Battery {
    const MAX_ELECTRIC_CHARGE_AMPERE_HOURS: f64 = 23.;

    pub fn full(number: usize) -> Battery {
        Battery::new(
            number,
            ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS),
        )
    }

    pub fn half(number: usize) -> Battery {
        Battery::new(
            number,
            ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS / 2.),
        )
    }

    pub fn empty(number: usize) -> Battery {
        Battery::new(number, ElectricCharge::new::<ampere_hour>(0.))
    }

    pub fn new(number: usize, charge: ElectricCharge) -> Self {
        Self {
            number,
            writer: ElectricalStateWriter::new(&format!("BAT_{}", number)),
            input_potential: Potential::none(),
            charge,
            output_potential: Battery::calculate_potential_for_charge(charge),
            charge_current: ElectricCurrent::new::<ampere>(0.),
            discharge_current: ElectricCurrent::new::<ampere>(0.),
        }
    }

    pub fn needs_charging(&self) -> bool {
        self.charge
            <= ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS - 3.)
    }

    // fn fully_charged(&self) -> bool {
    //     self.charge >= ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS)
    // }

    // fn has_charge(&self) -> bool {
    //     self.charge > ElectricCharge::new::<ampere_hour>(0.)
    // }

    fn is_powered_by_other_potential(&self) -> bool {
        self.input_potential.is_powered()
            && !self
                .input_potential
                .is_single(PotentialOrigin::Battery(self.number))
    }

    pub fn input_potential(&self) -> Potential {
        self.input_potential
    }

    #[cfg(test)]
    fn charge(&self) -> ElectricCharge {
        self.charge
    }

    fn calculate_potential_for_charge(charge: ElectricCharge) -> ElectricPotential {
        if charge == ElectricCharge::new::<ampere_hour>(0.) {
            return ElectricPotential::new::<volt>(0.);
        }

        const POTENTIAL_CONST: f64 = 20.015739802940814;
        const POTENTIAL_X: f64 = 5.399275255088176;
        const POTENTIAL_X2: f64 = -3.5074029701952085;
        const POTENTIAL_X3: f64 = 1.1541944463099747;
        const POTENTIAL_X4: f64 = -0.21476918325806746;
        const POTENTIAL_X5: f64 = 0.02400262380751744;
        const POTENTIAL_X6: f64 = -0.0016414408818473919;
        const POTENTIAL_X7: f64 = 0.00006718898287051494;
        const POTENTIAL_X8: f64 = -0.00000150999595170716;
        const POTENTIAL_X9: f64 = 0.00000001431467173089;

        let charge = charge.get::<ampere_hour>();

        ElectricPotential::new::<volt>(
            POTENTIAL_CONST
                + (POTENTIAL_X * charge)
                + (POTENTIAL_X2 * charge.powi(2))
                + (POTENTIAL_X3 * charge.powi(3))
                + (POTENTIAL_X4 * charge.powi(4))
                + (POTENTIAL_X5 * charge.powi(5))
                + (POTENTIAL_X6 * charge.powi(6))
                + (POTENTIAL_X7 * charge.powi(7))
                + (POTENTIAL_X8 * charge.powi(8))
                + (POTENTIAL_X9 * charge.powi(9)),
        )
    }

    fn calculate_charging_current_for_charge(charge: ElectricCharge) -> ElectricCurrent {
        if charge >= ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS) {
            return ElectricCurrent::new::<ampere>(0.01);
        }

        const CURRENT_CONST: f64 = 0.16056266290124882;
        const CURRENT_X: f64 = 0.29777831170087965;
        const CURRENT_X2: f64 = 0.4117521045535857;
        const CURRENT_X3: f64 = 0.03147169336065588;
        const CURRENT_X4: f64 = -0.024537434663775688;
        const CURRENT_X5: f64 = 0.0031783746208521165;
        const CURRENT_X6: f64 = -0.0001826392797145209;
        const CURRENT_X7: f64 = 0.00000497197899440307;
        const CURRENT_X8: f64 = -0.00000005222454626826;

        let charge = charge.get::<ampere_hour>();

        ElectricCurrent::new::<ampere>(
            CURRENT_CONST
                + (CURRENT_X * charge)
                + (CURRENT_X2 * charge.powi(2))
                + (CURRENT_X3 * charge.powi(3))
                + (CURRENT_X4 * charge.powi(4))
                + (CURRENT_X5 * charge.powi(5))
                + (CURRENT_X6 * charge.powi(6))
                + (CURRENT_X7 * charge.powi(7))
                + (CURRENT_X8 * charge.powi(8)),
        )
    }
}
potential_target!(Battery);
impl PotentialSource for Battery {
    fn output(&self) -> Potential {
        if self.output_potential > ElectricPotential::new::<volt>(0.) {
            Potential::single(PotentialOrigin::Battery(self.number), self.output_potential)
        } else {
            Potential::none()
        }
    }
}
impl ProvideCurrent for Battery {
    fn current(&self) -> ElectricCurrent {
        if self.charge_current > self.discharge_current {
            self.charge_current - self.discharge_current
        } else {
            self.discharge_current - self.charge_current
        }
    }

    fn current_normal(&self) -> bool {
        self.charge_current > self.discharge_current
            || (ElectricCurrent::new::<ampere>(0.0)..=ElectricCurrent::new::<ampere>(5.0))
                .contains(&self.current())
    }
}
provide_potential!(Battery, (25.0..=31.0));
impl SimulationElement for Battery {
    fn write(&self, writer: &mut SimulatorWriter) {
        self.writer.write_direct(self, writer);
    }

    fn consume_power(&mut self, consumption: &mut PowerConsumption) {
        if self.is_powered_by_other_potential() {
            self.charge_current = Battery::calculate_charging_current_for_charge(self.charge);
            let input_potential = self
                .input_potential
                .average_without(PotentialOrigin::Battery(self.number));

            if input_potential > ElectricPotential::new::<volt>(0.) {
                let power = input_potential * self.charge_current;
                consumption.add_ignoring(
                    PotentialOrigin::Battery(self.number),
                    &self.input_potential,
                    power,
                );

                let time = Time::new::<second>(consumption.delta().as_secs_f64());
                self.charge += ((input_potential * self.charge_current) * time) / input_potential;
            }
        } else {
            self.charge_current = ElectricCurrent::new::<ampere>(0.);
        }
    }

    fn process_power_consumption_report<T: PowerConsumptionReport>(&mut self, report: &T) {
        self.output_potential = Battery::calculate_potential_for_charge(self.charge);

        let consumption = report.total_consumption_of(PotentialOrigin::Battery(self.number));
        self.discharge_current = if self.output_potential > ElectricPotential::new::<volt>(0.) {
            consumption / self.output_potential
        } else {
            ElectricCurrent::new::<ampere>(0.)
        };

        if self.output_potential > ElectricPotential::new::<volt>(0.) {
            let time = Time::new::<second>(report.delta().as_secs_f64());
            self.charge -= ((consumption * time) / self.output_potential).min(self.charge);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Powered {
        potential: ElectricPotential,
    }
    impl Powered {
        fn new(potential: ElectricPotential) -> Self {
            Self { potential }
        }
    }
    impl PotentialSource for Powered {
        fn output(&self) -> Potential {
            Potential::single(PotentialOrigin::TransformerRectifier(1), self.potential)
        }
    }

    // #[cfg(test)]
    // mod battery_charge_limiter_tests {
    //     use std::time::Duration;

    //     use crate::{
    //         electrical::{Contactor, ElectricalBus, ElectricalBusType},
    //         simulation::{test::SimulationTestBed, Aircraft, SimulationElementVisitor},
    //     };

    //     use super::*;

    //     struct BatteryChargeLimiterUpdateArgs {
    //         apu_started: bool,
    //     }
    //     impl BatteryChargeLimiterUpdateArgs {
    //         fn new(apu_started: bool) -> Self {
    //             Self { apu_started }
    //         }
    //     }
    //     impl BatteryChargeLimiterUpdateArguments for BatteryChargeLimiterUpdateArgs {
    //         fn apu_started(&self) -> bool {
    //             self.apu_started
    //         }
    //     }

    //     struct BatteryChargeLimiterTestBed {
    //         test_bed: SimulationTestBed,
    //         delta: Duration,
    //     }
    //     impl BatteryChargeLimiterTestBed {
    //         fn new() -> Self {
    //             Self::new_with_delta(Duration::from_secs(1))
    //         }

    //         fn new_with_delta(delta: Duration) -> Self {
    //             Self {
    //                 test_bed: SimulationTestBed::new_with_delta(delta),
    //                 delta,
    //             }
    //         }

    //         fn set_delta(&mut self, delta: Duration) {
    //             self.delta = delta;
    //         }

    //         pub fn set_on_ground(&mut self, on_ground: bool) {
    //             self.test_bed.set_on_ground(on_ground);
    //         }

    //         fn run_aircraft<T: Aircraft>(&mut self, aircraft: &mut T) {
    //             // Firstly run without any time passing at all, such that if the DelayedTrueLogicGate reaches
    //             // the true state after waiting for the given time it will be reflected in its output.
    //             self.test_bed.set_delta(Duration::from_secs(0));
    //             self.test_bed.run_aircraft(aircraft);

    //             self.test_bed.set_delta(Duration::from_secs(0));
    //             self.test_bed.run_aircraft(aircraft);

    //             self.test_bed.set_delta(self.delta);
    //             self.test_bed.run_aircraft(aircraft);
    //         }

    //         fn with_charging_battery(mut self, aircraft: &mut TestAircraft) -> Self {
    //             self.set_delta(Duration::from_millis(225));

    //             aircraft.power_battery_bus();
    //             self.run_aircraft(aircraft);

    //             self
    //         }
    //     }

    //     struct TestAircraft {
    //         battery: Battery,
    //         battery_charge_limiter: BatteryChargeLimiter,
    //         battery_bus: ElectricalBus,
    //         battery_contactor: Contactor,
    //         running_apu: bool,
    //     }
    //     impl TestAircraft {
    //         fn new(battery: Battery) -> Self {
    //             Self {
    //                 battery: battery,
    //                 battery_charge_limiter: BatteryChargeLimiter::new(),
    //                 battery_bus: ElectricalBus::new(ElectricalBusType::DirectCurrentBattery),
    //                 battery_contactor: Contactor::new("TEST"),
    //                 running_apu: false,
    //             }
    //         }

    //         fn with_full_battery() -> Self {
    //             Self::new(Battery::full(1))
    //         }

    //         fn with_empty_battery() -> Self {
    //             Self::new(Battery::empty(1))
    //         }

    //         fn power_battery_bus(&mut self) {
    //             self.battery_bus
    //                 .powered_by(&Powered::new(ElectricPotential::new::<volt>(28.)))
    //         }

    //         fn should_close_battery_contactor(&self) -> bool {
    //             self.battery_charge_limiter.should_close_battery_contactor()
    //         }

    //         fn set_full_battery(&mut self) {
    //             self.battery.fully_charge();
    //         }

    //         fn start_apu(&mut self) {
    //             self.running_apu = true;
    //         }

    //         fn stop_apu(&mut self) {
    //             self.running_apu = false;
    //         }
    //     }
    //     impl Aircraft for TestAircraft {
    //         fn update_before_power_distribution(&mut self, context: &UpdateContext) {
    //             self.battery_charge_limiter.update(
    //                 context,
    //                 &self.battery,
    //                 &self.battery_bus,
    //                 &BatteryChargeLimiterUpdateArgs::new(self.running_apu),
    //             );
    //             self.battery_contactor
    //                 .close_when(self.battery_charge_limiter.should_close_battery_contactor());

    //             self.battery_contactor.powered_by(&self.battery_bus);
    //             self.battery.powered_by(&self.battery_contactor);
    //             self.battery_contactor.or_powered_by(&self.battery);
    //             self.battery_bus.or_powered_by(&self.battery_contactor);
    //         }
    //     }
    //     impl SimulationElement for TestAircraft {
    //         fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
    //             self.battery.accept(visitor);
    //             self.battery_bus.accept(visitor);
    //             self.battery_contactor.accept(visitor);

    //             visitor.visit(self);
    //         }
    //     }

    //     #[test]
    //     fn should_close_contactor_when_battery_needs_charging_and_225_ms_passed() {
    //         let mut aircraft = TestAircraft::with_empty_battery();
    //         let mut test_bed =
    //             BatteryChargeLimiterTestBed::new_with_delta(Duration::from_millis(225));

    //         aircraft.power_battery_bus();
    //         test_bed.run_aircraft(&mut aircraft);

    //         assert!(aircraft.should_close_battery_contactor());
    //     }

    //     #[test]
    //     fn should_not_yet_close_contactor_when_battery_needs_charging_and_224_ms_passed() {
    //         let mut aircraft = TestAircraft::with_empty_battery();
    //         let mut test_bed =
    //             BatteryChargeLimiterTestBed::new_with_delta(Duration::from_millis(224));

    //         aircraft.power_battery_bus();
    //         test_bed.run_aircraft(&mut aircraft);

    //         assert!(!aircraft.should_close_battery_contactor());
    //     }

    //     #[test]
    //     fn should_open_contactor_when_charging_current_less_than_4_a_for_10_seconds_on_ground_and_apu_not_started(
    //     ) {
    //         // Ich < 4A for 10s on ground.
    //         // APU start on ground overrides this?
    //         let mut aircraft = TestAircraft::with_empty_battery();
    //         let mut test_bed =
    //             BatteryChargeLimiterTestBed::new().with_charging_battery(&mut aircraft);

    //         assert!(
    //             aircraft.should_close_battery_contactor(),
    //             "The test expects the contactor to be closed at this point."
    //         );

    //         aircraft.set_full_battery();
    //         test_bed.set_on_ground(true);
    //         test_bed.set_delta(Duration::from_secs(10));
    //         test_bed.run_aircraft(&mut aircraft);

    //         assert!(!aircraft.should_close_battery_contactor());
    //     }

    //     #[test]
    //     fn should_not_yet_open_contactor_when_charging_current_less_than_4_a_for_9999_milliseconds_on_ground_and_apu_not_started(
    //     ) {
    //         // Ich < 4A for 10s on ground.
    //         // APU start on ground overrides this?
    //         let mut aircraft = TestAircraft::with_empty_battery();
    //         let mut test_bed =
    //             BatteryChargeLimiterTestBed::new().with_charging_battery(&mut aircraft);

    //         assert!(
    //             aircraft.should_close_battery_contactor(),
    //             "The test expects the contactor to be closed at this point."
    //         );

    //         aircraft.set_full_battery();
    //         test_bed.set_delta(Duration::from_millis(9999));
    //         test_bed.run_aircraft(&mut aircraft);

    //         assert!(aircraft.should_close_battery_contactor());
    //     }

    //     #[test]
    //     fn should_open_contactor_when_charging_current_less_than_4_a_for_30_minutes_after_apu_start(
    //     ) {
    //         // Ich < 4A for more than 30 min or following an APU start on the ground.
    //         let mut aircraft = TestAircraft::with_empty_battery();
    //         let mut test_bed =
    //             BatteryChargeLimiterTestBed::new().with_charging_battery(&mut aircraft);

    //         assert!(
    //             aircraft.should_close_battery_contactor(),
    //             "The test expects the contactor to be closed at this point."
    //         );

    //         aircraft.start_apu();
    //         aircraft.set_full_battery();
    //         test_bed.set_delta(Duration::from_secs(1800));
    //         test_bed.run_aircraft(&mut aircraft);

    //         assert!(!aircraft.should_close_battery_contactor());
    //     }

    //     #[test]
    //     fn should_not_yet_open_contactor_when_charging_current_less_than_4_a_for_29_minutes_and_59_seconds_after_apu_start(
    //     ) {
    //         // Ich < 4A for more than 30 min or following an APU start on the ground.
    //         let mut aircraft = TestAircraft::with_empty_battery();
    //         let mut test_bed =
    //             BatteryChargeLimiterTestBed::new().with_charging_battery(&mut aircraft);

    //         assert!(
    //             aircraft.should_close_battery_contactor(),
    //             "The test expects the contactor to be closed at this point."
    //         );

    //         aircraft.start_apu();
    //         aircraft.set_full_battery();
    //         test_bed.set_delta(Duration::from_secs(1799));
    //         test_bed.run_aircraft(&mut aircraft);

    //         assert!(aircraft.should_close_battery_contactor());
    //     }

    //     #[test]
    //     fn should_not_yet_open_contactor_when_charging_current_less_than_4_a_for_30_minutes_after_multiple_apu_start(
    //     ) {
    //         // Ich < 4A for more than 30 min or following an APU start on the ground.
    //         let mut aircraft = TestAircraft::with_empty_battery();
    //         let mut test_bed =
    //             BatteryChargeLimiterTestBed::new().with_charging_battery(&mut aircraft);

    //         assert!(
    //             aircraft.should_close_battery_contactor(),
    //             "The test expects the contactor to be closed at this point."
    //         );

    //         aircraft.start_apu();
    //         aircraft.set_full_battery();
    //         test_bed.set_delta(Duration::from_secs(1500));
    //         test_bed.run_aircraft(&mut aircraft);
    //         aircraft.stop_apu();
    //         test_bed.set_delta(Duration::from_secs(100));
    //         test_bed.run_aircraft(&mut aircraft);
    //         aircraft.start_apu();
    //         test_bed.set_delta(Duration::from_secs(200));
    //         test_bed.run_aircraft(&mut aircraft);

    //         assert!(aircraft.should_close_battery_contactor());
    //     }

    //     #[test]
    //     fn should_open_contactor_when_charging_current_less_than_4_a_for_30_minutes_above_100_knots(
    //     ) {
    //         // Ich < 4A for more than 30 min with speed > 100 kts.
    //     }

    //     #[test]
    //     fn should_open_contactor_when_complete_discharge_protection_on_ground() {
    //         // Vbat < 23 V for longer than 15s on ground.
    //     }

    //     #[test]
    //     fn should_close_contactor_when_apu_master_sw_push_button_is_on() {
    //         // Batteries are connected to the DC BAT BUS when APU MASTER pb is pressed.
    //     }

    //     #[test]
    //     fn in_emer_elec_config_contactor_closing_for_apu_start_inhibited_for_first_45_seconds() {
    //         // EMER ELEC CONFIG + 45s AND emergency generator not available.
    //     }

    //     #[test]
    //     fn in_emer_elec_config_contactor_closing_for_apu_start_no_longer_inhibited_after_45_seconds(
    //     ) {
    //         // EMER ELEC CONFIG + 45s AND emergency generator not available.
    //     }

    //     #[test]
    //     fn in_emer_elec_config_contactor_closing_for_apu_start_no_longer_inhibited_once_emer_gen_available(
    //     ) {
    //         // EMER ELEC CONFIG + 45s AND emergency generator not available.
    //     }

    //     #[test]
    //     fn in_emer_elec_config_contactor_closing_for_apu_start_inhibited_when_landing_gear_extended(
    //     ) {
    //         // In emer elec config, when the landing gear is down, no more APU start is possible.
    //     }

    //     #[test]
    //     fn in_emer_elec_config_contactor_closing_for_charging_inhibited() {
    //         // When in emergency electrical configuration, the emergency generator supplies the ESS busses. Batteries are disconnected.
    //     }
    // }

    #[cfg(test)]
    mod battery_tests {
        use super::*;
        use crate::{
            electrical::{
                consumption::{PowerConsumer, SuppliedPower},
                ElectricalBusType,
            },
            simulation::{test::SimulationTestBed, Aircraft, SimulationElementVisitor},
        };
        use std::time::Duration;
        use uom::si::power::watt;

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

            fn with_half_charged_battery() -> Self {
                Self::new(Battery::half(1))
            }

            fn with_nearly_empty_battery() -> Self {
                Self::new(Battery::new(1, ElectricCharge::new::<ampere_hour>(0.001)))
            }

            fn with_empty_battery() -> Self {
                Self::new(Battery::empty(1))
            }

            fn supply_input_potential(&mut self) {
                self.battery
                    .powered_by(&Powered::new(ElectricPotential::new::<volt>(28.)));
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
                        Potential::single(
                            PotentialOrigin::Battery(1),
                            ElectricPotential::new::<volt>(28.),
                        ),
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
                self.battery_consumption = report.total_consumption_of(PotentialOrigin::Battery(1));
            }
        }

        struct Powerless {}
        impl PotentialSource for Powerless {
            fn output(&self) -> Potential {
                Potential::none()
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
        fn when_empty_without_input_has_no_output() {
            let mut aircraft = TestAircraft::with_empty_battery();
            let mut test_bed = BatteryTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(!aircraft.battery_is_powered());
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

        // #[test]
        // fn when_charging_battery_current_is_charge_current() {
        //     let mut aircraft = TestAircraft::with_empty_battery();
        //     let mut test_bed = BatteryTestBed::new();

        //     aircraft.supply_input_potential();
        //     test_bed.run_aircraft(&mut aircraft);

        //     assert_eq!(test_bed.current(), ElectricCurrent::new::<ampere>(9.));
        // }

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
        fn when_discharging_battery_current_is_approximately_discharge_current() {
            let mut aircraft = TestAircraft::with_full_battery();
            let mut test_bed = BatteryTestBed::new();

            aircraft.power_demand(Power::new::<watt>(140.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(
                ElectricCurrent::new::<ampere>(4.999) < test_bed.current()
                    && test_bed.current() < ElectricCurrent::new::<ampere>(5.001)
            )
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
        fn when_fully_charged_has_approximately_28_volt_potential() {
            let mut aircraft = TestAircraft::with_full_battery();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(0));

            test_bed.run_aircraft(&mut aircraft);

            assert!(
                ElectricPotential::new::<volt>(27.999) < test_bed.potential()
                    && test_bed.potential() < ElectricPotential::new::<volt>(28.001)
            );
        }

        #[test]
        fn when_half_charged_has_approximately_24_volt_potential() {
            let mut aircraft = TestAircraft::with_half_charged_battery();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(0));

            test_bed.run_aircraft(&mut aircraft);

            assert!(
                ElectricPotential::new::<volt>(23.9) < test_bed.potential()
                    && test_bed.potential() < ElectricPotential::new::<volt>(24.1)
            );
        }

        #[test]
        fn when_almost_empty_has_approximately_20_volt_potential() {
            let mut aircraft = TestAircraft::with_nearly_empty_battery();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(0));

            test_bed.run_aircraft(&mut aircraft);

            assert!(
                ElectricPotential::new::<volt>(19.97) < test_bed.potential()
                    && test_bed.potential() < ElectricPotential::new::<volt>(20.03)
            );
        }

        #[test]
        fn cannot_discharge_below_zero() {
            let mut aircraft = TestAircraft::with_nearly_empty_battery();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(50));

            aircraft.power_demand(Power::new::<watt>(5000.));
            test_bed.run_aircraft(&mut aircraft);

            assert_eq!(
                aircraft.battery_charge(),
                ElectricCharge::new::<ampere_hour>(0.)
            )
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
}
