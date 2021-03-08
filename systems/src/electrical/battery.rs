use std::time::Duration;

use super::{
    consumption::{PowerConsumption, PowerConsumptionReport},
    ElectricalStateWriter, Potential, PotentialOrigin, PotentialSource, PotentialTarget,
    ProvideCurrent, ProvidePotential,
};
use crate::{
    shared::DelayedTrueLogicGate,
    simulation::{SimulationElement, SimulatorWriter, UpdateContext},
};
use uom::si::{
    electric_charge::ampere_hour, electric_current::ampere, electric_potential::volt,
    electrical_resistance::ohm, f64::*, time::second, velocity::knot,
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
    should_show_arrow_when_contactor_closed_id: String,
    should_close_contactor: bool,
    discharging_for_15_seconds_or_more: DelayedTrueLogicGate,
    charging_for_15_seconds_or_more: DelayedTrueLogicGate,
}
impl BatteryChargeLimiter {
    pub fn new(contactor_id: &str) -> Self {
        Self {
            should_show_arrow_when_contactor_closed_id: format!(
                "ELEC_CONTACTOR_{}_SHOW_ARROW_WHEN_CLOSED",
                contactor_id
            ),
            should_close_contactor: false,
            discharging_for_15_seconds_or_more: DelayedTrueLogicGate::new(Duration::from_secs(15)),
            charging_for_15_seconds_or_more: DelayedTrueLogicGate::new(Duration::from_secs(15)),
        }
    }

    pub fn update(
        &mut self,
        context: &UpdateContext,
        battery: &Battery,
        arguments: &BatteryChargeLimiterArguments,
    ) {
        self.discharging_for_15_seconds_or_more.update(
            context,
            battery.current() < ElectricCurrent::new::<ampere>(-1.),
        );
        self.charging_for_15_seconds_or_more.update(
            context,
            battery.current() > ElectricCurrent::new::<ampere>(1.),
        );

        self.should_close_contactor = battery.needs_charging()
            || (arguments.ac_buses_unpowered()
                && context.indicated_airspeed() < Velocity::new::<knot>(100.))
    }

    pub fn should_close_contactor(&self) -> bool {
        self.should_close_contactor
    }
}
impl SimulationElement for BatteryChargeLimiter {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_bool(
            &self.should_show_arrow_when_contactor_closed_id,
            self.discharging_for_15_seconds_or_more.output()
                || self.charging_for_15_seconds_or_more.output(),
        );
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
    current: ElectricCurrent,
}
impl Battery {
    const RATED_CAPACITY_AMPERE_HOURS: f64 = 23.;

    pub fn full(number: usize) -> Battery {
        Battery::new(
            number,
            ElectricCharge::new::<ampere_hour>(Battery::RATED_CAPACITY_AMPERE_HOURS),
        )
    }

    pub fn half(number: usize) -> Battery {
        Battery::new(
            number,
            ElectricCharge::new::<ampere_hour>(Battery::RATED_CAPACITY_AMPERE_HOURS / 2.),
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
            output_potential: Battery::calculate_output_potential_for_charge(charge),
            current: ElectricCurrent::new::<ampere>(0.),
        }
    }

    pub fn needs_charging(&self) -> bool {
        self.charge <= ElectricCharge::new::<ampere_hour>(Battery::RATED_CAPACITY_AMPERE_HOURS - 3.)
    }

    fn is_powered_by_other_potential(&self) -> bool {
        self.input_potential.raw() > self.output_potential
    }

    pub fn input_potential(&self) -> Potential {
        self.input_potential
    }

    #[cfg(test)]
    fn charge(&self) -> ElectricCharge {
        self.charge
    }

    fn calculate_output_potential_for_charge(charge: ElectricCharge) -> ElectricPotential {
        // There are four distinct charges, being:
        // 1. No charge, giving no potential.
        // 2. Low charge, rapidly decreasing from 26.578V.
        // 3. Regular charge, linear from 26.578V to 27.33V.
        // 4. High charge, rapidly increasing from 27.33V to 28.958V.
        // Refer to Battery.md for details.
        let charge = charge.get::<ampere_hour>();
        ElectricPotential::new::<volt>(if charge <= 0. {
            0.
        } else if charge <= 3.488 {
            (13.95303731988 * charge) - 2. * charge.powi(2)
        } else if charge < 22.449 {
            23.85 + 0.14 * charge
        } else {
            8483298.
                + (-2373273.312763873 * charge)
                + (276476.10619333945 * charge.powi(2))
                + (-17167.409762003314 * charge.powi(3))
                + (599.2597390001015 * charge.powi(4))
                + (-11.149802489333474 * charge.powi(5))
                + (0.08638809969727154 * charge.powi(6))
        })
    }

    fn calculate_charging_current(
        input: ElectricPotential,
        output: ElectricPotential,
    ) -> ElectricCurrent {
        // Internal resistance = 0.011 ohm. However that would make current go through the roof.
        // Thus we add some fake wire resistance here too. If needed, later one can
        // add resistance of wires between buses to calculate correct values.
        let resistance = ElectricalResistance::new::<ohm>(0.15);
        ((input - output) / resistance)
            .min(ElectricCurrent::new::<ampere>(10.))
            .max(ElectricCurrent::new::<ampere>(0.))
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
        self.current
    }

    fn current_normal(&self) -> bool {
        (ElectricCurrent::new::<ampere>(-5.0)..=ElectricCurrent::new::<ampere>(f64::MAX))
            .contains(&self.current)
    }
}
impl ProvidePotential for Battery {
    fn potential(&self) -> ElectricPotential {
        self.output_potential.max(self.input_potential.raw())
    }

    fn potential_normal(&self) -> bool {
        (ElectricPotential::new::<volt>(25.0)..=ElectricPotential::new::<volt>(31.0))
            .contains(&self.potential())
    }
}
impl SimulationElement for Battery {
    fn write(&self, writer: &mut SimulatorWriter) {
        self.writer.write_direct(self, writer);
    }

    fn consume_power(&mut self, consumption: &mut PowerConsumption) {
        if self.is_powered_by_other_potential() {
            self.current = Battery::calculate_charging_current(
                self.input_potential.raw(),
                self.output_potential,
            );

            let power = self.input_potential.raw() * self.current;
            consumption.add(&self.input_potential, power);

            let time = Time::new::<second>(consumption.delta().as_secs_f64());
            self.charge +=
                ((self.input_potential.raw() * self.current) * time) / self.input_potential.raw();
        }
    }

    fn process_power_consumption_report<T: PowerConsumptionReport>(&mut self, report: &T) {
        if !self.is_powered_by_other_potential() {
            let consumption = report.total_consumption_of(PotentialOrigin::Battery(self.number));

            self.current = if self.output_potential > ElectricPotential::new::<volt>(0.) {
                -(consumption / self.output_potential)
            } else {
                ElectricCurrent::new::<ampere>(0.)
            };

            if self.output_potential > ElectricPotential::new::<volt>(0.) {
                let time = Time::new::<second>(report.delta().as_secs_f64());
                self.charge -= ((consumption * time) / self.output_potential).min(self.charge);
            }
        }

        self.output_potential = Battery::calculate_output_potential_for_charge(self.charge);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod battery_charge_limiter_tests {
        use super::*;
        use crate::{
            electrical::{
                consumption::{PowerConsumer, SuppliedPower},
                Contactor, ElectricalBus, ElectricalBusType,
            },
            simulation::{test::SimulationTestBed, Aircraft, SimulationElementVisitor},
        };
        use std::time::Duration;
        use uom::si::power::watt;

        struct BatteryChargeLimiterTestBed {
            test_bed: SimulationTestBed,
            delta: Duration,
        }
        impl BatteryChargeLimiterTestBed {
            // fn new() -> Self {
            //     Self::new_with_delta(Duration::from_secs(1))
            // }

            fn new_with_delta(delta: Duration) -> Self {
                Self {
                    test_bed: SimulationTestBed::new_with_delta(delta),
                    delta,
                }
            }

            // fn set_delta(&mut self, delta: Duration) {
            //     self.delta = delta;
            // }

            // pub fn set_on_ground(&mut self, on_ground: bool) {
            //     self.test_bed.set_on_ground(on_ground);
            // }

            fn run_aircraft<T: Aircraft>(&mut self, aircraft: &mut T) {
                // The battery's current is updated after the BCL, thus we need two ticks.
                self.test_bed.set_delta(Duration::from_secs(0));
                self.test_bed.run_aircraft(aircraft);

                self.test_bed.set_delta(self.delta);
                self.test_bed.run_aircraft(aircraft);
            }

            // fn with_charging_battery(mut self, aircraft: &mut TestAircraft) -> Self {
            //     self.set_delta(Duration::from_millis(225));

            //     aircraft.power_battery_bus();
            //     self.run_aircraft(aircraft);

            //     self
            // }

            fn should_show_arrow_when_contactor_closed(&mut self) -> bool {
                self.test_bed
                    .read_bool("ELEC_CONTACTOR_TEST_SHOW_ARROW_WHEN_CLOSED")
            }
        }

        struct TestAircraft {
            battery: Battery,
            battery_charge_limiter: BatteryChargeLimiter,
            battery_bus: ElectricalBus,
            battery_contactor: Contactor,
            consumer: PowerConsumer,
        }
        impl TestAircraft {
            fn new(battery: Battery) -> Self {
                Self {
                    battery: battery,
                    battery_charge_limiter: BatteryChargeLimiter::new("TEST"),
                    battery_bus: ElectricalBus::new(ElectricalBusType::DirectCurrentBattery),
                    battery_contactor: Contactor::new("TEST"),
                    consumer: PowerConsumer::from(ElectricalBusType::DirectCurrentBattery),
                }
            }

            fn with_full_battery() -> Self {
                Self::new(Battery::full(1))
            }

            fn with_empty_battery() -> Self {
                Self::new(Battery::empty(1))
            }

            fn power_battery_bus(&mut self) {
                self.battery_bus.powered_by(&Potential::single(
                    PotentialOrigin::TransformerRectifier(1),
                    ElectricPotential::new::<volt>(28.),
                ))
            }

            fn power_demand(&mut self, power: Power) {
                self.consumer.demand(power);
            }

            fn close_battery_contactor(&mut self) {
                self.battery_contactor.close_when(true);
            }
        }
        impl Aircraft for TestAircraft {
            fn update_before_power_distribution(&mut self, context: &UpdateContext) {
                self.battery_charge_limiter.update(
                    context,
                    &self.battery,
                    &BatteryChargeLimiterArguments::new(true),
                );

                self.battery_contactor.powered_by(&self.battery_bus);
                self.battery.powered_by(&self.battery_contactor);
                self.battery_contactor.or_powered_by(&self.battery);
                self.battery_bus.or_powered_by(&self.battery_contactor);
            }

            fn get_supplied_power(&mut self) -> SuppliedPower {
                let mut supplied_power = SuppliedPower::new();
                supplied_power.add(
                    ElectricalBusType::DirectCurrentBattery,
                    self.battery_bus.output(),
                );

                supplied_power
            }
        }
        impl SimulationElement for TestAircraft {
            fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
                self.battery.accept(visitor);
                self.battery_bus.accept(visitor);
                self.battery_contactor.accept(visitor);
                self.battery_charge_limiter.accept(visitor);
                self.consumer.accept(visitor);

                visitor.visit(self);
            }
        }

        #[test]
        fn should_show_arrow_when_contactor_closed_returns_true_when_15_seconds_have_passed_charging_above_1_a(
        ) {
            let mut aircraft = TestAircraft::with_empty_battery();
            let mut test_bed = BatteryChargeLimiterTestBed::new_with_delta(Duration::from_secs(15));

            aircraft.power_battery_bus();
            aircraft.close_battery_contactor();
            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_show_arrow_when_contactor_closed_returns_false_almost_15_seconds_have_passed_charging_above_1_a(
        ) {
            let mut aircraft = TestAircraft::with_empty_battery();
            let mut test_bed =
                BatteryChargeLimiterTestBed::new_with_delta(Duration::from_millis(14999));

            aircraft.power_battery_bus();
            aircraft.close_battery_contactor();
            test_bed.run_aircraft(&mut aircraft);

            assert!(!test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_show_arrow_when_contactor_closed_returns_false_when_charging_below_1_a() {
            let mut aircraft = TestAircraft::with_full_battery();
            let mut test_bed =
                BatteryChargeLimiterTestBed::new_with_delta(Duration::from_millis(14999));

            aircraft.power_battery_bus();
            aircraft.close_battery_contactor();
            test_bed.run_aircraft(&mut aircraft);

            assert!(!test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_show_arrow_when_contactor_closed_returns_true_when_15_seconds_have_passed_discharging_above_1_a(
        ) {
            let mut aircraft = TestAircraft::with_full_battery();
            let mut test_bed = BatteryChargeLimiterTestBed::new_with_delta(Duration::from_secs(15));

            aircraft.power_demand(Power::new::<watt>(30.));
            aircraft.close_battery_contactor();
            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_show_arrow_when_contactor_closed_returns_false_almost_15_seconds_have_passed_discharging_above_1_a(
        ) {
            let mut aircraft = TestAircraft::with_full_battery();
            let mut test_bed =
                BatteryChargeLimiterTestBed::new_with_delta(Duration::from_millis(14999));

            aircraft.power_demand(Power::new::<watt>(30.));
            aircraft.close_battery_contactor();
            test_bed.run_aircraft(&mut aircraft);

            assert!(!test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_show_arrow_when_contactor_closed_returns_false_when_discharging_below_1_a() {
            let mut aircraft = TestAircraft::with_full_battery();
            let mut test_bed =
                BatteryChargeLimiterTestBed::new_with_delta(Duration::from_millis(14999));

            aircraft.power_demand(Power::new::<watt>(1.));
            aircraft.close_battery_contactor();
            test_bed.run_aircraft(&mut aircraft);

            assert!(!test_bed.should_show_arrow_when_contactor_closed())
        }
    }

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
                Contactor, ElectricalBus, ElectricalBusType,
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

            fn current_is_normal(&mut self, number: usize) -> bool {
                self.test_bed
                    .read_bool(&format!("ELEC_BAT_{}_CURRENT_NORMAL", number))
            }

            fn current(&mut self, number: usize) -> ElectricCurrent {
                ElectricCurrent::new::<ampere>(
                    self.test_bed
                        .read_f64(&format!("ELEC_BAT_{}_CURRENT", number)),
                )
            }

            fn potential_is_normal(&mut self, number: usize) -> bool {
                self.test_bed
                    .read_bool(&format!("ELEC_BAT_{}_POTENTIAL_NORMAL", number))
            }

            fn potential(&mut self, number: usize) -> ElectricPotential {
                ElectricPotential::new::<volt>(
                    self.test_bed
                        .read_f64(&format!("ELEC_BAT_{}_POTENTIAL", number)),
                )
            }
        }

        struct TestAircraft {
            bat_bus: ElectricalBus,
            battery_1: Battery,
            battery_1_contactor: Contactor,
            battery_2: Battery,
            battery_2_contactor: Contactor,
            consumer: PowerConsumer,
            battery_consumption: Power,
            supplied_input_potential: Potential,
        }
        impl TestAircraft {
            fn new(battery_1: Battery, battery_2: Battery) -> Self {
                let mut aircraft = Self {
                    battery_1,
                    battery_2,
                    bat_bus: ElectricalBus::new(ElectricalBusType::DirectCurrentBattery),
                    battery_1_contactor: Contactor::new("BAT1"),
                    battery_2_contactor: Contactor::new("BAT2"),
                    consumer: PowerConsumer::from(ElectricalBusType::DirectCurrentBattery),
                    battery_consumption: Power::new::<watt>(0.),
                    supplied_input_potential: Potential::none(),
                };

                aircraft.battery_1_contactor.close_when(true);

                aircraft
            }

            fn with_full_batteries() -> Self {
                Self::new(Battery::full(1), Battery::full(2))
            }

            fn with_half_charged_batteries() -> Self {
                Self::new(Battery::half(1), Battery::half(2))
            }

            fn with_nearly_empty_batteries() -> Self {
                Self::new(
                    Battery::new(1, ElectricCharge::new::<ampere_hour>(0.001)),
                    Battery::new(2, ElectricCharge::new::<ampere_hour>(0.001)),
                )
            }

            fn with_nearly_empty_dissimilarly_charged_batteries() -> Self {
                Self::new(
                    Battery::new(1, ElectricCharge::new::<ampere_hour>(0.002)),
                    Battery::new(2, ElectricCharge::new::<ampere_hour>(0.001)),
                )
            }

            fn with_empty_batteries() -> Self {
                Self::new(Battery::empty(1), Battery::empty(2))
            }

            fn with_full_and_empty_battery() -> Self {
                Self::new(Battery::full(1), Battery::empty(2))
            }

            fn supply_input_potential(&mut self, potential: ElectricPotential) {
                self.supplied_input_potential =
                    Potential::single(PotentialOrigin::TransformerRectifier(1), potential);
            }

            fn close_battery_2_contactor(&mut self) {
                self.battery_2_contactor.close_when(true);
            }

            fn power_demand(&mut self, power: Power) {
                self.consumer.demand(power);
            }

            fn battery_1_charge(&self) -> ElectricCharge {
                self.battery_1.charge()
            }

            fn battery_2_charge(&self) -> ElectricCharge {
                self.battery_2.charge()
            }

            fn bat_bus_is_powered(&self) -> bool {
                self.bat_bus.is_powered()
            }
        }
        impl Aircraft for TestAircraft {
            fn get_supplied_power(&mut self) -> SuppliedPower {
                let mut supplied_power = SuppliedPower::new();
                supplied_power.add(
                    ElectricalBusType::DirectCurrentBattery,
                    self.bat_bus.output(),
                );

                supplied_power
            }

            fn update_before_power_distribution(&mut self, _: &UpdateContext) {
                self.battery_1_contactor.powered_by(&self.battery_1);
                self.battery_2_contactor.powered_by(&self.battery_2);

                self.bat_bus.powered_by(&self.supplied_input_potential);
                self.bat_bus.or_powered_by_both_batteries(
                    &self.battery_1_contactor,
                    &self.battery_2_contactor,
                );

                self.battery_1_contactor.or_powered_by(&self.bat_bus);
                self.battery_1.powered_by(&self.battery_1_contactor);

                self.battery_2_contactor.or_powered_by(&self.bat_bus);
                self.battery_2.powered_by(&self.battery_2_contactor);
            }
        }
        impl SimulationElement for TestAircraft {
            fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
                self.bat_bus.accept(visitor);
                self.battery_1.accept(visitor);
                self.battery_1_contactor.accept(visitor);
                self.battery_2.accept(visitor);
                self.battery_2_contactor.accept(visitor);
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
        fn when_full_has_potential() {
            let mut aircraft = TestAircraft::with_full_batteries();
            let mut test_bed = BatteryTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.potential(1) > ElectricPotential::new::<volt>(0.));
        }

        #[test]
        fn when_full_potential_is_normal() {
            let mut aircraft = TestAircraft::with_full_batteries();
            let mut test_bed = BatteryTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.potential_is_normal(1));
        }

        #[test]
        fn when_empty_has_no_potential() {
            let mut aircraft = TestAircraft::with_empty_batteries();
            let mut test_bed = BatteryTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert_eq!(test_bed.potential(1), ElectricPotential::new::<volt>(0.));
        }

        #[test]
        fn when_empty_potential_is_abnormal() {
            let mut aircraft = TestAircraft::with_empty_batteries();
            let mut test_bed = BatteryTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(!test_bed.potential_is_normal(1));
        }

        #[test]
        fn when_input_potential_is_greater_than_output_potential_returns_input_potential_for_ecam_and_overhead_indication(
        ) {
            let mut aircraft = TestAircraft::with_half_charged_batteries();
            let mut test_bed = BatteryTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            let input_potential = ElectricPotential::new::<volt>(28.);
            assert!(test_bed.potential(1) < input_potential,
                "This test assumes the battery's potential is lower than the given input potential.");

            aircraft.supply_input_potential(input_potential);
            test_bed.run_aircraft(&mut aircraft);

            assert_eq!(test_bed.potential(1), input_potential);
        }

        #[test]
        fn when_input_potential_is_less_than_output_potential_returns_output_potential_for_ecam_and_overhead_indication(
        ) {
            let mut aircraft = TestAircraft::with_full_batteries();
            let mut test_bed = BatteryTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            let input_potential = ElectricPotential::new::<volt>(26.);
            assert!(input_potential < test_bed.potential(1),
                "This test assumes the battery's potential is higher than the given input potential.");

            aircraft.supply_input_potential(input_potential);
            test_bed.run_aircraft(&mut aircraft);

            assert!(input_potential < test_bed.potential(1));
        }

        #[test]
        fn when_charging_current_is_normal() {
            let mut aircraft = TestAircraft::with_empty_batteries();
            let mut test_bed = BatteryTestBed::new();

            aircraft.supply_input_potential(ElectricPotential::new::<volt>(28.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.current_is_normal(1));
        }

        #[test]
        fn when_charging_battery_current_is_charge_current() {
            let mut aircraft = TestAircraft::with_half_charged_batteries();
            let mut test_bed = BatteryTestBed::new();

            aircraft.supply_input_potential(ElectricPotential::new::<volt>(28.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.current(1) > ElectricCurrent::new::<ampere>(0.));
        }

        #[test]
        fn when_discharging_slowly_current_is_normal() {
            let mut aircraft = TestAircraft::with_full_batteries();
            let mut test_bed = BatteryTestBed::new();

            aircraft.power_demand(Power::new::<watt>(40.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.current_is_normal(1));
        }

        #[test]
        fn when_discharging_quickly_current_is_abnormal() {
            let mut aircraft = TestAircraft::with_full_batteries();
            let mut test_bed = BatteryTestBed::new();

            aircraft.power_demand(Power::new::<watt>(500.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(!test_bed.current_is_normal(1));
        }

        #[test]
        fn when_discharging_battery_current_is_discharge_current() {
            let mut aircraft = TestAircraft::with_full_batteries();
            let mut test_bed = BatteryTestBed::new();

            aircraft.power_demand(Power::new::<watt>(100.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.current(1) < ElectricCurrent::new::<ampere>(0.))
        }

        #[test]
        fn when_discharging_loses_charge() {
            let mut aircraft = TestAircraft::with_full_batteries();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(60));

            let charge_prior_to_run = aircraft.battery_1_charge();

            aircraft.power_demand(Power::new::<watt>(28. * 5.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(aircraft.battery_1_charge() < charge_prior_to_run);
        }

        #[test]
        fn when_charging_gains_charge() {
            let mut aircraft = TestAircraft::with_empty_batteries();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(60));

            let charge_prior_to_run = aircraft.battery_1_charge();

            aircraft.supply_input_potential(ElectricPotential::new::<volt>(28.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(aircraft.battery_1_charge() > charge_prior_to_run);
        }

        #[test]
        fn can_charge_beyond_rated_capacity() {
            let mut aircraft = TestAircraft::with_full_batteries();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(1_000));

            let charge_prior_to_run = aircraft.battery_1_charge();

            aircraft.supply_input_potential(ElectricPotential::new::<volt>(28.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(aircraft.battery_1_charge() > charge_prior_to_run);
        }

        #[test]
        fn does_not_charge_when_input_potential_lower_than_battery_potential() {
            let mut aircraft = TestAircraft::with_half_charged_batteries();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(1_000));

            let charge_prior_to_run = aircraft.battery_1_charge();

            aircraft.supply_input_potential(ElectricPotential::new::<volt>(10.));
            test_bed.run_aircraft(&mut aircraft);

            assert_eq!(aircraft.battery_1_charge(), charge_prior_to_run);
        }

        #[test]
        fn when_neither_charging_nor_discharging_charge_remains_equal() {
            let mut aircraft = TestAircraft::with_half_charged_batteries();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(1_000));

            let charge_prior_to_run = aircraft.battery_1_charge();

            test_bed.run_aircraft(&mut aircraft);

            assert_eq!(aircraft.battery_1_charge(), charge_prior_to_run);
        }

        #[test]
        fn when_neither_charging_nor_discharging_current_is_zero() {
            let mut aircraft = TestAircraft::with_half_charged_batteries();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(1_000));

            test_bed.run_aircraft(&mut aircraft);

            assert_eq!(test_bed.current(1), ElectricCurrent::new::<ampere>(0.));
        }

        #[test]
        fn cannot_discharge_below_zero() {
            let mut aircraft = TestAircraft::with_nearly_empty_batteries();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(50));

            aircraft.power_demand(Power::new::<watt>(5000.));
            test_bed.run_aircraft(&mut aircraft);

            assert_eq!(
                aircraft.battery_1_charge(),
                ElectricCharge::new::<ampere_hour>(0.)
            );
        }

        #[test]
        fn dissimilar_charged_batteries_in_parallel_deplete() {
            let mut aircraft = TestAircraft::with_nearly_empty_dissimilarly_charged_batteries();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(1));

            aircraft.power_demand(Power::new::<watt>(10.));
            aircraft.close_battery_2_contactor();

            for _ in 0..15 {
                test_bed.run_aircraft(&mut aircraft);
            }

            assert!(aircraft.battery_1_charge() < ElectricCharge::new::<ampere_hour>(0.000000001));
            assert!(aircraft.battery_2_charge() < ElectricCharge::new::<ampere_hour>(0.000000001));
            assert!(!aircraft.bat_bus_is_powered());
        }

        #[test]
        fn batteries_charge_each_other_until_relatively_equal_charge() {
            let mut aircraft = TestAircraft::with_full_and_empty_battery();
            let mut test_bed = BatteryTestBed::new_with_delta(Duration::from_secs(120));

            let original_charge = aircraft.battery_1_charge();

            aircraft.close_battery_2_contactor();

            for _ in 0..100 {
                test_bed.run_aircraft(&mut aircraft);
            }

            // For now we assume the batteries are perfect at charging and discharging without any power loss.
            assert!(
                (aircraft.battery_1_charge() - aircraft.battery_2_charge()).abs()
                    < ElectricCharge::new::<ampere_hour>(0.1)
            );
            assert!(
                (aircraft.battery_1_charge() + aircraft.battery_2_charge() - original_charge).abs()
                    < ElectricCharge::new::<ampere_hour>(0.001)
            );
        }
    }
}
