//! This module models the APS3200 APU.
//!
//! Internally it contains a state machine with the following states:
//! - Shutdown
//! - Starting
//! - Running
//! - Stopping
//!
//! > Not all characteristics have been verified as of yet. Meaning things such as
//! > EGT increases, EGT warning level, etc. are there but might not reflect the
//! > real APU fully. This involves further tweaking as we get more information.
//!
//! # Remaining work and questions
//! - What does "the APU speed is always 100% except for air conditioning, ..." mean?
//!   Komp vaguely remembers this as "when APU bleed is supplying the packs, the rpm reduces to 99%".
//! - When the aircraft has ground power or main gen power, the APU page appears on the ECAM.
//!   At this time we have no ECAM "controller" within the system software, and thus we cannot model
//!   this. We probably want to have some event for this.
//! - As above, the APU page disappears on the ECAM 10 seconds after AVAIL came on.
//! - Automatic shutdown:
//!   - Flap not open.
//!   - EGT overtemperature.
//!   - DC Power Loss (BAT OFF when aircraft on batteries only).
//!   - There are more situations, but we likely won't model all of them.
//! - Effect of APU fire pb on APU state.
//! - EGT MAX improvements: "is a function of N during start and a function of ambient
//!   temperature when running".
//! - Advanced electrical scenarios:
//!   - ECB and starter motor is supplied by DC BAT.
//!   - When in electrical emergency config, battery contactors close for max 3 mins when
//!     APU MASTER SW is on.
//!   - When in flight, and in electrical emergency config, APU start is inhibited for 45 secs.
//! - On creation of an APU, pass some context including ambient temp, so the temp can start at the right value?

use std::time::Duration;

use uom::si::{
    electric_current::ampere, electric_potential::volt, f64::*, frequency::hertz, length::foot,
    ratio::percent, temperature_interval, thermodynamic_temperature::degree_celsius,
};

use crate::{
    electrical::{Current, PowerConductor, PowerSource},
    overhead::OnOffPushButton,
    pneumatic::{BleedAirValve, Valve},
    shared::random_number,
    simulator::{
        SimulatorReadState, SimulatorReadWritable, SimulatorVisitable, SimulatorVisitor,
        SimulatorWriteState, UpdateContext,
    },
};

/// This type will be here until we have a full ELEC implementation.
struct ApuStartContactor {
    closed: bool,
}
impl ApuStartContactor {
    fn new() -> Self {
        ApuStartContactor { closed: false }
    }

    fn update<T: ApuStartContactorController>(&mut self, controller: &T) {
        self.closed = controller.should_close_start_contactor();
    }
}
impl PowerConductor for ApuStartContactor {
    fn output(&self) -> Current {
        if self.closed {
            Current::Direct(
                PowerSource::Battery(1),
                ElectricPotential::new::<volt>(28.5),
                ElectricCurrent::new::<ampere>(35.),
            )
        } else {
            Current::None
        }
    }
}

/// Komp: There is a pressure switch between the fuel valve and the APU.
/// It switches from 0 to 1 when the pressure is >=17 PSI and the signal is received by the ECB
/// And there is a small hysteresis, means it switches back to 0 when <=16 PSI
/// This type will be here until we have a full FUEL implementation.
struct FuelPressureSwitch {
    has_fuel_remaining: bool
}
impl FuelPressureSwitch {
    fn new() -> Self {
        FuelPressureSwitch {
            has_fuel_remaining: false
        }
    }

    fn update(&mut self, has_fuel_remaining: bool) {
        self.has_fuel_remaining = has_fuel_remaining;
    }

    fn has_pressure(&self) -> bool {
        self.has_fuel_remaining
    }
}

trait ApuStartContactorController {
    fn should_close_start_contactor(&self) -> bool;
}

trait AirIntakeFlapController {
    fn should_open_air_intake_flap(&self) -> bool;
}

trait ApuStartStopController {
    fn should_start(&self) -> bool;
    fn should_stop(&self) -> bool;
}

trait BleedAirValveController {
    fn should_open_bleed_air_valve(&self) -> bool;
}

/// Powered by the DC BAT BUS (801PP).
/// Not yet implemented. Will power this up when implementing the electrical system.
/// It is powered when MASTER SW is ON.
struct ElectronicControlBox {
    master_is_on: bool,
    start_is_on: bool,
    start_contactor_is_energized: bool,
    apu_n: Ratio,
    bleed_is_on: bool,
    bleed_air_valve_last_open_time_ago: Duration,
    fault: Option<ApuFault>,
    air_intake_flap_fully_open: bool,
    is_starting: bool,
    egt: ThermodynamicTemperature,
    egt_warning_temperature: ThermodynamicTemperature,
}
impl ElectronicControlBox {
    const BLEED_AIR_COOLDOWN_DURATION_MILLIS: u64 = 120000;

    fn new() -> Self {
        ElectronicControlBox {
            master_is_on: false,
            start_is_on: false,
            start_contactor_is_energized: false,
            apu_n: Ratio::new::<percent>(0.),
            bleed_is_on: false,
            bleed_air_valve_last_open_time_ago: Duration::from_secs(1000),
            fault: None,
            air_intake_flap_fully_open: false,
            is_starting: false,
            egt: ThermodynamicTemperature::new::<degree_celsius>(0.),
            egt_warning_temperature: ThermodynamicTemperature::new::<degree_celsius>(
                Running::MAX_EGT,
            ),
        }
    }

    fn update_overhead_panel_state(&mut self, overhead: &AuxiliaryPowerUnitOverheadPanel, apu_bleed_is_on: bool) {
        self.master_is_on = overhead.master_is_on();
        self.start_is_on = overhead.start_is_on();
        self.bleed_is_on = apu_bleed_is_on;
    }

    fn update_air_intake_flap_state(&mut self, air_intake_flap: &AirIntakeFlap) {
        self.air_intake_flap_fully_open = air_intake_flap.is_fully_open();
    }

    fn update_start_contactor_state<T: PowerConductor>(&mut self, start_contactor: &T) {
        self.start_contactor_is_energized = start_contactor.is_powered()
    }

    fn update(&mut self, context: &UpdateContext, state: &mut dyn ApuState) {
        self.apu_n = state.get_n();
        self.egt = state.get_egt();
        self.egt_warning_temperature = state.get_egt_warning_temperature(context);

        if !self.master_is_on && self.apu_n.get::<percent>() == 0. {
            // We reset the fault when master is not on and the APU is not running.
            // Once electrical is implemented, the ECB will be unpowered that will reset the fault.
            self.fault = None;
        }

        self.is_starting = state.is_starting();
    }

    fn update_bleed_air_valve_state<T: Valve>(
        &mut self,
        context: &UpdateContext,
        bleed_air_valve: &T,
    ) {
        if bleed_air_valve.is_open() {
            self.bleed_air_valve_last_open_time_ago = Duration::from_secs(0);
        } else {
            self.bleed_air_valve_last_open_time_ago += context.delta;
        }
    }

    fn update_fuel_pressure_switch_state(&mut self, fuel_pressure_switch: &FuelPressureSwitch) {
        if 3. <= self.apu_n.get::<percent>() && !fuel_pressure_switch.has_pressure() {
            self.fault = Some(ApuFault::FuelLowPressure);
        }
    }

    /// Indicates if a fault has occurred which would cause the
    /// MASTER SW fault light to turn on.
    fn has_fault(&self) -> bool {
        self.fault.is_some()
    }

    fn bleed_air_valve_was_open_in_last(&self, duration: Duration) -> bool {
        self.bleed_air_valve_last_open_time_ago <= duration
    }

    fn is_available(&self) -> bool {
        !self.has_fault() && self.apu_n.get::<percent>() > 99.5
    }

    fn get_egt_warning_temperature(&self) -> ThermodynamicTemperature {
        self.egt_warning_temperature
    }

    fn get_egt_caution_temperature(&self) -> ThermodynamicTemperature {
        const WARNING_TO_CAUTION_DIFFERENCE: f64 = 33.;
        ThermodynamicTemperature::new::<degree_celsius>(
            self.egt_warning_temperature.get::<degree_celsius>() - WARNING_TO_CAUTION_DIFFERENCE,
        )
    }

    fn get_egt(&self) -> ThermodynamicTemperature {
        self.egt
    }

    fn get_n(&self) -> Ratio {
        self.apu_n
    }
}
impl ApuStartContactorController for ElectronicControlBox {
    /// Indicates if the APU start contactor should be closed.
    fn should_close_start_contactor(&self) -> bool {
        if self.has_fault() {
            false
        } else {
            self.apu_n.get::<percent>() < 55.
                && ((self.master_is_on && self.start_is_on && self.air_intake_flap_fully_open)
                    || self.is_starting)
        }
    }
}
impl AirIntakeFlapController for ElectronicControlBox {
    /// Indicates if the air intake flap should be opened.
    fn should_open_air_intake_flap(&self) -> bool {
        self.master_is_on || 
            // While running, the air intake flap remains open.
            // Manual shutdown sequence: the air intake flap closes at N = 7%.
            7. <= self.apu_n.get::<percent>() 
                // While starting, the air intake flap remains open; even when the
                // starting sequence has only just begun and the MASTER SW is turned off.
                || self.is_starting
    }
}
impl ApuStartStopController for ElectronicControlBox {
    /// Indicates if the start sequence should be started.
    fn should_start(&self) -> bool {
        self.start_contactor_is_energized
    }

    fn should_stop(&self) -> bool {
        self.has_fault()
            || (!self.master_is_on && !self.is_starting
                && !self.bleed_air_valve_was_open_in_last(Duration::from_millis(
                    ElectronicControlBox::BLEED_AIR_COOLDOWN_DURATION_MILLIS,
                )))
    }
}
impl BleedAirValveController for ElectronicControlBox {
    fn should_open_bleed_air_valve(&self) -> bool {
        self.master_is_on && self.apu_n.get::<percent>() > 95. && self.bleed_is_on
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ApuFault {
    FuelLowPressure,
}

pub struct AuxiliaryPowerUnit {
    state: Option<Box<dyn ApuState>>,
    ecb: ElectronicControlBox,
    start_contactor: ApuStartContactor,
    air_intake_flap: AirIntakeFlap,
    bleed_air_valve: ApuBleedAirValve,
    fuel_pressure_switch: FuelPressureSwitch
}
impl AuxiliaryPowerUnit {
    pub fn new() -> AuxiliaryPowerUnit {
        AuxiliaryPowerUnit {
            state: Some(Box::new(Shutdown::new(
                ThermodynamicTemperature::new::<degree_celsius>(0.),
            ))),
            ecb: ElectronicControlBox::new(),
            start_contactor: ApuStartContactor::new(),
            air_intake_flap: AirIntakeFlap::new(),
            bleed_air_valve: ApuBleedAirValve::new(),
            fuel_pressure_switch: FuelPressureSwitch::new(),
        }
    }

    pub fn update(
        &mut self,
        context: &UpdateContext,
        overhead: &AuxiliaryPowerUnitOverheadPanel,
        apu_bleed_is_on: bool,
        apu_gen_is_used: bool,
        has_fuel_remaining: bool,
    ) {
        self.ecb.update_overhead_panel_state(overhead, apu_bleed_is_on);
        self.start_contactor.update(&self.ecb);
        self.ecb.update_start_contactor_state(&self.start_contactor);
        self.fuel_pressure_switch.update(has_fuel_remaining);
        self.ecb.update_fuel_pressure_switch_state(&self.fuel_pressure_switch);

        if let Some(state) = self.state.take() {
            let mut new_state = state.update(
                context,
                self.bleed_air_valve.is_open(),
                apu_gen_is_used,
                &self.ecb,
            );

            self.ecb.update(context, new_state.as_mut());

            self.state = Some(new_state);
        }

        self.air_intake_flap.update(context, &self.ecb);
        self.ecb.update_air_intake_flap_state(&self.air_intake_flap);
        self.bleed_air_valve.update(&self.ecb);
        self.ecb.update_bleed_air_valve_state(context, &self.bleed_air_valve);
    }

    pub fn get_n(&self) -> Ratio {
        self.ecb.get_n()
    }

    pub fn is_available(&self) -> bool {
        self.ecb.is_available()
    }

    fn get_air_intake_flap_open_amount(&self) -> Ratio {
        self.air_intake_flap.get_open_amount()
    }

    fn get_egt(&self) -> ThermodynamicTemperature {
        self.ecb.get_egt()
    }

    fn start_contactor_energized(&self) -> bool {
        self.start_contactor.is_powered()
    }

    fn bleed_air_valve_is_open(&self) -> bool {
        self.bleed_air_valve.is_open()
    }

    fn has_fault(&self) -> bool {
        self.ecb.has_fault()
    }

    fn get_egt_caution_temperature(&self) -> ThermodynamicTemperature {
        self.ecb.get_egt_caution_temperature()
    }

    fn get_egt_warning_temperature(&self) -> ThermodynamicTemperature {
        self.ecb.get_egt_warning_temperature()
    }
}
impl SimulatorVisitable for AuxiliaryPowerUnit {
    fn accept<T: SimulatorVisitor>(&mut self, visitor: &mut T) {
        visitor.visit(self);
    }
}
impl SimulatorReadWritable for AuxiliaryPowerUnit {
    fn write(&self, state: &mut SimulatorWriteState) {
        state.apu_bleed_air_valve_open = self.bleed_air_valve_is_open();
        state.apu_air_intake_flap_opened_for = self.get_air_intake_flap_open_amount();
        state.apu_caution_egt = self.get_egt_caution_temperature();
        state.apu_egt = self.get_egt();
        state.apu_n = self.get_n();
        state.apu_start_contactor_energized = self.start_contactor_energized();
        state.apu_warning_egt = self.get_egt_warning_temperature();
    }
}

trait ApuState {
    fn update(
        self: Box<Self>,
        context: &UpdateContext,
        apu_bleed_is_used: bool,
        apu_gen_is_used: bool,
        controller: &dyn ApuStartStopController,
    ) -> Box<dyn ApuState>;

    fn get_n(&self) -> Ratio;

    fn get_egt(&self) -> ThermodynamicTemperature;

    fn get_egt_warning_temperature(&self, context: &UpdateContext) -> ThermodynamicTemperature;

    fn is_starting(&self) -> bool;
}

struct Shutdown {
    egt: ThermodynamicTemperature,
}
impl Shutdown {
    fn new(egt: ThermodynamicTemperature) -> Shutdown {
        Shutdown { egt }
    }
}
impl ApuState for Shutdown {
    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        _: bool,
        _: bool,
        controller: &dyn ApuStartStopController,
    ) -> Box<dyn ApuState> {
        self.egt = calculate_towards_ambient_egt(self.egt, context);

        if controller.should_start() {
            Box::new(Starting::new(self.egt))
        } else {
            self
        }
    }

    fn get_n(&self) -> Ratio {
        Ratio::new::<percent>(0.)
    }

    fn get_egt(&self) -> ThermodynamicTemperature {
        self.egt
    }

    fn get_egt_warning_temperature(&self, _: &UpdateContext) -> ThermodynamicTemperature {
        // Not a programming error, MAX EGT displayed when shutdown is the running max EGT.
        ThermodynamicTemperature::new::<degree_celsius>(Running::MAX_EGT)
    }

    fn is_starting(&self) -> bool {
        false
    }
}

struct Starting {
    since: Duration,
    n: Ratio,
    egt: ThermodynamicTemperature,
    ignore_calculated_egt: bool,
}
impl Starting {
    const MAX_EGT_BELOW_25000_FEET: f64 = 900.;
    const MAX_EGT_AT_OR_ABOVE_25000_FEET: f64 = 982.;

    fn new(egt: ThermodynamicTemperature) -> Starting {
        Starting {
            since: Duration::from_secs(0),
            n: Ratio::new::<percent>(0.),
            egt,
            ignore_calculated_egt: true,
        }
    }

    fn calculate_egt(&mut self, context: &UpdateContext) -> ThermodynamicTemperature {
        // Refer to APS3200.md for details on the values below and source data.
        const APU_N_TEMP_CONST: f64 = 0.8260770092912485;
        const APU_N_TEMP_X: f64 = -10.521171805148322;
        const APU_N_TEMP_X2: f64 = 9.99178942595435338876;
        const APU_N_TEMP_X3: f64 = -3.08275284793509220859;
        const APU_N_TEMP_X4: f64 = 0.42614542950594842237;
        const APU_N_TEMP_X5: f64 = -0.03117154621503876974;
        const APU_N_TEMP_X6: f64 = 0.00138431867550105467;
        const APU_N_TEMP_X7: f64 = -0.00004016856934546301;
        const APU_N_TEMP_X8: f64 = 0.00000078892955962222;
        const APU_N_TEMP_X9: f64 = -0.00000001058955825891;
        const APU_N_TEMP_X10: f64 = 0.00000000009582985112;
        const APU_N_TEMP_X11: f64 = -0.00000000000055952490;
        const APU_N_TEMP_X12: f64 = 0.00000000000000190415;
        const APU_N_TEMP_X13: f64 = -0.00000000000000000287;

        let n = self.n.get::<percent>();

        // Results below this value momentarily go above 0, while not intended.
        if n < 5.5 {
            calculate_towards_ambient_egt(self.egt, context)
        } else {
            let temperature = ThermodynamicTemperature::new::<degree_celsius>(
                APU_N_TEMP_CONST
                    + (APU_N_TEMP_X * n)
                    + (APU_N_TEMP_X2 * n.powi(2))
                    + (APU_N_TEMP_X3 * n.powi(3))
                    + (APU_N_TEMP_X4 * n.powi(4))
                    + (APU_N_TEMP_X5 * n.powi(5))
                    + (APU_N_TEMP_X6 * n.powi(6))
                    + (APU_N_TEMP_X7 * n.powi(7))
                    + (APU_N_TEMP_X8 * n.powi(8))
                    + (APU_N_TEMP_X9 * n.powi(9))
                    + (APU_N_TEMP_X10 * n.powi(10))
                    + (APU_N_TEMP_X11 * n.powi(11))
                    + (APU_N_TEMP_X12 * n.powi(12))
                    + (APU_N_TEMP_X13 * n.powi(13)),
            );

            // The above calculated EGT can be lower than the ambient temperature,
            // or the current APU EGT (when cooling down). To prevent sudden changes
            // in temperature, we ignore the calculated EGT until it exceeds the current
            // EGT.
            let towards_ambient_egt = calculate_towards_ambient_egt(self.egt, context);
            if temperature > towards_ambient_egt {
                self.ignore_calculated_egt = false;
            }

            if self.ignore_calculated_egt {
                towards_ambient_egt
            } else {
                temperature
            }
        }
    }

    fn calculate_n(&self) -> Ratio {
        const APU_N_CONST: f64 = -0.08013606018640967497;
        const APU_N_X: f64 = 2.12983273639453440535;
        const APU_N_X2: f64 = 3.92827343878640406445;
        const APU_N_X3: f64 = -1.88613299921213003406;
        const APU_N_X4: f64 = 0.42749452749180915438;
        const APU_N_X5: f64 = -0.05757707967690425694;
        const APU_N_X6: f64 = 0.00502214279545100437;
        const APU_N_X7: f64 = -0.00029612873626050868;
        const APU_N_X8: f64 = 0.00001204152497871946;
        const APU_N_X9: f64 = -0.00000033829604438116;
        const APU_N_X10: f64 = 0.00000000645140818528;
        const APU_N_X11: f64 = -0.00000000007974743535;
        const APU_N_X12: f64 = 0.00000000000057654695;
        const APU_N_X13: f64 = -0.00000000000000185126;

        // Protect against the formula returning decreasing results after this value.
        const TIME_LIMIT: f64 = 45.12;
        const START_IGNITION_AFTER_SECONDS: f64 = 1.5;
        let ignition_turned_on_secs =
            (self.since.as_secs_f64() - START_IGNITION_AFTER_SECONDS).min(TIME_LIMIT);

        if ignition_turned_on_secs > 0. {
            let n = (APU_N_CONST
                + (APU_N_X * ignition_turned_on_secs)
                + (APU_N_X2 * ignition_turned_on_secs.powi(2))
                + (APU_N_X3 * ignition_turned_on_secs.powi(3))
                + (APU_N_X4 * ignition_turned_on_secs.powi(4))
                + (APU_N_X5 * ignition_turned_on_secs.powi(5))
                + (APU_N_X6 * ignition_turned_on_secs.powi(6))
                + (APU_N_X7 * ignition_turned_on_secs.powi(7))
                + (APU_N_X8 * ignition_turned_on_secs.powi(8))
                + (APU_N_X9 * ignition_turned_on_secs.powi(9))
                + (APU_N_X10 * ignition_turned_on_secs.powi(10))
                + (APU_N_X11 * ignition_turned_on_secs.powi(11))
                + (APU_N_X12 * ignition_turned_on_secs.powi(12))
                + (APU_N_X13 * ignition_turned_on_secs.powi(13)))
            .min(100.)
            .max(0.);

            Ratio::new::<percent>(n)
        } else {
            Ratio::new::<percent>(0.)
        }
    }
}
impl ApuState for Starting {
    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        _: bool,
        _: bool,
        controller: &dyn ApuStartStopController,
    ) -> Box<dyn ApuState> {
        self.since = self.since + context.delta;
        self.n = self.calculate_n();
        self.egt = self.calculate_egt(context);

        if controller.should_stop() {
            Box::new(Stopping::new(
                self.egt,
                self.n
            ))
        } else if self.n.get::<percent>() == 100. {
            Box::new(Running::new(self.egt))
        } else {
            self
        }
    }

    fn get_n(&self) -> Ratio {
        self.n
    }

    fn get_egt(&self) -> ThermodynamicTemperature {
        self.egt
    }

    fn get_egt_warning_temperature(&self, context: &UpdateContext) -> ThermodynamicTemperature {
        if context.indicated_altitude.get::<foot>() < 25_000. {
            ThermodynamicTemperature::new::<degree_celsius>(Starting::MAX_EGT_BELOW_25000_FEET)
        } else {
            ThermodynamicTemperature::new::<degree_celsius>(
                Starting::MAX_EGT_AT_OR_ABOVE_25000_FEET,
            )
        }
    }

    fn is_starting(&self) -> bool {
        true
    }
}

struct Running {
    egt: ThermodynamicTemperature,
    base_temperature: ThermodynamicTemperature,
    bleed_air_in_use_delta_temperature: TemperatureInterval,
    apu_gen_in_use_delta_temperature: TemperatureInterval,
}
impl Running {
    const MAX_EGT: f64 = 682.;

    fn new(egt: ThermodynamicTemperature) -> Running {
        let base_temperature = 340. + ((random_number() % 11) as f64);
        let bleed_air_in_use_delta_temperature = 30. + ((random_number() % 11) as f64);
        let apu_gen_in_use_delta_temperature = 10. + ((random_number() % 6) as f64);
        Running {
            egt,
            base_temperature: ThermodynamicTemperature::new::<degree_celsius>(base_temperature),
            bleed_air_in_use_delta_temperature: TemperatureInterval::new::<
                temperature_interval::degree_celsius,
            >(bleed_air_in_use_delta_temperature),
            apu_gen_in_use_delta_temperature: TemperatureInterval::new::<
                temperature_interval::degree_celsius,
            >(apu_gen_in_use_delta_temperature),
        }
    }

    fn calculate_slow_cooldown_to_running_temperature(
        &self,
        context: &UpdateContext,
        apu_gen_is_used: bool,
        apu_bleed_is_used: bool,
    ) -> ThermodynamicTemperature {
        let mut target_temperature = self.base_temperature;
        if apu_bleed_is_used {
            target_temperature += self.bleed_air_in_use_delta_temperature;
        }
        if apu_gen_is_used {
            target_temperature += self.apu_gen_in_use_delta_temperature;
        }

        calculate_towards_target_egt(self.egt, target_temperature, 0.4, context.delta)
    }
}
impl ApuState for Running {
    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        apu_bleed_is_used: bool,
        apu_gen_is_used: bool,
        controller: &dyn ApuStartStopController,
    ) -> Box<dyn ApuState> {
        self.egt = self.calculate_slow_cooldown_to_running_temperature(
            context,
            apu_gen_is_used,
            apu_bleed_is_used,
        );

        if controller.should_stop() {
            Box::new(Stopping::new(self.egt, Ratio::new::<percent>(100.)))
        } else {
            self
        }
    }

    fn get_n(&self) -> Ratio {
        Ratio::new::<percent>(100.)
    }

    fn get_egt(&self) -> ThermodynamicTemperature {
        self.egt
    }

    fn get_egt_warning_temperature(&self, _: &UpdateContext) -> ThermodynamicTemperature {
        ThermodynamicTemperature::new::<degree_celsius>(Running::MAX_EGT)
    }

    fn is_starting(&self) -> bool {
        false
    }
}

struct Stopping {
    since: Duration,
    n: Ratio,
    egt: ThermodynamicTemperature,
}
impl Stopping {
    fn new(egt: ThermodynamicTemperature, n: Ratio) -> Stopping {
        Stopping {
            since: Duration::from_secs(0),
            n,
            egt,
        }
    }

    fn calculate_n(&self, context: &UpdateContext) -> Ratio {
        const SPOOL_DOWN_COEFFICIENT: f64 = 2.;
        let mut n = self.n.get::<percent>();
        n = (n - (context.delta.as_secs_f64() * SPOOL_DOWN_COEFFICIENT)).max(0.);

        Ratio::new::<percent>(n)
    }
}
impl ApuState for Stopping {
    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        _: bool,
        _: bool,
        _: &dyn ApuStartStopController,
    ) -> Box<dyn ApuState> {
        self.since = self.since + context.delta;
        self.n = self.calculate_n(context);
        self.egt = calculate_towards_ambient_egt(self.egt, context);

        if self.n.get::<percent>() == 0. {
            Box::new(Shutdown::new(self.egt))
        } else {
            self
        }
    }

    fn get_n(&self) -> Ratio {
        self.n
    }

    fn get_egt(&self) -> ThermodynamicTemperature {
        self.egt
    }

    fn get_egt_warning_temperature(&self, _: &UpdateContext) -> ThermodynamicTemperature {
        // Not a programming error, MAX EGT displayed when stopping is the running max EGT.
        ThermodynamicTemperature::new::<degree_celsius>(Running::MAX_EGT)
    }

    fn is_starting(&self) -> bool {
        false
    }
}

fn calculate_towards_ambient_egt(
    current_egt: ThermodynamicTemperature,
    context: &UpdateContext,
) -> ThermodynamicTemperature {
    const APU_AMBIENT_COEFFICIENT: f64 = 2.;
    calculate_towards_target_egt(
        current_egt,
        context.ambient_temperature,
        APU_AMBIENT_COEFFICIENT,
        context.delta,
    )
}

fn calculate_towards_target_egt(
    current: ThermodynamicTemperature,
    target: ThermodynamicTemperature,
    coefficient: f64,
    delta: Duration,
) -> ThermodynamicTemperature {
    if current == target {
        current
    } else if current > target {
        ThermodynamicTemperature::new::<degree_celsius>(
            (current.get::<degree_celsius>() - (coefficient * delta.as_secs_f64()))
                .max(target.get::<degree_celsius>()),
        )
    } else {
        ThermodynamicTemperature::new::<degree_celsius>(
            (current.get::<degree_celsius>() + (coefficient * delta.as_secs_f64()))
                .min(target.get::<degree_celsius>()),
        )
    }
}

struct ApuBleedAirValve {
    valve: BleedAirValve,
}
impl ApuBleedAirValve {
    fn new() -> Self {
        ApuBleedAirValve {
            valve: BleedAirValve::new(),
        }
    }

    fn update<T: BleedAirValveController>(&mut self, controller: &T) {
        self.valve
            .open_when(controller.should_open_bleed_air_valve());
    }
}
impl Valve for ApuBleedAirValve {
    fn is_open(&self) -> bool {
        self.valve.is_open()
    }
}

/// APS3200 APU Generator
pub struct ApuGenerator {
    output: Current,
}
impl ApuGenerator {
    const APU_GEN_POWERED_N: f64 = 84.;

    pub fn new() -> ApuGenerator {
        ApuGenerator {
            output: Current::None,
        }
    }

    pub fn update(&mut self, apu: &AuxiliaryPowerUnit) {
        let n = apu.get_n();
        self.output = if n.get::<percent>() < ApuGenerator::APU_GEN_POWERED_N {
            Current::None
        } else {
            Current::Alternating(
                PowerSource::ApuGenerator,
                self.calculate_frequency(n),
                self.calculate_potential(n),
                // TODO: Once we actually know what to do with the amperes, we'll have to adapt this.
                ElectricCurrent::new::<ampere>(782.60),
            )
        }
    }

    fn calculate_potential(&self, n: Ratio) -> ElectricPotential {
        let n = n.get::<percent>();

        if n < ApuGenerator::APU_GEN_POWERED_N {
            panic!("Should not be invoked for APU N below {}", n);
        } else if n < 85. {
            ElectricPotential::new::<volt>(105.)
        } else if n < 100. {
            ElectricPotential::new::<volt>(114. + (random_number() % 2) as f64)
        } else {
            // TODO: This should sometimes go from 115 to 114 and back.
            // However, if we simply recalculate with a random number every tick, it will jump around too much.
            // We need to create some type that can manage recalculations which are somewhat time limited.
            ElectricPotential::new::<volt>(115.)
        }
    }

    fn calculate_frequency(&self, n: Ratio) -> Frequency {
        let n = n.get::<percent>();

        // Refer to APS3200.md for details on the values below and source data.
        if n < ApuGenerator::APU_GEN_POWERED_N {
            panic!("Should not be invoked for APU N below {}", n);
        } else if n < 100. {
            const APU_FREQ_CONST: f64 = 1076894372064.8204;
            const APU_FREQ_X: f64 = -118009165327.71873606955288934986;
            const APU_FREQ_X2: f64 = 5296044666.71179983947567172640;
            const APU_FREQ_X3: f64 = -108419965.09400677044360088955;
            const APU_FREQ_X4: f64 = -36793.31899267512494461444;
            const APU_FREQ_X5: f64 = 62934.36386220135515418897;
            const APU_FREQ_X6: f64 = -1870.51971585477668178674;
            const APU_FREQ_X7: f64 = 31.37647374314980530193;
            const APU_FREQ_X8: f64 = -0.35101507164597609613;
            const APU_FREQ_X9: f64 = 0.00272649361414786631;
            const APU_FREQ_X10: f64 = -0.00001463272647792659;
            const APU_FREQ_X11: f64 = 0.00000005203375009496;
            const APU_FREQ_X12: f64 = -0.00000000011071318044;
            const APU_FREQ_X13: f64 = 0.00000000000010697005;

            Frequency::new::<hertz>(
                APU_FREQ_CONST
                    + (APU_FREQ_X * n)
                    + (APU_FREQ_X2 * n.powi(2))
                    + (APU_FREQ_X3 * n.powi(3))
                    + (APU_FREQ_X4 * n.powi(4))
                    + (APU_FREQ_X5 * n.powi(5))
                    + (APU_FREQ_X6 * n.powi(6))
                    + (APU_FREQ_X7 * n.powi(7))
                    + (APU_FREQ_X8 * n.powi(8))
                    + (APU_FREQ_X9 * n.powi(9))
                    + (APU_FREQ_X10 * n.powi(10))
                    + (APU_FREQ_X11 * n.powi(11))
                    + (APU_FREQ_X12 * n.powi(12))
                    + (APU_FREQ_X13 * n.powi(13)),
            )
        } else {
            Frequency::new::<hertz>(400.)
        }
    }

    fn frequency_within_normal_range(&self) -> bool {
        let hz = self.output().get_frequency().get::<hertz>();
        390. <= hz && hz <= 410.
    }

    fn potential_within_normal_range(&self) -> bool {
        let volts = self.output().get_potential().get::<volt>();
        110. <= volts && volts <= 120.
    }
}
impl PowerConductor for ApuGenerator {
    fn output(&self) -> Current {
        self.output
    }
}
impl SimulatorVisitable for ApuGenerator {
    fn accept<T: SimulatorVisitor>(&mut self, visitor: &mut T) {
        visitor.visit(self);
    }
}
impl SimulatorReadWritable for ApuGenerator {
    fn write(&self, state: &mut SimulatorWriteState) {
        state.apu_gen_current = self.output().get_current();
        state.apu_gen_frequency = self.output().get_frequency();
        state.apu_gen_frequency_within_normal_range = self.frequency_within_normal_range();
        state.apu_gen_potential = self.output().get_potential();
        state.apu_gen_potential_within_normal_range = self.potential_within_normal_range();
    }
}

pub struct AuxiliaryPowerUnitOverheadPanel {
    pub master: OnOffPushButton,
    pub start: OnOffPushButton,
}
impl AuxiliaryPowerUnitOverheadPanel {
    pub fn new() -> AuxiliaryPowerUnitOverheadPanel {
        AuxiliaryPowerUnitOverheadPanel {
            master: OnOffPushButton::new_off(),
            start: OnOffPushButton::new_off(),
        }
    }

    pub fn update_after_apu(&mut self, apu: &AuxiliaryPowerUnit) {
        self.start.set_available(apu.is_available());
        if self.start_is_on() && (apu.is_available() || apu.has_fault()) {
            self.start.turn_off();
        }

        self.master.set_fault(apu.has_fault());
    }

    fn master_has_fault(&self) -> bool {
        self.master.has_fault()
    }

    fn master_is_on(&self) -> bool {
        self.master.is_on()
    }

    fn start_is_on(&self) -> bool {
        self.start.is_on()
    }

    fn start_shows_available(&self) -> bool {
        self.start.shows_available()
    }
}
impl SimulatorVisitable for AuxiliaryPowerUnitOverheadPanel {
    fn accept<T: SimulatorVisitor>(&mut self, visitor: &mut T) {
        visitor.visit(self);
    }
}
impl SimulatorReadWritable for AuxiliaryPowerUnitOverheadPanel {
    fn read(&mut self, state: &SimulatorReadState) {
        self.master.set(state.apu_master_sw_on);
        self.start.set(state.apu_start_sw_on);
    }

    fn write(&self, state: &mut SimulatorWriteState) {
        state.apu_master_sw_fault = self.master_has_fault();
        state.apu_start_sw_on = self.start_is_on();
        state.apu_start_sw_available = self.start_shows_available();
    }
}

#[derive(Debug)]
struct AirIntakeFlap {
    state: Ratio,
    delay: Duration,
}
impl AirIntakeFlap {
    const MINIMUM_TRAVEL_TIME_SECS: u8 = 3;

    fn new() -> AirIntakeFlap {
        let delay = Duration::from_secs(
            (AirIntakeFlap::MINIMUM_TRAVEL_TIME_SECS + (random_number() % 13)) as u64,
        );

        AirIntakeFlap {
            state: Ratio::new::<percent>(0.),
            delay,
        }
    }

    fn update<T: AirIntakeFlapController>(
        &mut self,
        context: &UpdateContext,
        controller: &T,
    ) {
        if controller.should_open_air_intake_flap() && self.state < Ratio::new::<percent>(100.) {
            self.state += Ratio::new::<percent>(
                self.get_flap_change_for_delta(context)
                    .min(100. - self.state.get::<percent>()),
            );
        } else if !controller.should_open_air_intake_flap()
            && self.state > Ratio::new::<percent>(0.)
        {
            self.state -= Ratio::new::<percent>(
                self.get_flap_change_for_delta(context)
                    .min(self.state.get::<percent>()),
            );
        }
    }

    fn get_flap_change_for_delta(&self, context: &UpdateContext) -> f64 {
        100. * (context.delta.as_secs_f64() / self.delay.as_secs_f64())
    }

    fn is_fully_open(&self) -> bool {
        self.state == Ratio::new::<percent>(100.)
    }

    fn get_open_amount(&self) -> Ratio {
        self.state
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::simulator::test_helpers::context_with;
    use std::time::Duration;
    use uom::si::thermodynamic_temperature::degree_celsius;

    pub fn running_apu() -> AuxiliaryPowerUnit {
        tester_with().running_apu().get_apu()
    }

    pub fn stopped_apu() -> AuxiliaryPowerUnit {
        tester().get_apu()
    }

    fn starting_apu() -> AuxiliaryPowerUnit {
        tester_with().starting_apu().get_apu()
    }

    fn tester_with() -> AuxiliaryPowerUnitTester {
        AuxiliaryPowerUnitTester::new()
    }

    fn tester() -> AuxiliaryPowerUnitTester {
        AuxiliaryPowerUnitTester::new()
    }

    struct AuxiliaryPowerUnitTester {
        apu: AuxiliaryPowerUnit,
        apu_overhead: AuxiliaryPowerUnitOverheadPanel,
        apu_bleed: OnOffPushButton,
        apu_generator: ApuGenerator,
        ambient_temperature: ThermodynamicTemperature,
        indicated_altitude: Length,
        apu_gen_is_used: bool,
        has_fuel_remaining: bool,
    }
    impl AuxiliaryPowerUnitTester {
        fn new() -> Self {
            AuxiliaryPowerUnitTester {
                apu: AuxiliaryPowerUnit::new(),
                apu_overhead: AuxiliaryPowerUnitOverheadPanel::new(),
                apu_bleed: OnOffPushButton::new_on(),
                apu_generator: ApuGenerator::new(),
                ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(0.),
                indicated_altitude: Length::new::<foot>(5000.),
                apu_gen_is_used: true,
                has_fuel_remaining: true,
            }
        }

        fn air_intake_flap_that_opens_in(mut self, duration: Duration) -> Self {
            self.apu.air_intake_flap.delay = duration;
            self
        }

        fn master_on(mut self) -> Self {
            self.apu_overhead.master.turn_on();
            self
        }

        fn master_off(mut self) -> Self {
            self.apu_overhead.master.turn_off();
            self
        }

        fn start_on(mut self) -> Self {
            self.apu_overhead.start.turn_on();
            self
        }

        fn start_off(mut self) -> Self {
            self.apu_overhead.start.turn_off();
            self
        }

        fn bleed_air_off(mut self) -> Self {
            self.apu_bleed.turn_off();
            self
        }

        fn starting_apu(self) -> Self {
            self.apu_ready_to_start()
                .then_continue_with()
                .start_on()
                .run(Duration::from_secs(0))
        }

        fn apu_gen_not_used(mut self) -> Self {
            self.apu_gen_is_used = false;
            self
        }

        fn no_fuel_available(mut self) -> Self {
            self.has_fuel_remaining = false;
            self
        }

        fn running_apu(mut self) -> Self {
            self = self.starting_apu();
            loop {
                self = self.run(Duration::from_secs(1));
                if self.apu.is_available() {
                    self = self.run(Duration::from_secs(10));
                    break;
                }
            }

            self
        }

        fn cooling_down_apu(mut self) -> Self {
            self = self.running_apu();
            self = self.master_off();
            loop {
                self = self.run(Duration::from_secs(1));

                if self.get_n().get::<percent>() == 0. {
                    break;
                }
            }

            self
        }

        fn apu_ready_to_start(mut self) -> Self {
            self = self.master_on();

            loop {
                self = self.run(Duration::from_secs(1));

                if self.apu.get_air_intake_flap_open_amount().get::<percent>() == 100. {
                    break;
                }
            }

            self
        }

        fn running_apu_with_bleed_air(mut self) -> Self {
            self.apu_bleed.turn_on();
            self.running_apu()
        }

        fn running_apu_without_bleed_air(mut self) -> Self {
            self.apu_bleed.turn_off();
            self.running_apu()
        }

        fn ambient_temperature(mut self, ambient: ThermodynamicTemperature) -> Self {
            self.ambient_temperature = ambient;
            self
        }

        fn indicated_altitude(mut self, indicated_altitute: Length) -> Self {
            self.indicated_altitude = indicated_altitute;
            self
        }

        fn and(self) -> Self {
            self
        }

        fn then_continue_with(self) -> Self {
            self
        }

        fn run(self, delta: Duration) -> Self {
            self.run_inner(delta).run_inner(Duration::from_secs(0))
        }

        fn run_inner(mut self, delta: Duration) -> Self {
            self.apu.update(
                &context_with()
                    .delta(delta)
                    .ambient_temperature(self.ambient_temperature)
                    .and()
                    .indicated_altitude(self.indicated_altitude)
                    .build(),
                &self.apu_overhead,
                self.apu_bleed.is_on(),
                self.apu_gen_is_used,
                self.has_fuel_remaining,
            );

            self.apu_generator.update(&self.apu);

            self.apu_overhead.update_after_apu(&self.apu);

            self
        }

        fn is_air_intake_flap_fully_open(&self) -> bool {
            self.apu.get_air_intake_flap_open_amount().get::<percent>() == 100.
        }

        fn get_n(&self) -> Ratio {
            self.apu.get_n()
        }

        fn get_egt(&self) -> ThermodynamicTemperature {
            self.apu.get_egt()
        }

        fn get_egt_warning_temperature(&self) -> ThermodynamicTemperature {
            self.apu.get_egt_warning_temperature()
        }

        fn get_egt_caution_temperature(&self) -> ThermodynamicTemperature {
            self.apu.get_egt_caution_temperature()
        }

        fn apu_is_available(&self) -> bool {
            self.apu.is_available()
        }

        fn start_is_on(&self) -> bool {
            self.apu_overhead.start_is_on()
        }

        fn start_shows_available(&self) -> bool {
            self.apu_overhead.start_shows_available()
        }

        fn master_has_fault(&self) -> bool {
            self.apu_overhead.master_has_fault()
        }

        fn get_apu(self) -> AuxiliaryPowerUnit {
            self.apu
        }

        fn get_generator_output(&self) -> Current {
            self.apu_generator.output()
        }

        fn start_contactor_energized(&self) -> bool {
            self.apu.start_contactor_energized()
        }

        fn generator_frequency_within_normal_range(&self) -> bool {
            self.apu_generator.frequency_within_normal_range()
        }

        fn generator_potential_within_normal_range(&self) -> bool {
            self.apu_generator.potential_within_normal_range()
        }
    }

    #[cfg(test)]
    mod apu_tests {
        use ntest::{assert_about_eq, timeout};

        use super::*;

        const APPROXIMATE_STARTUP_TIME: u64 = 49;

        #[test]
        fn when_apu_master_sw_turned_on_air_intake_flap_opens() {
            let tester = tester_with().master_on().run(Duration::from_secs(20));

            assert_eq!(tester.is_air_intake_flap_fully_open(), true)
        }

        #[test]
        fn when_apu_master_sw_turned_on_and_air_intake_flap_not_yet_open_apu_does_not_start() {
            let tester = tester_with()
                .air_intake_flap_that_opens_in(Duration::from_secs(20))
                .master_on()
                .run(Duration::from_millis(1))
                .then_continue_with()
                .start_on()
                .run(Duration::from_secs(0))
                .run(Duration::from_secs(15));

            assert_eq!(tester.get_n().get::<percent>(), 0.);
        }

        #[test]
        fn while_starting_below_n_7_when_apu_master_sw_turned_off_air_intake_flap_does_not_close() {
            let mut tester = tester_with().starting_apu();
            let mut n = 0.;

            loop {
                tester = tester.run(Duration::from_millis(50));
                n = tester.get_n().get::<percent>();
                if n > 1. {
                    break;
                }
            }

            assert!(n < 2.);
            tester = tester.master_off().run(Duration::from_millis(50));
            assert!(tester.is_air_intake_flap_fully_open());
        }

        #[test]
        fn when_start_sw_on_apu_starts_within_expected_time() {
            let tester = tester_with()
                .starting_apu()
                .run(Duration::from_secs(APPROXIMATE_STARTUP_TIME));

            assert_eq!(tester.get_n().get::<percent>(), 100.);
        }

        #[test]
        fn one_and_a_half_seconds_after_starting_sequence_commences_ignition_starts() {
            let tester = tester_with()
                .starting_apu()
                .run(Duration::from_millis(1500));

            assert_eq!(
                tester.get_n().get::<percent>(),
                0.,
                "Ignition started too early."
            );

            // The first 35ms ignition started but N hasn't increased beyond 0 yet.
            let tester = tester.then_continue_with().run(Duration::from_millis(36));

            assert!(
                tester.get_n().get::<percent>() > 0.,
                "Ignition started too late."
            );
        }

        #[test]
        fn when_apu_not_started_egt_is_ambient() {
            const AMBIENT_TEMPERATURE: f64 = 0.;

            let tester = tester_with()
                .ambient_temperature(ThermodynamicTemperature::new::<degree_celsius>(
                    AMBIENT_TEMPERATURE,
                ))
                .run(Duration::from_secs(1_000));

            assert_eq!(
                tester.get_egt().get::<degree_celsius>(),
                AMBIENT_TEMPERATURE
            );
        }

        #[test]
        fn when_ambient_temperature_high_startup_egt_never_below_ambient() {
            const AMBIENT_TEMPERATURE: f64 = 50.;

            let tester = tester_with()
                .ambient_temperature(ThermodynamicTemperature::new::<degree_celsius>(
                    AMBIENT_TEMPERATURE,
                ))
                .run(Duration::from_secs(500))
                .then_continue_with()
                .starting_apu()
                .run(Duration::from_secs(1));

            assert_eq!(
                tester.get_egt().get::<degree_celsius>(),
                AMBIENT_TEMPERATURE
            );
        }

        #[test]
        fn when_apu_starting_egt_reaches_above_700_degree_celsius() {
            let mut tester = tester_with().starting_apu();
            let mut max_egt: f64 = 0.;

            loop {
                tester = tester.run(Duration::from_secs(1));

                let egt = tester.get_egt().get::<degree_celsius>();
                if egt < max_egt {
                    break;
                }

                max_egt = egt;
            }

            assert!(max_egt > 700.);
        }

        #[test]
        fn egt_max_always_33_above_egt_warn() {
            let mut tester = tester_with().starting_apu();

            for _ in 1..=100 {
                tester = tester.run(Duration::from_secs(1));

                assert_about_eq!(
                    tester.get_egt_warning_temperature().get::<degree_celsius>(),
                    tester.get_egt_caution_temperature().get::<degree_celsius>() + 33.
                );
            }
        }

        #[test]
        fn start_sw_on_light_turns_off_when_apu_available() {
            let mut tester = tester_with().starting_apu();

            loop {
                tester = tester.run(Duration::from_secs(1));

                if tester.apu_is_available() {
                    break;
                }
            }

            assert!(!tester.start_is_on());
            assert!(tester.start_shows_available());
        }

        #[test]
        fn when_apu_bleed_valve_open_on_shutdown_cooldown_period_commences_and_apu_remains_available(
        ) {
            // The cool down period is between 60 to 120. It is configurable by aircraft mechanics and
            // we'll make it a configurable option in the sim. For now, 120s.
            let mut tester =
                tester_with()
                    .running_apu()
                    .and()
                    .master_off()
                    .run(Duration::from_millis(
                        ElectronicControlBox::BLEED_AIR_COOLDOWN_DURATION_MILLIS,
                    ));

            assert!(tester.apu_is_available());

            // Move from Running to Shutdown state.
            tester = tester.run(Duration::from_millis(1));
            // APU N reduces below 99,5%.
            tester = tester.run(Duration::from_secs(1));

            assert!(!tester.apu_is_available());
        }

        #[test]
        fn when_apu_bleed_valve_was_open_recently_on_shutdown_cooldown_period_commences_and_apu_remains_available(
        ) {
            // The cool down period requires that the bleed valve is shut for a duration (default 120s).
            // If the bleed valve was shut earlier than the MASTER SW going to OFF, that time period counts towards the cool down period.

            let mut tester = tester_with()
                .running_apu_with_bleed_air()
                .and()
                .bleed_air_off()
                .run(Duration::from_millis(
                    (ElectronicControlBox::BLEED_AIR_COOLDOWN_DURATION_MILLIS / 3) * 2,
                ));

            assert!(tester.apu_is_available());

            tester = tester.master_off().run(Duration::from_millis(
                ElectronicControlBox::BLEED_AIR_COOLDOWN_DURATION_MILLIS / 3,
            ));

            assert!(tester.apu_is_available());

            // Move from Running to Shutdown state.
            tester = tester.run(Duration::from_millis(1));
            // APU N reduces below 99,5%.
            tester = tester.run(Duration::from_secs(1));

            assert!(!tester.apu_is_available());
        }

        #[test]
        fn when_apu_bleed_valve_closed_on_shutdown_cooldown_period_is_skipped_and_apu_stops() {
            let mut tester = tester_with().running_apu_without_bleed_air();

            assert!(tester.apu_is_available());

            // Move from Running to Shutdown state.
            tester = tester.master_off().run(Duration::from_millis(1));
            // APU N reduces below 99,5%.
            tester = tester.run(Duration::from_secs(1));

            assert!(!tester.apu_is_available());
        }

        #[test]
        fn when_master_sw_off_then_back_on_during_cooldown_period_apu_continues_running() {
            let tester = tester_with()
                .running_apu_with_bleed_air()
                .and()
                .master_off()
                .run(Duration::from_millis(
                    ElectronicControlBox::BLEED_AIR_COOLDOWN_DURATION_MILLIS,
                ));

            let tester = tester
                .then_continue_with()
                .master_on()
                .run(Duration::from_millis(1));

            assert!(tester.apu_is_available());
        }

        #[test]
        #[timeout(500)]
        fn when_apu_starting_and_master_plus_start_sw_off_then_apu_continues_starting_and_shuts_down_after_start(
        ) {
            let mut tester = tester_with()
                .starting_apu()
                .run(Duration::from_secs(APPROXIMATE_STARTUP_TIME / 2));

            assert!(tester.get_n().get::<percent>() > 0.);

            tester = tester
                .then_continue_with()
                .master_off()
                .and()
                .start_off()
                .run(Duration::from_secs(APPROXIMATE_STARTUP_TIME / 2));

            assert!(tester.get_n().get::<percent>() > 90.);

            loop {
                tester = tester.then_continue_with().run(Duration::from_secs(1));

                if tester.get_n().get::<percent>() == 0. {
                    break;
                }
            }
        }

        #[test]
        fn when_apu_shutting_down_at_7_percent_n_air_inlet_flap_closes() {
            let mut tester = tester_with().running_apu().and().master_off();

            loop {
                tester = tester.run(Duration::from_millis(50));

                if tester.get_n().get::<percent>() <= 7. {
                    break;
                }
            }

            assert!(!tester.is_air_intake_flap_fully_open());
        }

        #[test]
        #[timeout(500)]
        fn apu_cools_down_to_ambient_temperature_after_running() {
            let ambient = ThermodynamicTemperature::new::<degree_celsius>(10.);
            let mut tester = tester_with()
                .running_apu()
                .ambient_temperature(ambient)
                .and()
                .master_off();

            while tester.get_egt() != ambient {
                tester = tester.run(Duration::from_secs(1));
            }
        }

        #[test]
        fn shutdown_apu_warms_up_as_ambient_temperature_increases() {
            let starting_temperature = ThermodynamicTemperature::new::<degree_celsius>(0.);
            let tester = tester_with().ambient_temperature(starting_temperature);

            let tester = tester.run(Duration::from_secs(1_000));

            let target_temperature = ThermodynamicTemperature::new::<degree_celsius>(20.);

            let tester = tester
                .then_continue_with()
                .ambient_temperature(target_temperature)
                .run(Duration::from_secs(1_000));

            assert_eq!(tester.get_egt(), target_temperature);
        }

        #[test]
        /// Q: What would you say is a normal running EGT?
        /// Komp: It cools down by a few degrees. Not much though. 340-350 I'd say.
        fn running_apu_egt_without_bleed_air_usage_stabilizes_between_340_to_350_degrees() {
            let tester = tester_with()
                .running_apu_without_bleed_air()
                .and()
                .apu_gen_not_used()
                .run(Duration::from_secs(1_000));

            let egt = tester.get_egt().get::<degree_celsius>();
            assert!(340. <= egt && egt <= 350.);
        }

        #[test]
        /// Komp: APU generator supplying will add maybe like 10-15 degrees.
        fn running_apu_with_generator_supplying_electricity_increases_egt_by_10_to_15_degrees_to_between_350_to_365_degrees(
        ) {
            let tester = tester_with()
                .running_apu_without_bleed_air()
                .run(Duration::from_secs(1_000));

            let egt = tester.get_egt().get::<degree_celsius>();
            assert!(350. <= egt && egt <= 365.);
        }

        #[test]
        /// Komp: Bleed adds even more. Not sure how much, 30-40 degrees as a rough guess.
        fn running_apu_supplying_bleed_air_increases_egt_by_30_to_40_degrees_to_between_370_to_390_degrees(
        ) {
            let tester = tester_with()
                .running_apu_with_bleed_air()
                .and()
                .apu_gen_not_used()
                .run(Duration::from_secs(1_000));

            let egt = tester.get_egt().get::<degree_celsius>();
            assert!(370. <= egt && egt <= 390.);
        }

        #[test]
        /// Komp: Bleed adds even more. Not sure how much, 30-40 degrees as a rough guess.
        fn running_apu_supplying_bleed_air_and_electrical_increases_egt_to_between_380_to_405_degrees(
        ) {
            let tester = tester_with()
                .running_apu_with_bleed_air()
                .run(Duration::from_secs(1_000));

            let egt = tester.get_egt().get::<degree_celsius>();
            assert!(380. <= egt && egt <= 405.);
        }

        #[test]
        fn max_starting_egt_below_25000_feet_is_900_degrees() {
            let tester = tester_with()
                .starting_apu()
                .and()
                .indicated_altitude(Length::new::<foot>(24999.))
                .run(Duration::from_secs(1));

            assert_about_eq!(
                tester.get_egt_warning_temperature().get::<degree_celsius>(),
                900.
            );
        }

        #[test]
        fn max_starting_egt_at_or_above_25000_feet_is_982_degrees() {
            let tester = tester_with()
                .starting_apu()
                .and()
                .indicated_altitude(Length::new::<foot>(25000.))
                .run(Duration::from_secs(1));

            assert_about_eq!(
                tester.get_egt_warning_temperature().get::<degree_celsius>(),
                982.
            );
        }

        #[test]
        fn starting_apu_n_is_never_below_0() {
            let mut tester = tester_with().starting_apu();

            loop {
                tester = tester.run(Duration::from_millis(10));

                assert!(tester.get_n().get::<percent>() >= 0.);

                if tester.apu_is_available() {
                    break;
                }
            }
        }

        #[test]
        fn restarting_apu_which_is_cooling_down_does_not_suddenly_reduce_egt_to_ambient_temperature(
        ) {
            let mut tester = tester_with().cooling_down_apu();

            assert!(tester.get_egt().get::<degree_celsius>() > 100.);

            tester = tester
                .then_continue_with()
                .starting_apu()
                .run(Duration::from_secs(5));

            assert!(tester.get_egt().get::<degree_celsius>() > 100.);
        }

        #[test]
        fn restarting_apu_which_is_cooling_down_does_reduce_towards_ambient_until_startup_egt_above_current_egt(
        ) {
            let mut tester = tester_with().cooling_down_apu();

            let initial_egt = tester.get_egt();

            tester = tester
                .then_continue_with()
                .starting_apu()
                .run(Duration::from_secs(5));

            assert!(tester.get_egt() < initial_egt);
        }

        #[test]
        fn start_contactor_is_energised_when_starting_until_n_55() {
            let mut tester = tester_with().starting_apu();

            loop {
                tester = tester.run(Duration::from_millis(50));
                let n = tester.get_n().get::<percent>();

                assert_eq!(tester.start_contactor_energized(), n < 55.);

                if n == 100. {
                    break;
                }
            }
        }

        #[test]
        fn start_contactor_is_energised_when_starting_until_n_55_even_if_master_sw_turned_off() {
            let mut tester = tester_with().starting_apu();

            loop {
                tester = tester.run(Duration::from_millis(50));
                let n = tester.get_n().get::<percent>();

                if n > 30. {
                    tester = tester.master_off();
                }

                assert_eq!(tester.start_contactor_energized(), n < 55.);

                if n == 100. {
                    break;
                }
            }
        }

        #[test]
        fn start_contactor_is_not_energised_when_shutdown() {
            let tester = tester().run(Duration::from_secs(1_000));
            assert_eq!(tester.start_contactor_energized(), false);
        }

        #[test]
        fn start_contactor_is_not_energised_when_shutting_down() {
            let mut tester = tester_with()
                .running_apu()
                .then_continue_with()
                .master_off();

            loop {
                tester = tester.run(Duration::from_millis(50));
                assert_eq!(tester.start_contactor_energized(), false);

                if tester.get_n().get::<percent>() == 0. {
                    break;
                }
            }
        }

        #[test]
        fn start_contactor_is_not_energised_when_running() {
            let tester = tester_with().running_apu().run(Duration::from_secs(1_000));
            assert_eq!(tester.start_contactor_energized(), false);
        }

        #[test]
        fn available_when_n_above_99_5_percent() {
            let mut tester = tester_with().starting_apu();

            loop {
                tester = tester.run(Duration::from_millis(50));
                let n = tester.get_n().get::<percent>();
                assert!((n > 99.5 && tester.apu_is_available()) || !tester.apu_is_available());

                if n == 100. {
                    break;
                }
            }
        }

        #[test]
        #[timeout(500)]
        fn without_fuel_apu_starts_until_approximately_n_3_percent_and_then_shuts_down_with_fault() {
            let mut tester = tester_with()
                .no_fuel_available()
                .run(Duration::from_secs(1_000))
                .then_continue_with()
                .starting_apu();

            loop {
                tester = tester.run(Duration::from_millis(50));
                if tester.get_n().get::<percent>() >= 3.  {
                    break;
                }
            }

            tester = tester.run(Duration::from_secs(10));

            assert_eq!(tester.apu_is_available(), false);
            assert!(tester.master_has_fault());
            assert!(!tester.start_is_on());
        }

        #[test]
        fn starting_apu_shuts_down_when_no_more_fuel_available() {
            let tester = tester_with()
                .starting_apu()
                .run(Duration::from_secs(10))
                .then_continue_with()
                .no_fuel_available()
                .run(Duration::from_secs(1_000));

            assert_eq!(tester.apu_is_available(), false);
            assert!(tester.master_has_fault());
            assert!(!tester.start_is_on());
        }

        #[test]
        fn running_apu_shuts_down_when_no_more_fuel_available() {
            let tester = tester_with()
                .running_apu()
                .then_continue_with()
                .no_fuel_available()
                // Two runs, because of state change from Running to Stopping.
                .run(Duration::from_millis(1))
                .run(Duration::from_secs(1_000));

            assert_eq!(tester.apu_is_available(), false);
            assert!(tester.master_has_fault());
            assert!(!tester.start_is_on());
        }
    }

    #[cfg(test)]
    mod apu_generator_tests {
        use ntest::assert_about_eq;

        use crate::apu::tests::{running_apu, stopped_apu};

        use super::*;

        #[test]
        fn starts_without_output() {
            assert!(apu_generator().output.is_unpowered());
        }

        #[test]
        fn when_apu_running_provides_output() {
            let mut generator = apu_generator();
            update_below_threshold(&mut generator);
            update_above_threshold(&mut generator);

            assert!(generator.output.is_powered());
        }

        #[test]
        fn when_apu_shutdown_provides_no_output() {
            let mut generator = apu_generator();
            update_above_threshold(&mut generator);
            update_below_threshold(&mut generator);

            assert!(generator.output.is_unpowered());
        }

        #[test]
        fn from_n_84_provides_voltage() {
            let mut tester = tester_with().starting_apu();

            loop {
                tester = tester.run(Duration::from_millis(50));

                let n = tester.get_n().get::<percent>();
                if n > 84. {
                    assert!(tester.get_generator_output().get_potential().get::<volt>() > 0.);
                }

                if n == 100. {
                    break;
                }
            }
        }

        #[test]
        fn from_n_84_has_frequency() {
            let mut tester = tester_with().starting_apu();

            loop {
                tester = tester.run(Duration::from_millis(50));

                let n = tester.get_n().get::<percent>();
                if n > 84. {
                    assert!(tester.get_generator_output().get_frequency().get::<hertz>() > 0.);
                }

                if n == 100. {
                    break;
                }
            }
        }

        #[test]
        fn in_normal_conditions_when_n_100_voltage_114_or_115() {
            let mut tester = tester_with().running_apu();

            for _ in 0..100 {
                tester = tester.run(Duration::from_millis(50));

                let voltage = tester.get_generator_output().get_potential().get::<volt>();
                assert!(114. <= voltage && voltage <= 115.)
            }
        }

        #[test]
        fn in_normal_conditions_when_n_100_frequency_400() {
            let mut tester = tester_with().running_apu();

            for _ in 0..100 {
                tester = tester.run(Duration::from_millis(50));

                let frequency = tester.get_generator_output().get_frequency().get::<hertz>();
                assert_about_eq!(frequency, 400.);
            }
        }

        #[test]
        fn when_shutdown_frequency_not_normal() {
            let tester = tester().run(Duration::from_secs(1_000));

            assert!(!tester.generator_frequency_within_normal_range());
        }

        #[test]
        fn when_running_frequency_normal() {
            let tester = tester().running_apu().run(Duration::from_secs(1_000));

            assert!(tester.generator_frequency_within_normal_range());
        }

        #[test]
        fn when_shutdown_potential_not_normal() {
            let tester = tester().run(Duration::from_secs(1_000));

            assert!(!tester.generator_potential_within_normal_range());
        }

        #[test]
        fn when_running_potential_normal() {
            let tester = tester().running_apu().run(Duration::from_secs(1_000));

            assert!(tester.generator_potential_within_normal_range());
        }

        fn apu_generator() -> ApuGenerator {
            ApuGenerator::new()
        }

        fn update_above_threshold(generator: &mut ApuGenerator) {
            generator.update(&running_apu());
        }

        fn update_below_threshold(generator: &mut ApuGenerator) {
            generator.update(&stopped_apu());
        }
    }

    #[cfg(test)]
    mod air_intake_flap_tests {
        use super::*;

        struct TestFlapController {
            should_open: bool,
        }
        impl TestFlapController {
            fn new() -> Self {
                TestFlapController { should_open: false }
            }

            fn open(&mut self) {
                self.should_open = true;
            }

            fn close(&mut self) {
                self.should_open = false;
            }
        }
        impl AirIntakeFlapController for TestFlapController {
            fn should_open_air_intake_flap(&self) -> bool {
                self.should_open
            }
        }

        #[test]
        fn starts_opening_when_target_is_open() {
            let mut flap = AirIntakeFlap::new();
            let mut controller = TestFlapController::new();
            controller.open();

            flap.update(
                &context_with().delta(Duration::from_secs(5)).build(),
                &controller,
            );

            assert!(flap.state.get::<percent>() > 0.);
        }

        #[test]
        fn does_not_instantly_open() {
            let mut flap = AirIntakeFlap::new();
            let mut controller = TestFlapController::new();
            controller.open();

            flap.update(
                &context_with()
                    .delta(Duration::from_secs(
                        (AirIntakeFlap::MINIMUM_TRAVEL_TIME_SECS - 1) as u64,
                    ))
                    .build(),
                &controller,
            );

            assert_ne!(flap.state.get::<percent>(), 100.);
        }

        #[test]
        fn closes_when_target_is_closed() {
            let mut flap = AirIntakeFlap::new();
            let mut controller = TestFlapController::new();
            controller.open();

            flap.update(
                &context_with().delta(Duration::from_secs(5)).build(),
                &controller,
            );
            let open_percentage = flap.state.get::<percent>();

            controller.close();
            flap.update(
                &context_with().delta(Duration::from_secs(2)).build(),
                &controller,
            );

            assert!(flap.state.get::<percent>() < open_percentage);
        }

        #[test]
        fn does_not_instantly_close() {
            let mut flap = AirIntakeFlap::new();
            let mut controller = TestFlapController::new();
            controller.open();

            flap.update(
                &context_with().delta(Duration::from_secs(5)).build(),
                &controller,
            );

            controller.close();
            flap.update(
                &context_with()
                    .delta(Duration::from_secs(
                        (AirIntakeFlap::MINIMUM_TRAVEL_TIME_SECS - 1) as u64,
                    ))
                    .build(),
                &controller,
            );

            assert_ne!(flap.state.get::<percent>(), 0.);
        }

        #[test]
        fn never_closes_beyond_0_percent() {
            let mut flap = AirIntakeFlap::new();
            let mut controller = TestFlapController::new();
            controller.close();

            flap.update(
                &context_with().delta(Duration::from_secs(1_000)).build(),
                &controller,
            );

            assert_eq!(flap.state.get::<percent>(), 0.);
        }

        #[test]
        fn never_opens_beyond_100_percent() {
            let mut flap = AirIntakeFlap::new();
            let mut controller = TestFlapController::new();
            controller.open();

            flap.update(
                &context_with().delta(Duration::from_secs(1_000)).build(),
                &controller,
            );

            assert_eq!(flap.state.get::<percent>(), 100.);
        }

        #[test]
        fn is_fully_open_returns_false_when_closed() {
            let flap = AirIntakeFlap::new();

            assert_eq!(flap.is_fully_open(), false)
        }

        #[test]
        fn is_fully_open_returns_true_when_open() {
            let mut flap = AirIntakeFlap::new();
            let mut controller = TestFlapController::new();
            controller.open();

            flap.update(
                &context_with().delta(Duration::from_secs(1_000)).build(),
                &controller,
            );

            assert_eq!(flap.is_fully_open(), true)
        }
    }
}
