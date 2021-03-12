use std::time::Duration;

use super::{PotentialSource, ProvideCurrent};
use crate::{
    shared::DelayedTrueLogicGate,
    simulation::{SimulationElement, SimulatorWriter, UpdateContext},
};
use uom::si::{electric_current::ampere, electric_potential::volt, f64::*, velocity::knot};

pub struct BatteryChargeLimiterArguments {
    ac_buses_unpowered: bool,
    battery_potential: ElectricPotential,
    battery_current: ElectricCurrent,
    battery_bus_potential: ElectricPotential,
    apu_master_sw_pb_on: bool,
    apu_start_sw_pb_on: bool,
    apu_available: bool,
    battery_push_button_is_auto: bool,
    landing_gear_is_up_and_locked: bool,
    emergency_generator_available: bool,
}
impl BatteryChargeLimiterArguments {
    #[allow(clippy::too_many_arguments)]
    pub fn new<TBat: PotentialSource + ProvideCurrent, TBatBus: PotentialSource>(
        ac_buses_unpowered: bool,
        battery: &TBat,
        battery_bus: &TBatBus,
        apu_master_sw_pb_on: bool,
        apu_start_sw_pb_on: bool,
        apu_available: bool,
        battery_push_button_is_auto: bool,
        landing_gear_is_up_and_locked: bool,
        emergency_generator_available: bool,
    ) -> Self {
        Self {
            ac_buses_unpowered,
            battery_potential: battery.output().raw(),
            battery_current: battery.current(),
            battery_bus_potential: battery_bus.output().raw(),
            apu_master_sw_pb_on,
            apu_start_sw_pb_on,
            apu_available,
            battery_push_button_is_auto,
            landing_gear_is_up_and_locked,
            emergency_generator_available,
        }
    }

    fn ac_buses_unpowered(&self) -> bool {
        self.ac_buses_unpowered
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

    fn apu_available(&self) -> bool {
        self.apu_available
    }

    fn battery_push_button_is_auto(&self) -> bool {
        self.battery_push_button_is_auto
    }

    fn landing_gear_is_up_and_locked(&self) -> bool {
        self.landing_gear_is_up_and_locked
    }

    fn emergency_generator_available(&self) -> bool {
        self.emergency_generator_available
    }
}

pub struct BatteryChargeLimiter {
    should_show_arrow_when_contactor_closed_id: String,
    arrow: ArrowBetweenBatteryAndBatBus,
    observer: Option<Box<dyn BatteryStateObserver>>,
}
impl BatteryChargeLimiter {
    const CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS: u64 = 15;

    pub fn new(contactor_id: &str) -> Self {
        Self {
            should_show_arrow_when_contactor_closed_id: format!(
                "ELEC_CONTACTOR_{}_SHOW_ARROW_WHEN_CLOSED",
                contactor_id
            ),
            arrow: ArrowBetweenBatteryAndBatBus::new(),
            observer: Some(Box::new(OpenContactorObserver::from_off())),
        }
    }

    pub fn update(&mut self, context: &UpdateContext, arguments: &BatteryChargeLimiterArguments) {
        self.arrow.update(context, arguments);

        if let Some(observer) = self.observer.take() {
            self.observer = Some(observer.update(context, arguments));
        }
    }

    pub fn should_close_contactor(&self) -> bool {
        self.observer.as_ref().unwrap().should_close_contactor()
    }
}
impl SimulationElement for BatteryChargeLimiter {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_bool(
            &self.should_show_arrow_when_contactor_closed_id,
            self.arrow.should_show_when_contactor_closed(),
        );
    }
}

/// Observes the battery, battery contactor and related systems
/// to determine if the battery contactor should open or close.
trait BatteryStateObserver {
    fn should_close_contactor(&self) -> bool;
    fn update(
        self: Box<Self>,
        context: &UpdateContext,
        arguments: &BatteryChargeLimiterArguments,
    ) -> Box<dyn BatteryStateObserver>;
}

/// The BCL is not powered when the BAT push button is in the OFF
/// position. This observer simply watches for the BAT push button to
/// move to the AUTO position.
struct OffPushButtonObserver {}
impl OffPushButtonObserver {
    fn new() -> Self {
        Self {}
    }
}
impl BatteryStateObserver for OffPushButtonObserver {
    fn should_close_contactor(&self) -> bool {
        false
    }

    fn update(
        self: Box<Self>,
        _: &UpdateContext,
        arguments: &BatteryChargeLimiterArguments,
    ) -> Box<dyn BatteryStateObserver> {
        if arguments.battery_push_button_is_auto() {
            Box::new(OpenContactorObserver::from_off())
        } else {
            self
        }
    }
}

/// Observes the open battery contactor and related systems
/// to determine if the battery contactor should be closed.
struct OpenContactorObserver {
    begin_charging_cycle_delay: DelayedTrueLogicGate,
    open_due_to_discharge_protection: bool,
    open_due_to_exceeding_emergency_elec_closing_time_allowance: bool,
    emergency_elec: EmergencyElec,
}
impl OpenContactorObserver {
    const CHARGE_BATTERY_BELOW_VOLTAGE: f64 = 26.5;
    const BATTERY_BUS_BELOW_CHARGING_VOLTAGE: f64 = 27.;
    const BATTERY_CHARGING_CLOSE_DELAY_MILLISECONDS: u64 = 225;

    fn new(
        emergency_elec: EmergencyElec,
        open_due_to_discharge_protection: bool,
        open_due_to_exceeding_emergency_elec_closing_time_allowance: bool,
    ) -> Self {
        Self {
            begin_charging_cycle_delay: DelayedTrueLogicGate::new(Duration::from_millis(
                OpenContactorObserver::BATTERY_CHARGING_CLOSE_DELAY_MILLISECONDS,
            )),
            emergency_elec,
            open_due_to_discharge_protection,
            open_due_to_exceeding_emergency_elec_closing_time_allowance,
        }
    }

    fn from_off() -> Self {
        Self::new(EmergencyElec::new(), false, false)
    }

    fn from_closed(emergency_elec: EmergencyElec) -> Self {
        Self::new(emergency_elec, false, false)
    }

    fn with_discharge_protection(emergency_elec: EmergencyElec) -> Self {
        Self::new(emergency_elec, true, false)
    }

    fn after_exceeding_emergency_elec_closing_time_allowance(
        emergency_elec: EmergencyElec,
    ) -> Self {
        Self::new(emergency_elec, false, true)
    }

    fn update_state(&mut self, context: &UpdateContext, arguments: &BatteryChargeLimiterArguments) {
        self.emergency_elec.update(context, arguments);
        self.update_begin_charging_cycle_delay(context, arguments);

        if self.open_due_to_exceeding_emergency_elec_closing_time_allowance
            && !arguments.apu_master_sw_pb_on()
        {
            self.open_due_to_exceeding_emergency_elec_closing_time_allowance = false;
        }
    }

    fn should_close(
        &self,
        context: &UpdateContext,
        arguments: &BatteryChargeLimiterArguments,
    ) -> bool {
        !self.open_due_to_exceeding_emergency_elec_closing_time_allowance
            && !self.emergency_elec_inhibited(arguments)
            && !self.open_due_to_discharge_protection
            && (self.should_get_ready_for_apu_start(arguments)
                || on_ground_at_low_speed_with_unpowered_ac_buses(context, arguments)
                || self.should_charge_battery())
    }

    fn should_get_ready_for_apu_start(&self, arguments: &BatteryChargeLimiterArguments) -> bool {
        arguments.apu_master_sw_pb_on() && !arguments.apu_available()
    }

    fn should_charge_battery(&self) -> bool {
        self.begin_charging_cycle_delay.output()
    }

    fn emergency_elec_inhibited(&self, arguments: &BatteryChargeLimiterArguments) -> bool {
        self.emergency_elec.is_active()
            && (!arguments.landing_gear_is_up_and_locked()
                || (!arguments.emergency_generator_available()
                    && self.emergency_elec.apu_start_inhibited()))
    }

    fn update_begin_charging_cycle_delay(
        &mut self,
        context: &UpdateContext,
        arguments: &BatteryChargeLimiterArguments,
    ) {
        self.begin_charging_cycle_delay.update(
            context,
            arguments.battery_potential()
                < ElectricPotential::new::<volt>(
                    OpenContactorObserver::CHARGE_BATTERY_BELOW_VOLTAGE,
                )
                && arguments.battery_bus_potential()
                    > ElectricPotential::new::<volt>(
                        OpenContactorObserver::BATTERY_BUS_BELOW_CHARGING_VOLTAGE,
                    ),
        );
    }
}
impl BatteryStateObserver for OpenContactorObserver {
    fn should_close_contactor(&self) -> bool {
        false
    }

    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        arguments: &BatteryChargeLimiterArguments,
    ) -> Box<dyn BatteryStateObserver> {
        self.update_state(context, arguments);

        if !arguments.battery_push_button_is_auto() {
            Box::new(OffPushButtonObserver::new())
        } else if self.should_close(context, arguments) {
            Box::new(ClosedContactorObserver::from_open(self.emergency_elec))
        } else {
            self
        }
    }
}

/// Observes the closed battery contactor and related systems
/// to determine if the battery contactor should be opened.
struct ClosedContactorObserver {
    below_4_ampere_charging_duration: Duration,
    below_23_volt_duration: Duration,
    apu_master_sw_pb_on_duration: Duration,
    emergency_elec: EmergencyElec,
    had_apu_start: bool,
    entered_in_emergency_elec: bool,
}
impl ClosedContactorObserver {
    const BATTERY_CHARGING_OPEN_DELAY_ON_GROUND_SECONDS: u64 = 10;
    const BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS: u64 = 1800;
    const BATTERY_DISCHARGE_PROTECTION_DELAY_SECONDS: u64 = 15;
    const EMER_ELEC_APU_MASTER_MAXIMUM_CLOSED_SECONDS: u64 = 180;

    fn from_open(emergency_elec: EmergencyElec) -> Self {
        Self {
            below_4_ampere_charging_duration: Duration::from_secs(0),
            below_23_volt_duration: Duration::from_secs(0),
            apu_master_sw_pb_on_duration: Duration::from_secs(0),
            entered_in_emergency_elec: emergency_elec.is_active(),
            emergency_elec,
            had_apu_start: false,
        }
    }

    fn update_state(&mut self, context: &UpdateContext, arguments: &BatteryChargeLimiterArguments) {
        self.emergency_elec.update(context, arguments);

        if arguments.apu_start_sw_pb_on() {
            self.had_apu_start = true;
        }

        if arguments.battery_current() < ElectricCurrent::new::<ampere>(4.) {
            self.below_4_ampere_charging_duration += context.delta();
        } else {
            self.below_4_ampere_charging_duration = Duration::from_secs(0);
        }

        if arguments.battery_potential() < ElectricPotential::new::<volt>(23.) {
            self.below_23_volt_duration += context.delta();
        } else {
            self.below_23_volt_duration = Duration::from_secs(0);
        }

        if arguments.apu_master_sw_pb_on() {
            self.apu_master_sw_pb_on_duration += context.delta();
        } else {
            self.apu_master_sw_pb_on_duration = Duration::from_secs(0);
        }
    }

    fn should_open_due_to_discharge_protection(&self, context: &UpdateContext) -> bool {
        context.is_on_ground()
            && self.below_23_volt_duration
                >= Duration::from_secs(
                    ClosedContactorObserver::BATTERY_DISCHARGE_PROTECTION_DELAY_SECONDS,
                )
    }

    fn should_open_due_to_exceeding_emergency_elec_closed_time_allowance(&self) -> bool {
        self.emergency_elec.is_active() && self.beyond_emergency_elec_closed_time_allowance()
    }

    fn should_open(
        &self,
        context: &UpdateContext,
        arguments: &BatteryChargeLimiterArguments,
    ) -> bool {
        if self.emergency_elec.is_active() {
            !arguments.apu_master_sw_pb_on() || self.emergency_elec_inhibited(arguments)
        } else {
            !self.awaiting_apu_start(arguments)
                && !on_ground_at_low_speed_with_unpowered_ac_buses(context, arguments)
                && (self.beyond_charge_duration_on_ground_without_apu_start(context)
                    || self
                        .beyond_charge_duration_above_100_knots_or_after_apu_start_attempt(context))
        }
    }

    fn emergency_elec_inhibited(&self, arguments: &BatteryChargeLimiterArguments) -> bool {
        !self.entered_in_emergency_elec || !arguments.landing_gear_is_up_and_locked()
    }

    fn beyond_emergency_elec_closed_time_allowance(&self) -> bool {
        self.apu_master_sw_pb_on_duration
            >= Duration::from_secs(
                ClosedContactorObserver::EMER_ELEC_APU_MASTER_MAXIMUM_CLOSED_SECONDS,
            )
    }

    fn awaiting_apu_start(&self, arguments: &BatteryChargeLimiterArguments) -> bool {
        arguments.apu_master_sw_pb_on() && !arguments.apu_available()
    }

    fn beyond_charge_duration_on_ground_without_apu_start(&self, context: &UpdateContext) -> bool {
        (!self.had_apu_start && context.is_on_ground())
            && self.below_4_ampere_charging_duration
                >= Duration::from_secs(
                    ClosedContactorObserver::BATTERY_CHARGING_OPEN_DELAY_ON_GROUND_SECONDS,
                )
    }

    fn beyond_charge_duration_above_100_knots_or_after_apu_start_attempt(
        &self,
        context: &UpdateContext,
    ) -> bool {
        (context.indicated_airspeed() >= Velocity::new::<knot>(100.) || self.had_apu_start)
            && self.below_4_ampere_charging_duration
                >= Duration::from_secs(
                    ClosedContactorObserver::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS,
                )
    }
}
impl BatteryStateObserver for ClosedContactorObserver {
    fn should_close_contactor(&self) -> bool {
        true
    }

    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        arguments: &BatteryChargeLimiterArguments,
    ) -> Box<dyn BatteryStateObserver> {
        self.update_state(context, arguments);

        if !arguments.battery_push_button_is_auto() {
            Box::new(OffPushButtonObserver::new())
        } else if self.should_open_due_to_discharge_protection(context) {
            Box::new(OpenContactorObserver::with_discharge_protection(
                self.emergency_elec,
            ))
        } else if self.should_open_due_to_exceeding_emergency_elec_closed_time_allowance() {
            Box::new(
                OpenContactorObserver::after_exceeding_emergency_elec_closing_time_allowance(
                    self.emergency_elec,
                ),
            )
        } else if self.should_open(context, arguments) {
            Box::new(OpenContactorObserver::from_closed(self.emergency_elec))
        } else {
            self
        }
    }
}

fn on_ground_at_low_speed_with_unpowered_ac_buses(
    context: &UpdateContext,
    arguments: &BatteryChargeLimiterArguments,
) -> bool {
    arguments.ac_buses_unpowered()
        && context.is_on_ground()
        && context.indicated_airspeed() < Velocity::new::<knot>(100.)
}

struct EmergencyElec {
    is_active_for_duration: Duration,
}
impl EmergencyElec {
    const APU_START_INHIBIT_DELAY_SECONDS: u64 = 45;

    fn new() -> Self {
        Self {
            is_active_for_duration: Duration::from_secs(0),
        }
    }

    fn update(&mut self, context: &UpdateContext, arguments: &BatteryChargeLimiterArguments) {
        if arguments.ac_buses_unpowered()
            && context.indicated_airspeed() > Velocity::new::<knot>(100.)
        {
            self.is_active_for_duration += context.delta();
        } else {
            self.is_active_for_duration = Duration::from_secs(0)
        }
    }

    fn is_active(&self) -> bool {
        self.is_active_for_duration > Duration::from_secs(0)
    }

    fn apu_start_inhibited(&self) -> bool {
        self.is_active_for_duration
            < Duration::from_secs(EmergencyElec::APU_START_INHIBIT_DELAY_SECONDS)
    }
}

struct ArrowBetweenBatteryAndBatBus {
    discharging_above_1_ampere_beyond_time: DelayedTrueLogicGate,
    charging_above_1_ampere_beyond_time: DelayedTrueLogicGate,
}
impl ArrowBetweenBatteryAndBatBus {
    fn new() -> Self {
        Self {
            discharging_above_1_ampere_beyond_time: DelayedTrueLogicGate::new(Duration::from_secs(
                BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
            )),
            charging_above_1_ampere_beyond_time: DelayedTrueLogicGate::new(Duration::from_secs(
                BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
            )),
        }
    }

    fn update(&mut self, context: &UpdateContext, arguments: &BatteryChargeLimiterArguments) {
        self.discharging_above_1_ampere_beyond_time.update(
            context,
            arguments.battery_current() < ElectricCurrent::new::<ampere>(-1.),
        );
        self.charging_above_1_ampere_beyond_time.update(
            context,
            arguments.battery_current() > ElectricCurrent::new::<ampere>(1.),
        );
    }

    fn should_show_when_contactor_closed(&self) -> bool {
        self.discharging_above_1_ampere_beyond_time.output()
            || self.charging_above_1_ampere_beyond_time.output()
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
                battery::Battery,
                consumption::{PowerConsumer, SuppliedPower},
                Contactor, ElectricalBus, ElectricalBusType, Potential, PotentialOrigin,
                PotentialTarget,
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

                // Run once more to detect programming errors where the state goes from
                // open to closed (or vice versa) and then back to the other state within
                // the next tick while this shouldn't happen.
                self.test_bed.set_delta(Duration::from_secs(0));
                self.test_bed.run_aircraft(&mut self.aircraft);

                self
            }

            fn wait_for_closed_contactor(mut self, assert_is_closed: bool) -> Self {
                self.aircraft.set_battery_bus_at_minimum_charging_voltage();
                self = self.run(Duration::from_millis(
                    OpenContactorObserver::BATTERY_CHARGING_CLOSE_DELAY_MILLISECONDS,
                ));

                if assert_is_closed {
                    assert!(
                        self.aircraft.battery_contactor_is_closed(),
                        "Battery contactor didn't close within the expected time frame.
                            Is the battery bus at a high enough voltage and the battery not full?"
                    );
                }

                self
            }

            fn wait_for_emergency_elec_apu_no_longer_inhibited(mut self) -> Self {
                self = self.emergency_elec().run(Duration::from_secs(
                    EmergencyElec::APU_START_INHIBIT_DELAY_SECONDS,
                ));

                self
            }

            fn pre_discharge_protection_state(mut self) -> Self {
                self = self
                    .indicated_airspeed_of(Velocity::new::<knot>(0.))
                    .and()
                    .on_the_ground()
                    .wait_for_closed_contactor(true)
                    .then_continue_with()
                    .nearly_empty_battery_charge()
                    .and()
                    .no_power_outside_of_battery();

                self
            }

            fn cycle_battery_push_button(mut self) -> Self {
                self = self.battery_push_button_off();
                self = self.battery_push_button_auto();

                self
            }

            fn battery_push_button_off(mut self) -> Self {
                self.aircraft.set_battery_push_button_off();
                self = self.run(Duration::from_secs(0));

                self
            }

            fn battery_push_button_auto(mut self) -> Self {
                self.aircraft.set_battery_push_button_auto();
                self = self.run(Duration::from_secs(0));

                self
            }

            fn available_emergency_generator(mut self) -> Self {
                self.aircraft.set_emergency_generator_available();
                self
            }

            fn started_apu(mut self) -> Self {
                self.aircraft.set_apu_master_sw_pb_on();
                self.aircraft.set_apu_start_pb_on();

                self = self.run(Duration::from_secs(0));

                self.aircraft.set_apu_available();
                self.aircraft.set_apu_start_pb_off();

                self
            }

            fn stopped_apu(mut self) -> Self {
                self.aircraft.set_apu_master_sw_pb_off();
                self = self.run(Duration::from_secs(0));

                self.aircraft.set_apu_unavailable();

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

            fn nearly_empty_battery_charge(mut self) -> Self {
                self.aircraft.set_nearly_empty_battery_charge();
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

            fn apu_master_sw_pb_on(mut self) -> Self {
                self.aircraft.set_apu_master_sw_pb_on();
                self
            }

            fn apu_master_sw_pb_off(mut self) -> Self {
                self.aircraft.set_apu_master_sw_pb_off();
                self
            }

            fn should_show_arrow_when_contactor_closed(&mut self) -> bool {
                self.test_bed
                    .read_bool("ELEC_CONTACTOR_TEST_SHOW_ARROW_WHEN_CLOSED")
            }

            fn emergency_elec(mut self) -> Self {
                self = self.no_power_outside_of_battery();
                self.test_bed
                    .set_indicated_airspeed(Velocity::new::<knot>(101.));

                self
            }

            fn gear_down(mut self) -> Self {
                self.aircraft.set_gear_down();

                self
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
            apu_available: bool,
            battery_push_button_auto: bool,
            gear_is_down: bool,
            emergency_generator_available: bool,
        }
        impl TestAircraft {
            fn new(battery: Battery) -> Self {
                Self {
                    battery,
                    battery_charge_limiter: BatteryChargeLimiter::new("TEST"),
                    battery_bus: ElectricalBus::new(ElectricalBusType::DirectCurrentBattery),
                    battery_contactor: Contactor::new("TEST"),
                    consumer: PowerConsumer::from(ElectricalBusType::DirectCurrentBattery),
                    both_ac_buses_unpowered: false,
                    apu_master_sw_pb_on: false,
                    apu_start_pb_on: false,
                    apu_available: false,
                    battery_push_button_auto: true,
                    gear_is_down: false,
                    emergency_generator_available: false,
                }
            }

            fn set_full_battery_charge(&mut self) {
                self.battery.set_full_charge()
            }

            fn set_nearly_empty_battery_charge(&mut self) {
                self.battery.set_nearly_empty_battery_charge();
            }

            fn set_battery_bus_at_minimum_charging_voltage(&mut self) {
                self.battery_bus.powered_by(&Potential::single(
                    PotentialOrigin::TransformerRectifier(1),
                    ElectricPotential::new::<volt>(
                        OpenContactorObserver::BATTERY_BUS_BELOW_CHARGING_VOLTAGE + 0.000001,
                    ),
                ));
            }

            fn set_battery_bus_below_minimum_charging_voltage(&mut self) {
                self.battery_bus.powered_by(&Potential::single(
                    PotentialOrigin::TransformerRectifier(1),
                    ElectricPotential::new::<volt>(
                        OpenContactorObserver::BATTERY_BUS_BELOW_CHARGING_VOLTAGE,
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

            fn set_apu_available(&mut self) {
                self.apu_available = true;
            }

            fn set_apu_unavailable(&mut self) {
                self.apu_available = false;
            }

            fn set_power_demand(&mut self, power: Power) {
                self.consumer.demand(power);
            }

            fn battery_contactor_is_closed(&self) -> bool {
                self.battery_contactor.is_closed()
            }

            fn set_battery_push_button_auto(&mut self) {
                self.battery_push_button_auto = true;
            }

            fn set_battery_push_button_off(&mut self) {
                self.battery_push_button_auto = false;
            }

            fn set_gear_down(&mut self) {
                self.gear_is_down = true;
            }

            fn set_emergency_generator_available(&mut self) {
                self.emergency_generator_available = true;
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
                        self.apu_available,
                        self.battery_push_button_auto,
                        !self.gear_is_down,
                        self.emergency_generator_available,
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
                .wait_for_closed_contactor(true)
                .run(Duration::from_secs(
                    BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
                ));

            assert!(test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_not_show_arrow_when_contactor_closed_while_almost_15_seconds_have_passed_charging_above_1_a(
        ) {
            let mut test_bed = test_bed().wait_for_closed_contactor(true).run(
                Duration::from_secs(
                    BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
                ) - Duration::from_millis(1),
            );

            assert!(!test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_not_show_arrow_when_contactor_closed_while_charging_below_1_a() {
            let mut test_bed = test_bed()
                .wait_for_closed_contactor(true)
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
                .wait_for_emergency_elec_apu_no_longer_inhibited()
                .then_continue_with()
                .power_demand_of(Power::new::<watt>(50.))
                .and()
                .apu_master_sw_pb_on()
                .run(Duration::from_secs(
                    BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
                ));

            assert!(test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_not_show_arrow_when_contactor_closed_while_almost_15_seconds_have_passed_discharging_above_1_a(
        ) {
            let mut test_bed = test_bed()
                .wait_for_closed_contactor(true)
                .then_continue_with()
                .no_power_outside_of_battery()
                .and()
                .power_demand_of(Power::new::<watt>(30.))
                .run(
                    Duration::from_secs(
                        BatteryChargeLimiter::CHARGE_DISCHARGE_ARROW_DISPLAYED_AFTER_SECONDS,
                    ) - Duration::from_millis(1),
                );

            assert!(!test_bed.should_show_arrow_when_contactor_closed())
        }

        #[test]
        fn should_not_show_arrow_when_contactor_closed_while_discharging_below_1_a() {
            let mut test_bed = test_bed()
                .wait_for_closed_contactor(true)
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
                    OpenContactorObserver::BATTERY_CHARGING_CLOSE_DELAY_MILLISECONDS,
                ));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_not_closed_when_battery_voltage_below_charge_threshold_and_battery_bus_above_threshold_for_greater_than_225ms_with_push_button_off(
        ) {
            let test_bed = test_bed_with()
                .battery_bus_at_minimum_charging_voltage()
                .and()
                .battery_push_button_off()
                .run(Duration::from_millis(
                    OpenContactorObserver::BATTERY_CHARGING_CLOSE_DELAY_MILLISECONDS,
                ))
                .then_continue_with()
                .battery_push_button_auto()
                .run(Duration::from_millis(0));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_not_closed_when_battery_voltage_below_charge_threshold_and_battery_bus_above_threshold_for_less_than_225ms(
        ) {
            let test_bed = test_bed_with()
                .battery_bus_at_minimum_charging_voltage()
                .run(
                    Duration::from_millis(
                        OpenContactorObserver::BATTERY_CHARGING_CLOSE_DELAY_MILLISECONDS,
                    ) - Duration::from_millis(1),
                );

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
        fn contactor_closed_when_apu_master_sw_pb_is_turned_on() {
            let test_bed = test_bed_with()
                .full_battery_charge()
                .and()
                .apu_master_sw_pb_on()
                .run(Duration::from_millis(1));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn charging_cycle_on_ground_ends_10_seconds_after_current_less_than_4_ampere() {
            let mut test_bed = test_bed_with()
                .indicated_airspeed_of(Velocity::new::<knot>(0.))
                .and()
                .on_the_ground()
                .wait_for_closed_contactor(true);

            assert!(test_bed.current() >= ElectricCurrent::new::<ampere>(4.), "The test assumes that charging current is equal to or greater than 4 at this point.");

            test_bed =
                test_bed
                    .then_continue_with()
                    .full_battery_charge()
                    .run(Duration::from_secs(
                        ClosedContactorObserver::BATTERY_CHARGING_OPEN_DELAY_ON_GROUND_SECONDS,
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
                .wait_for_closed_contactor(true);

            assert!(test_bed.current() >= ElectricCurrent::new::<ampere>(4.), "The test assumes that charging current is equal to or greater than 4 at this point.");

            test_bed = test_bed.then_continue_with().full_battery_charge().run(
                Duration::from_secs(
                    ClosedContactorObserver::BATTERY_CHARGING_OPEN_DELAY_ON_GROUND_SECONDS,
                ) - Duration::from_millis(1),
            );

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn charging_cycle_does_not_end_when_bat_only_below_100_knots() {
            let mut test_bed = test_bed_with()
                .indicated_airspeed_of(Velocity::new::<knot>(0.))
                .and()
                .on_the_ground()
                .wait_for_closed_contactor(true);

            assert!(test_bed.current() >= ElectricCurrent::new::<ampere>(4.), "The test assumes that charging current is equal to or greater than 4 at this point.");

            test_bed = test_bed
                .then_continue_with()
                .no_power_outside_of_battery()
                .run(Duration::from_secs(
                    ClosedContactorObserver::BATTERY_CHARGING_OPEN_DELAY_ON_GROUND_SECONDS,
                ));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn when_above_100_knots_the_charging_cycle_ends_after_30_minutes_below_4_ampere() {
            let mut test_bed = test_bed().wait_for_closed_contactor(true);

            assert!(test_bed.current() >= ElectricCurrent::new::<ampere>(4.), "The test assumes that charging current is equal to or greater than 4 at this point.");

            test_bed =
                test_bed
                    .then_continue_with()
                    .full_battery_charge()
                    .run(Duration::from_secs(
                        ClosedContactorObserver::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS,
                    ));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn when_above_100_knots_the_charging_cycle_does_not_end_within_30_minutes_before_below_4_ampere(
        ) {
            let mut test_bed = test_bed().wait_for_closed_contactor(true);

            assert!(test_bed.current() >= ElectricCurrent::new::<ampere>(4.), "The test assumes that charging current is equal to or greater than 4 at this point.");

            test_bed =
                test_bed
                    .then_continue_with()
                    .full_battery_charge()
                    .run(Duration::from_secs(
                        ClosedContactorObserver::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS
                    ) - Duration::from_millis(1));

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
                    ClosedContactorObserver::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS,
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
            .run(Duration::from_secs(
                ClosedContactorObserver::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS
            ) - Duration::from_millis(1));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn when_apu_started_the_charging_cycle_ends_30_minutes_after_below_4_ampere_even_when_apu_still_available(
        ) {
            let test_bed = test_bed_with().full_battery_charge().and().started_apu()
                .run(Duration::from_secs(
                    ClosedContactorObserver::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS,
                ));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn when_apu_is_available_the_contactor_does_not_close_for_apu_start_despite_master_sw_pb_being_on(
        ) {
            let test_bed = test_bed_with()
                .full_battery_charge()
                .and()
                .started_apu()
                .run(Duration::from_secs(
                    ClosedContactorObserver::BATTERY_CHARGING_OPEN_DELAY_100_KNOTS_OR_AFTER_APU_START_SECONDS,
                ))
                .then_continue_with()
                .apu_master_sw_pb_on()
                .run(Duration::from_secs(1));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn complete_discharge_protection_ensures_the_battery_doesnt_fully_discharge_on_the_ground()
        {
            let test_bed =
                test_bed_with()
                    .pre_discharge_protection_state()
                    .run(Duration::from_secs(
                        ClosedContactorObserver::BATTERY_DISCHARGE_PROTECTION_DELAY_SECONDS,
                    ));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn complete_discharge_protection_is_reset_by_cycling_the_battery_push_button() {
            let mut test_bed =
                test_bed_with()
                    .pre_discharge_protection_state()
                    .run(Duration::from_secs(
                        ClosedContactorObserver::BATTERY_DISCHARGE_PROTECTION_DELAY_SECONDS,
                    ));

            assert!(
                !test_bed.battery_contactor_is_closed(),
                "The test assumes discharge protection has kicked in at this point in the test."
            );

            test_bed = test_bed
                .then_continue_with()
                .cycle_battery_push_button()
                .and()
                .wait_for_closed_contactor(false);

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn complete_discharge_protection_doesnt_trigger_too_early() {
            let test_bed = test_bed_with().pre_discharge_protection_state().run(
                Duration::from_secs(
                    ClosedContactorObserver::BATTERY_DISCHARGE_PROTECTION_DELAY_SECONDS,
                ) - Duration::from_millis(1),
            );

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn complete_discharge_protection_does_not_activate_in_flight() {
            let test_bed = test_bed()
                .wait_for_closed_contactor(true)
                .then_continue_with()
                .nearly_empty_battery_charge()
                .run(Duration::from_secs(
                    ClosedContactorObserver::BATTERY_DISCHARGE_PROTECTION_DELAY_SECONDS,
                ));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn bat_only_on_ground_doesnt_close_when_discharge_protection_triggered() {
            let mut test_bed =
                test_bed_with()
                    .pre_discharge_protection_state()
                    .run(Duration::from_secs(
                        ClosedContactorObserver::BATTERY_DISCHARGE_PROTECTION_DELAY_SECONDS,
                    ));

            assert!(
                !test_bed.battery_contactor_is_closed(),
                "The test assumes discharge protection has kicked in at this point in the test."
            );

            test_bed = test_bed
                .then_continue_with()
                .wait_for_closed_contactor(false);

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn turning_off_the_battery_while_the_contactor_is_closed_opens_the_contactor() {
            let test_bed = test_bed()
                .wait_for_closed_contactor(true)
                .then_continue_with()
                .battery_push_button_off();

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_doesnt_close_while_the_battery_is_off() {
            let test_bed = test_bed_with()
                .battery_push_button_off()
                .wait_for_closed_contactor(false);

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_doesnt_close_for_apu_start_in_emergency_configuration_with_landing_gear_down()
        {
            let test_bed = test_bed_with()
                .emergency_elec()
                .gear_down()
                .and()
                .apu_master_sw_pb_on()
                .run(Duration::from_millis(1));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_opens_when_gear_goes_down_during_apu_start_in_emergency_configuration() {
            let mut test_bed = test_bed_with()
                .apu_master_sw_pb_on()
                .run(Duration::from_millis(1));

            assert!(
                test_bed.battery_contactor_is_closed(),
                "The test assumes the contactor closed
                at this point due to the APU master kicking in."
            );

            test_bed = test_bed
                .emergency_elec()
                .gear_down()
                .run(Duration::from_millis(1));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_does_close_for_apu_start_outside_of_emergency_configuration_with_landing_gear_down(
        ) {
            let test_bed = test_bed_with()
                .gear_down()
                .and()
                .apu_master_sw_pb_on()
                .run(Duration::from_millis(1));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_does_close_for_apu_start_within_first_45_seconds_of_emergency_elec_when_emergency_generator_available(
        ) {
            let test_bed = test_bed_with()
                .emergency_elec()
                .available_emergency_generator()
                .and()
                .apu_master_sw_pb_on()
                .run(
                    Duration::from_secs(EmergencyElec::APU_START_INHIBIT_DELAY_SECONDS)
                        - Duration::from_millis(1),
                );

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_does_close_for_apu_start_after_45_seconds_of_emergency_elec_when_emergency_generator_unavailable(
        ) {
            let test_bed = test_bed_with()
                .emergency_elec()
                .and()
                .apu_master_sw_pb_on()
                .run(Duration::from_secs(
                    EmergencyElec::APU_START_INHIBIT_DELAY_SECONDS,
                ));

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_does_not_close_for_apu_start_within_first_45_seconds_of_emergency_elec_when_emergency_generator_unavailable(
        ) {
            let test_bed = test_bed_with()
                .emergency_elec()
                .and()
                .apu_master_sw_pb_on()
                .run(
                    Duration::from_secs(EmergencyElec::APU_START_INHIBIT_DELAY_SECONDS)
                        - Duration::from_millis(1),
                );

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_opens_when_aircraft_enters_emer_elec() {
            let mut test_bed = test_bed_with()
                .apu_master_sw_pb_on()
                .run(Duration::from_millis(1));

            assert!(
                test_bed.battery_contactor_is_closed(),
                "The test assumes the contactor closed
                at this point due to the APU master kicking in."
            );

            test_bed = test_bed.emergency_elec().run(Duration::from_millis(1));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_closes_for_a_maximum_of_three_minutes_for_apu_start_in_emergency_elec() {
            let test_bed = test_bed_with()
                .emergency_elec()
                .available_emergency_generator()
                .and()
                .apu_master_sw_pb_on()
                .run(
                    Duration::from_secs(
                        ClosedContactorObserver::EMER_ELEC_APU_MASTER_MAXIMUM_CLOSED_SECONDS,
                    ) - Duration::from_millis(1),
                );

            assert!(test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_opens_after_three_minutes_of_being_closed_for_apu_start_in_emergency_elec() {
            let test_bed = test_bed_with()
                .emergency_elec()
                .available_emergency_generator()
                .and()
                .apu_master_sw_pb_on()
                .run(Duration::from_secs(
                    ClosedContactorObserver::EMER_ELEC_APU_MASTER_MAXIMUM_CLOSED_SECONDS,
                ));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn contactor_opens_immediately_when_apu_master_sw_pb_pushed_off_in_emergency_elec() {
            let mut test_bed = test_bed_with()
                .emergency_elec()
                .available_emergency_generator()
                .and()
                .apu_master_sw_pb_on()
                .run(Duration::from_secs(1));

            assert!(
                test_bed.battery_contactor_is_closed(),
                "A precondition of this test is that the contactor is closed at this point."
            );

            test_bed = test_bed
                .then_continue_with()
                .apu_master_sw_pb_off()
                .run(Duration::from_secs(0));

            assert!(!test_bed.battery_contactor_is_closed());
        }

        #[test]
        fn emergency_elec_apu_start_inhibit_delay_shouldnt_repeat_when_transitioning_states() {
            let mut test_bed = test_bed_with()
                .emergency_elec()
                .and()
                .apu_master_sw_pb_on()
                .run(Duration::from_secs(
                    EmergencyElec::APU_START_INHIBIT_DELAY_SECONDS,
                ));

            assert!(
                test_bed.battery_contactor_is_closed(),
                "A precondition of this test is that the contactor is closed at this point."
            );

            test_bed = test_bed
                .then_continue_with()
                .apu_master_sw_pb_off()
                .run(Duration::from_secs(0));

            assert!(
                !test_bed.battery_contactor_is_closed(),
                "A precondition of this test is that the contactor is closed at this point."
            );

            test_bed = test_bed
                .then_continue_with()
                .apu_master_sw_pb_on()
                .run(Duration::from_secs(0));

            assert!(test_bed.battery_contactor_is_closed(),);
        }
    }
}
