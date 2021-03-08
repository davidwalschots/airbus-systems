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
    both_ac_buses_unpowered: bool,
    battery_potential: ElectricPotential,
    battery_current: ElectricCurrent,
    battery_bus_potential: ElectricPotential,
    apu_master_sw_pb_on: bool,
    apu_start_sw_pb_on: bool,
}
impl BatteryChargeLimiterArguments {
    pub fn new<TBat: PotentialSource + ProvideCurrent, TBatBus: PotentialSource>(
        ac_buses_unpowered: bool,
        battery: &TBat,
        battery_bus: &TBatBus,
        apu_master_sw_pb_on: bool,
        apu_start_sw_pb_on: bool,
    ) -> Self {
        Self {
            both_ac_buses_unpowered: ac_buses_unpowered,
            battery_potential: battery.output().raw(),
            battery_current: battery.current(),
            battery_bus_potential: battery_bus.output().raw(),
            apu_master_sw_pb_on,
            apu_start_sw_pb_on,
        }
    }

    fn both_ac_buses_unpowered(&self) -> bool {
        self.both_ac_buses_unpowered
    }

    fn battery_potential(&self) -> ElectricPotential {
        self.battery_potential
    }

    fn battery_current(&self) -> ElectricCurrent {
        self.battery_current
    }

    fn battery_bus_potential(&self) -> ElectricPotential {
        self.battery_bus_potential
    }

    fn apu_master_sw_pb_on(&self) -> bool {
        self.apu_master_sw_pb_on
    }

    fn apu_start_sw_pb_on(&self) -> bool {
        self.apu_start_sw_pb_on
    }
}

pub struct BatteryChargeLimiter {
    should_show_arrow_when_contactor_closed_id: String,
    should_close_contactor: bool,
    discharging_above_1_ampere_beyond_time: DelayedTrueLogicGate,
    charging_above_1_ampere_beyond_time: DelayedTrueLogicGate,
    begin_charging_cycle_delay: DelayedTrueLogicGate,
    below_4_ampere_charging_duration: Duration,
    had_apu_start: bool,
}
impl BatteryChargeLimiter {
    const CHARGE_BATTERY_BELOW_VOLTAGE: f64 = 26.5;
    const BATTERY_BUS_BELOW_CHARGING_VOLTAGE: f64 = 27.;
    const BATTERY_CHARGING_CLOSE_DELAY_MILLISECONDS: u64 = 225;
    const BATTERY_CHARGING_OPEN_DELAY_ON_GROUND_SECONDS: u64 = 10;
    const BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS: u64 = 1800;
    const CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS: u64 = 15;

    pub fn new(contactor_id: &str) -> Self {
        Self {
            should_show_arrow_when_contactor_closed_id: format!(
                "ELEC_CONTACTOR_{}_SHOW_ARROW_WHEN_CLOSED",
                contactor_id
            ),
            should_close_contactor: false,
            discharging_above_1_ampere_beyond_time: DelayedTrueLogicGate::new(Duration::from_secs(
                BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
            )),
            charging_above_1_ampere_beyond_time: DelayedTrueLogicGate::new(Duration::from_secs(
                BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
            )),
            begin_charging_cycle_delay: DelayedTrueLogicGate::new(Duration::from_millis(
                BatteryChargeLimiter::BATTERY_CHARGING_CLOSE_DELAY_MILLISECONDS,
            )),
            below_4_ampere_charging_duration: Duration::from_secs(0),
            had_apu_start: false,
        }
    }

    pub fn update(&mut self, context: &UpdateContext, arguments: &BatteryChargeLimiterArguments) {
        self.update_battery_to_battery_bus_arrow(context, arguments);

        if arguments.apu_start_sw_pb_on() {
            self.had_apu_start = true;
        }

        if arguments.battery_current() < ElectricCurrent::new::<ampere>(4.) {
            self.below_4_ampere_charging_duration += context.delta();
        } else {
            self.below_4_ampere_charging_duration = Duration::from_secs(0);
        }

        self.should_close_contactor = if !self.should_close_contactor {
            self.determine_if_should_close_contactor(context, arguments)
        } else {
            !self.determine_if_should_open_contactor(context, arguments)
        };

        if !self.should_close_contactor {
            // The moment we open the contactor again we no longer care about having had a start or not.
            self.had_apu_start = false;
        }
    }

    fn update_battery_to_battery_bus_arrow(
        &mut self,
        context: &UpdateContext,
        arguments: &BatteryChargeLimiterArguments,
    ) {
        self.discharging_above_1_ampere_beyond_time.update(
            context,
            arguments.battery_current() < ElectricCurrent::new::<ampere>(-1.),
        );
        self.charging_above_1_ampere_beyond_time.update(
            context,
            arguments.battery_current() > ElectricCurrent::new::<ampere>(1.),
        );
    }

    fn determine_if_should_close_contactor(
        &mut self,
        context: &UpdateContext,
        arguments: &BatteryChargeLimiterArguments,
    ) -> bool {
        if arguments.apu_master_sw_pb_on() {
            return true;
        }

        if arguments.both_ac_buses_unpowered()
            && context.is_on_ground()
            && context.indicated_airspeed() < Velocity::new::<knot>(100.)
        {
            return true;
        }

        self.begin_charging_cycle_delay.update(
            context,
            arguments.battery_potential()
                < ElectricPotential::new::<volt>(
                    BatteryChargeLimiter::CHARGE_BATTERY_BELOW_VOLTAGE,
                )
                && arguments.battery_bus_potential()
                    > ElectricPotential::new::<volt>(
                        BatteryChargeLimiter::BATTERY_BUS_BELOW_CHARGING_VOLTAGE,
                    ),
        );

        self.begin_charging_cycle_delay.output()
    }

    fn determine_if_should_open_contactor(
        &mut self,
        context: &UpdateContext,
        arguments: &BatteryChargeLimiterArguments,
    ) -> bool {
        !arguments.apu_master_sw_pb_on
            && (self.beyond_charge_duration_on_ground_without_apu_start(context)
                || self.beyond_charge_duration_above_100_knots_or_after_apu_start(context))
    }

    fn beyond_charge_duration_on_ground_without_apu_start(&self, context: &UpdateContext) -> bool {
        (!self.had_apu_start && context.is_on_ground())
            && self.below_4_ampere_charging_duration
                >= Duration::from_secs(
                    BatteryChargeLimiter::BATTERY_CHARGING_OPEN_DELAY_ON_GROUND_SECONDS,
                )
    }

    fn beyond_charge_duration_above_100_knots_or_after_apu_start(
        &self,
        context: &UpdateContext,
    ) -> bool {
        (context.indicated_airspeed() >= Velocity::new::<knot>(100.) || self.had_apu_start)
            && self.below_4_ampere_charging_duration
                >= Duration::from_secs(
                    BatteryChargeLimiter::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS,
                )
    }

    pub fn should_close_contactor(&self) -> bool {
        self.should_close_contactor
    }
}
impl SimulationElement for BatteryChargeLimiter {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_bool(
            &self.should_show_arrow_when_contactor_closed_id,
            self.discharging_above_1_ampere_beyond_time.output()
                || self.charging_above_1_ampere_beyond_time.output(),
        );
    }
}

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

    #[cfg(test)]
    fn set_full_charge(&mut self) {
        self.charge = ElectricCharge::new::<ampere_hour>(Battery::RATED_CAPACITY_AMPERE_HOURS);
        self.output_potential = Battery::calculate_output_potential_for_charge(self.charge);
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
        use uom::si::{length::foot, power::watt};

        struct BatteryChargeLimiterTestBed {
            test_bed: SimulationTestBed,
            aircraft: TestAircraft,
        }
        impl BatteryChargeLimiterTestBed {
            fn new() -> Self {
                Self {
                    test_bed: SimulationTestBed::new(),
                    aircraft: TestAircraft::new(Battery::half(1)),
                }
            }

            fn on_the_ground(mut self) -> Self {
                self.test_bed.set_on_ground(true);
                self.test_bed
                    .set_indicated_altitude(Length::new::<foot>(0.));

                self
            }

            fn indicated_airspeed_of(mut self, indicated_airspeed: Velocity) -> Self {
                self.test_bed.set_indicated_airspeed(indicated_airspeed);
                self
            }

            fn run(mut self, delta: Duration) -> Self {
                // The battery's current is updated after the BCL, thus we need two ticks.
                self.test_bed.set_delta(Duration::from_secs(0));
                self.test_bed.run_aircraft(&mut self.aircraft);

                self.test_bed.set_delta(delta);
                self.test_bed.run_aircraft(&mut self.aircraft);

                self
            }

            fn wait_for_closed_contactor(mut self) -> Self {
                self.aircraft.set_battery_bus_at_minimum_charging_voltage();
                self = self.run(Duration::from_millis(
                    BatteryChargeLimiter::BATTERY_CHARGING_CLOSE_DELAY_MILLISECONDS,
                ));

                assert!(
                    self.aircraft.battery_contactor_is_closed(),
                    "Battery contactor didn't close within the expected time frame. Is the battery bus at a high enough voltage and the battery not full?"
                );

                self
            }

            fn started_apu(mut self) -> Self {
                self.aircraft.set_apu_master_sw_pb_on();
                self.aircraft.set_apu_start_pb_on();

                self = self.run(Duration::from_secs(0));

                self.aircraft.set_apu_start_pb_off();

                self
            }

            fn stopped_apu(mut self) -> Self {
                self.aircraft.set_apu_master_sw_pb_off();
                self = self.run(Duration::from_secs(0));
                self
            }

            fn then_continue_with(self) -> Self {
                self
            }

            fn and(self) -> Self {
                self
            }

            fn full_battery_charge(mut self) -> Self {
                self.aircraft.set_full_battery_charge();
                self
            }

            fn no_power_outside_of_battery(mut self) -> Self {
                self.aircraft.set_battery_bus_unpowered();
                self.aircraft.set_both_ac_buses_unpowered();
                self
            }

            fn power_demand_of(mut self, power: Power) -> Self {
                self.aircraft.set_power_demand(power);
                self
            }

            fn battery_bus_at_minimum_charging_voltage(mut self) -> Self {
                self.aircraft.set_battery_bus_at_minimum_charging_voltage();
                self
            }

            fn battery_bus_below_minimum_charging_voltage(mut self) -> Self {
                self.aircraft
                    .set_battery_bus_below_minimum_charging_voltage();
                self
            }

            fn current(&mut self) -> ElectricCurrent {
                ElectricCurrent::new::<ampere>(
                    self.test_bed.read_f64(&format!("ELEC_BAT_{}_CURRENT", 1)),
                )
            }

            fn battery_contactor_is_closed(&self) -> bool {
                self.aircraft.battery_contactor_is_closed()
            }

            fn apu_master_sw_on(mut self) -> Self {
                self.aircraft.set_apu_master_sw_pb_on();
                self
            }

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
            both_ac_buses_unpowered: bool,
            apu_master_sw_pb_on: bool,
            apu_start_pb_on: bool,
        }
        impl TestAircraft {
            fn new(battery: Battery) -> Self {
                Self {
                    battery: battery,
                    battery_charge_limiter: BatteryChargeLimiter::new("TEST"),
                    battery_bus: ElectricalBus::new(ElectricalBusType::DirectCurrentBattery),
                    battery_contactor: Contactor::new("TEST"),
                    consumer: PowerConsumer::from(ElectricalBusType::DirectCurrentBattery),
                    both_ac_buses_unpowered: false,
                    apu_master_sw_pb_on: false,
                    apu_start_pb_on: false,
                }
            }

            fn set_full_battery_charge(&mut self) {
                self.battery.set_full_charge()
            }

            fn set_battery_bus_at_minimum_charging_voltage(&mut self) {
                self.battery_bus.powered_by(&Potential::single(
                    PotentialOrigin::TransformerRectifier(1),
                    ElectricPotential::new::<volt>(
                        BatteryChargeLimiter::BATTERY_BUS_BELOW_CHARGING_VOLTAGE + 0.000001,
                    ),
                ));
            }

            fn set_battery_bus_below_minimum_charging_voltage(&mut self) {
                self.battery_bus.powered_by(&Potential::single(
                    PotentialOrigin::TransformerRectifier(1),
                    ElectricPotential::new::<volt>(
                        BatteryChargeLimiter::BATTERY_BUS_BELOW_CHARGING_VOLTAGE,
                    ),
                ));
            }

            fn set_battery_bus_unpowered(&mut self) {
                self.battery_bus.powered_by(&Potential::none());
            }

            fn set_both_ac_buses_unpowered(&mut self) {
                self.both_ac_buses_unpowered = true;
            }

            fn set_apu_master_sw_pb_on(&mut self) {
                self.apu_master_sw_pb_on = true;
            }

            fn set_apu_master_sw_pb_off(&mut self) {
                self.apu_master_sw_pb_on = false;
            }

            fn set_apu_start_pb_on(&mut self) {
                self.apu_start_pb_on = true;
            }

            fn set_apu_start_pb_off(&mut self) {
                self.apu_start_pb_on = false;
            }

            fn set_power_demand(&mut self, power: Power) {
                self.consumer.demand(power);
            }

            fn battery_contactor_is_closed(&self) -> bool {
                self.battery_contactor.is_closed()
            }
        }
        impl Aircraft for TestAircraft {
            fn update_before_power_distribution(&mut self, context: &UpdateContext) {
                self.battery_charge_limiter.update(
                    context,
                    &BatteryChargeLimiterArguments::new(
                        self.both_ac_buses_unpowered,
                        &self.battery,
                        &self.battery_bus,
                        self.apu_master_sw_pb_on,
                        self.apu_start_pb_on,
                    ),
                );

                self.battery_contactor
                    .close_when(self.battery_charge_limiter.should_close_contactor());

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

        fn test_bed() -> BatteryChargeLimiterTestBed {
            BatteryChargeLimiterTestBed::new()
        }

        fn test_bed_with() -> BatteryChargeLimiterTestBed {
            test_bed()
        }

        #[test]
        fn should_show_arrow_when_contactor_closed_while_15_seconds_have_passed_charging_above_1_a()
        {
            let mut test_bed = test_bed()
                .wait_for_closed_contactor()
                .run(Duration::from_secs(
                    BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
                ));

            assert!(test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_not_show_arrow_when_contactor_closed_while_almost_15_seconds_have_passed_charging_above_1_a(
        ) {
            let mut test_bed = test_bed()
                .wait_for_closed_contactor()
                .run(Duration::from_secs_f64(
                    BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS as f64
                        - 0.0001,
                ));

            assert!(!test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_not_show_arrow_when_contactor_closed_while_charging_below_1_a() {
            let mut test_bed = test_bed()
                .wait_for_closed_contactor()
                .then_continue_with()
                .full_battery_charge()
                .run(Duration::from_secs(
                    BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
                ));

            assert!(!test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_show_arrow_when_contactor_closed_while_15_seconds_have_passed_discharging_above_1_a(
        ) {
            let mut test_bed = test_bed()
                .wait_for_closed_contactor()
                .then_continue_with()
                .no_power_outside_of_battery()
                .and()
                .power_demand_of(Power::new::<watt>(50.))
                .run(Duration::from_secs(
                    BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
                ));

            assert!(test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_not_show_arrow_when_contactor_closed_while_almost_15_seconds_have_passed_discharging_above_1_a(
        ) {
            let mut test_bed = test_bed()
                .wait_for_closed_contactor()
                .then_continue_with()
                .no_power_outside_of_battery()
                .and()
                .power_demand_of(Power::new::<watt>(30.))
                .run(Duration::from_secs_f64(
                    BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS as f64
                        - 0.0001,
                ));

            assert!(!test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_not_show_arrow_when_contactor_closed_while_discharging_below_1_a() {
            let mut test_bed = test_bed()
                .wait_for_closed_contactor()
                .then_continue_with()
                .no_power_outside_of_battery()
                .and()
                .power_demand_of(Power::new::<watt>(1.))
                .run(Duration::from_secs(
                    BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
                ));

            assert!(!test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn contactor_closed_when_battery_voltage_below_charge_threshold_and_battery_bus_above_threshold_for_greater_than_225ms(
        ) {
            let test_bed = test_bed_with()
                .battery_bus_at_minimum_charging_voltage()
                .run(Duration::from_millis(
                    BatteryChargeLimiter::BATTERY_CHARGING_CLOSE_DELAY_MILLISECONDS,
                ));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_not_closed_when_battery_voltage_below_charge_threshold_and_battery_bus_above_threshold_for_less_than_225ms(
        ) {
            let test_bed = test_bed_with()
                .battery_bus_at_minimum_charging_voltage()
                .run(Duration::from_millis(
                    BatteryChargeLimiter::BATTERY_CHARGING_CLOSE_DELAY_MILLISECONDS - 1,
                ));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_not_closed_when_battery_voltage_above_charge_threshold() {
            let test_bed = test_bed_with()
                .full_battery_charge()
                .and()
                .battery_bus_at_minimum_charging_voltage()
                .run(Duration::from_secs(10));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_not_closed_when_battery_bus_voltage_below_threshold() {
            let test_bed = test_bed_with()
                .battery_bus_below_minimum_charging_voltage()
                .run(Duration::from_secs(10));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_closed_when_bat_only_on_ground_at_or_below_100_knots() {
            let test_bed = test_bed_with()
                .full_battery_charge()
                .on_the_ground()
                .indicated_airspeed_of(Velocity::new::<knot>(99.9))
                .and()
                .no_power_outside_of_battery()
                .run(Duration::from_millis(1));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_open_when_bat_only_on_ground_at_or_above_100_knots() {
            let test_bed = test_bed_with()
                .full_battery_charge()
                .on_the_ground()
                .indicated_airspeed_of(Velocity::new::<knot>(100.))
                .and()
                .no_power_outside_of_battery()
                .run(Duration::from_millis(1));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_open_when_not_bat_only_on_ground_below_100_knots() {
            let test_bed = test_bed_with()
                .battery_bus_at_minimum_charging_voltage()
                .indicated_airspeed_of(Velocity::new::<knot>(99.9))
                .and()
                .on_the_ground()
                .run(Duration::from_millis(1));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_closed_when_apu_master_sw_pb_is_on() {
            let test_bed = test_bed_with()
                .full_battery_charge()
                .and()
                .apu_master_sw_on()
                .run(Duration::from_millis(1));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn charging_cycle_on_ground_ends_10_seconds_after_current_less_than_4_ampere() {
            let mut test_bed = test_bed_with()
                .indicated_airspeed_of(Velocity::new::<knot>(0.))
                .and()
                .on_the_ground()
                .wait_for_closed_contactor();

            assert!(test_bed.current() >= ElectricCurrent::new::<ampere>(4.), "The test assumes that charging current is equal to or greater than 4 at this point.");

            test_bed =
                test_bed
                    .then_continue_with()
                    .full_battery_charge()
                    .run(Duration::from_secs(
                        BatteryChargeLimiter::BATTERY_CHARGING_OPEN_DELAY_ON_GROUND_SECONDS,
                    ));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn charging_cycle_on_ground_does_not_end_within_10_seconds_after_current_less_than_4_ampere(
        ) {
            let mut test_bed = test_bed_with()
                .indicated_airspeed_of(Velocity::new::<knot>(0.))
                .and()
                .on_the_ground()
                .wait_for_closed_contactor();

            assert!(test_bed.current() >= ElectricCurrent::new::<ampere>(4.), "The test assumes that charging current is equal to or greater than 4 at this point.");

            test_bed =
                test_bed
                    .then_continue_with()
                    .full_battery_charge()
                    .run(Duration::from_secs_f64(
                        BatteryChargeLimiter::BATTERY_CHARGING_OPEN_DELAY_ON_GROUND_SECONDS as f64
                            - 0.001,
                    ));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn charging_cycle_does_not_end_when_apu_master_sw_on() {
            let mut test_bed = test_bed_with().wait_for_closed_contactor();

            assert!(test_bed.current() >= ElectricCurrent::new::<ampere>(4.), "The test assumes that charging current is equal to or greater than 4 at this point.");

            test_bed = test_bed
                .then_continue_with()
                .full_battery_charge()
                .and()
                .apu_master_sw_on()
                .run(Duration::from_secs(
                    BatteryChargeLimiter::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS,
                ));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn when_above_100_knots_the_charging_cycle_ends_after_30_minutes_below_4_ampere() {
            let mut test_bed = test_bed().wait_for_closed_contactor();

            assert!(test_bed.current() >= ElectricCurrent::new::<ampere>(4.), "The test assumes that charging current is equal to or greater than 4 at this point.");

            test_bed =
                test_bed
                    .then_continue_with()
                    .full_battery_charge()
                    .run(Duration::from_secs(
                        BatteryChargeLimiter::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS,
                    ));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn when_above_100_knots_the_charging_cycle_does_not_end_within_30_minutes_before_below_4_ampere(
        ) {
            let mut test_bed = test_bed().wait_for_closed_contactor();

            assert!(test_bed.current() >= ElectricCurrent::new::<ampere>(4.), "The test assumes that charging current is equal to or greater than 4 at this point.");

            test_bed =
                test_bed
                    .then_continue_with()
                    .full_battery_charge()
                    .run(Duration::from_secs_f64(
                        BatteryChargeLimiter::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS as f64
                            - 0.0001,
                    ));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn when_apu_started_the_charging_cycle_ends_30_minutes_after_below_4_ampere() {
            let test_bed = test_bed_with()
                .full_battery_charge()
                .on_the_ground()
                .indicated_airspeed_of(Velocity::new::<knot>(0.))
                .and()
                .started_apu()
                .then_continue_with()
                .stopped_apu()
                .run(Duration::from_secs(
                    BatteryChargeLimiter::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS,
                ));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn when_apu_started_the_charging_cycle_does_not_end_within_30_minutes_before_below_4_ampere(
        ) {
            let test_bed = test_bed_with()
            .full_battery_charge()
            .on_the_ground()
            .indicated_airspeed_of(Velocity::new::<knot>(0.))
            .and()
            .started_apu()
            .then_continue_with()
            .stopped_apu()
            .run(Duration::from_secs_f64(
                BatteryChargeLimiter::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS as f64 - 0.0001,
            ));

            assert!(test_bed.battery_contactor_is_closed());
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
