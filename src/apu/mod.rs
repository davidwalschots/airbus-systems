//! This module models the Honeywell 131-9A APU.
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
//! - EGT in stopping scenario.
//! - EGT after stopping (might still have some temperature left to decrease).
//! - What does "the APU speed is always 100% except for air conditioning, ..." mean?
//! - When the aircraft has ground power or main gen power, the APU page appears on the ECAM.
//!   At this time we have no ECAM "controller" within the system software, and thus we cannot model
//!   this. We probably want to have some event for this.
//! - As above, the APU page disappears on the ECAM 10 seconds after AVAIL came on.
//! - Manual shutdown by pressing the MASTER SW should:
//!   - Disable the AVAIL light on the START pb.
//!   - Commence a 60 second cooldown sequence if APU bleed air was used.
//!     Meaning the APU keeps running for that period. What if bleed air was used 15 minutes ago?
//! - Automatic shutdown:
//!   - Flap not open.
//!   - EGT overtemperature.
//!   - DC Power Loss (BAT OFF when aircraft on batteries only).
//!   - There are more situations, but we likely won't model all of them.
//! - What happens when you abort the start sequence of the APU? Can you?
//! - APU cooldown in stopping and shutdown state.
//! - START pb 1,5 second ignition delay.
//! - START pb ON light out at 2 seconds after N >= 95% or immediately when N >= 99.5%.
//! - START pb AVAIL light on at 2 seconds after N >= 95% or immediately when N >= 99.5%.
//! - Effect of APU fire pb on APU state.
//! - EGT MAX improvements: "is a function of N during start and a function of ambient
//!   temperature when running".
//! - Advanced electrical scenarios:
//!   - ECB and starter motor is supplied by DC BAT.
//!   - When in electrical emergency config, battery contactors close for max 3 mins when
//!     APU MASTER SW is on.
//!   - When in flight, and in electrical emergency config, APU start is inhibited for 45 secs.

use std::{thread::current, time::Duration};

use rand::prelude::*;
use uom::si::{f64::*, ratio::percent, thermodynamic_temperature::degree_celsius};

use crate::{overhead::OnOffPushButton, shared::UpdateContext, visitor::Visitable};

#[derive(Clone, Copy, Debug, PartialEq)]
enum ShutdownReason {
    Manual,
    Automatic, // Will be split further later into all kinds of reasons for automatic shutdown.
}

pub struct AuxiliaryPowerUnit {
    state: Option<Box<dyn ApuState>>,
    egt_warning_temp: ThermodynamicTemperature,
}
impl AuxiliaryPowerUnit {
    // TODO: Is this maximum correct for the Honeywell 131-9A?
    // Manual says max EGT is 1090 degree celsius during start and 675 degree celsius while running.
    // That might be for a different model.
    const WARNING_MAX_TEMPERATURE: f64 = 1200.;

    pub fn new_shutdown() -> AuxiliaryPowerUnit {
        AuxiliaryPowerUnit::new_with_state(Box::new(Shutdown::new(
            AirIntakeFlap::new(),
            ShutdownReason::Manual,
            ThermodynamicTemperature::new::<degree_celsius>(0.),
        )))
    }

    // TODO: Starting with a running APU doesn't really work due to AirIntakeFlap state being wrong. Reconsider.
    pub fn new_running() -> AuxiliaryPowerUnit {
        AuxiliaryPowerUnit::new_with_state(Box::new(Running::new(
            AirIntakeFlap::new(),
            ThermodynamicTemperature::new::<degree_celsius>(0.),
        )))
    }

    fn new_with_state(state: Box<dyn ApuState>) -> AuxiliaryPowerUnit {
        AuxiliaryPowerUnit {
            state: Some(state),
            egt_warning_temp: ThermodynamicTemperature::new::<degree_celsius>(
                AuxiliaryPowerUnit::WARNING_MAX_TEMPERATURE,
            ),
        }
    }

    pub fn update(&mut self, context: &UpdateContext, overhead: &AuxiliaryPowerUnitOverheadPanel) {
        if let Some(state) = self.state.take() {
            self.state = Some(state.update(context, overhead));
        }

        self.egt_warning_temp = self.calculate_egt_warning_temp();
    }

    pub fn get_n(&self) -> Ratio {
        self.state.as_ref().unwrap().get_n()
    }

    pub fn is_running(&self) -> bool {
        self.get_n().get::<percent>() == 100.
    }

    /// When true, the "FLAP OPEN" message on the ECAM APU page should be displayed.
    fn is_air_intake_flap_fully_open(&self) -> bool {
        self.state.as_ref().unwrap().is_air_intake_flap_fully_open()
    }

    fn get_egt(&self) -> ThermodynamicTemperature {
        self.state.as_ref().unwrap().get_egt()
    }

    fn get_egt_warning_temperature(&self) -> ThermodynamicTemperature {
        self.egt_warning_temp
    }

    fn get_egt_maximum_temperature(&self) -> ThermodynamicTemperature {
        const MAX_ABOVE_WARNING: f64 = 33.;
        ThermodynamicTemperature::new::<degree_celsius>(
            self.egt_warning_temp.get::<degree_celsius>() + MAX_ABOVE_WARNING,
        )
    }

    fn calculate_egt_warning_temp(&self) -> ThermodynamicTemperature {
        let x = match self.get_n().get::<percent>() {
            n if n < 11. => AuxiliaryPowerUnit::WARNING_MAX_TEMPERATURE,
            n if n <= 15. => (-50. * n) + 1750.,
            n if n <= 65. => (-3. * n) + 1045.,
            n => (-30. / 7. * n) + 1128.6,
        };

        ThermodynamicTemperature::new::<degree_celsius>(x)
    }
}

trait ApuState {
    fn update(
        self: Box<Self>,
        context: &UpdateContext,
        overhead: &AuxiliaryPowerUnitOverheadPanel,
    ) -> Box<dyn ApuState>;

    fn get_n(&self) -> Ratio;

    /// When true, the "FLAP OPEN" message on the ECAM APU page should be displayed.
    fn is_air_intake_flap_fully_open(&self) -> bool;

    fn get_egt(&self) -> ThermodynamicTemperature;
}

struct Shutdown {
    air_intake_flap: AirIntakeFlap,
    reason: ShutdownReason,
    egt: ThermodynamicTemperature,
}
impl Shutdown {
    fn new(
        air_intake_flap: AirIntakeFlap,
        reason: ShutdownReason,
        egt: ThermodynamicTemperature,
    ) -> Shutdown {
        Shutdown {
            air_intake_flap,
            reason,
            egt,
        }
    }
}
impl ApuState for Shutdown {
    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        overhead: &AuxiliaryPowerUnitOverheadPanel,
    ) -> Box<dyn ApuState> {
        if overhead.master_is_on() {
            self.air_intake_flap.open();
        } else {
            self.air_intake_flap.close();
        }
        self.air_intake_flap.update(context);

        self.egt = calculate_cooldown_egt(self.egt, context);

        if self.air_intake_flap.is_fully_open() && overhead.master_is_on() && overhead.start_is_on()
        {
            Box::new(Starting::new(self.air_intake_flap))
        } else {
            self
        }
    }

    fn get_n(&self) -> Ratio {
        Ratio::new::<percent>(0.)
    }

    fn is_air_intake_flap_fully_open(&self) -> bool {
        self.air_intake_flap.is_fully_open()
    }

    fn get_egt(&self) -> ThermodynamicTemperature {
        // TODO: We probably still want the EGT for a shutdown APU, as there might be residual heat or it is the ambient temperature?
        ThermodynamicTemperature::new::<degree_celsius>(0.)
    }
}

struct Starting {
    air_intake_flap: AirIntakeFlap,
    since: Duration,
    n: Ratio,
    egt: ThermodynamicTemperature,
}
impl Starting {
    fn new(air_intake_flap: AirIntakeFlap) -> Starting {
        Starting {
            air_intake_flap,
            since: Duration::from_secs(0),
            n: Ratio::new::<percent>(0.),
            egt: ThermodynamicTemperature::new::<degree_celsius>(0.),
        }
    }

    fn calculate_egt(&self, context: &UpdateContext) -> ThermodynamicTemperature {
        const APU_N_TEMP_CONST: f64 = -96.565;
        const APU_N_TEMP_X: f64 = 28.571;
        const APU_N_TEMP_X2: f64 = 0.0884;
        const APU_N_TEMP_X3: f64 = -0.0081;
        const APU_N_TEMP_X4: f64 = 0.00005;

        let n = self.n.get::<percent>();

        let temperature = (APU_N_TEMP_X4 * n.powi(4))
            + (APU_N_TEMP_X3 * n.powi(3))
            + (APU_N_TEMP_X2 * n.powi(2))
            + (APU_N_TEMP_X * n)
            + APU_N_TEMP_CONST;

        ThermodynamicTemperature::new::<degree_celsius>(
            temperature.max(context.ambient_temperature.get::<degree_celsius>()),
        )
    }

    fn calculate_n(&self) -> Ratio {
        const APU_N_X: f64 = 2.375010484;
        const APU_N_X2: f64 = 0.034236847;
        const APU_N_X3: f64 = -0.007404136;
        const APU_N_X4: f64 = 0.000254;
        const APU_N_X5: f64 = -0.000002438;
        const APU_N_CONST: f64 = 0.;

        // Protect against the formula returning decreasing results when a lot of time is skipped.
        const TIME_LIMIT: f64 = 50.;
        let time_since_start = self.since.as_secs_f64().min(TIME_LIMIT);

        Ratio::new::<percent>(
            ((APU_N_X5 * time_since_start.powi(5))
                + (APU_N_X4 * time_since_start.powi(4))
                + (APU_N_X3 * time_since_start.powi(3))
                + (APU_N_X2 * time_since_start.powi(2))
                + (APU_N_X * time_since_start)
                + APU_N_CONST)
                .min(100.),
        )
    }
}
impl ApuState for Starting {
    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        _: &AuxiliaryPowerUnitOverheadPanel,
    ) -> Box<dyn ApuState> {
        self.since = self.since + context.delta;
        self.n = self.calculate_n();
        self.egt = self.calculate_egt(context);

        self.air_intake_flap.update(context);

        if self.n.get::<percent>() == 100. {
            Box::new(Running::new(self.air_intake_flap, self.egt))
        } else {
            self
        }
    }

    fn get_n(&self) -> Ratio {
        self.n
    }

    fn is_air_intake_flap_fully_open(&self) -> bool {
        self.air_intake_flap.is_fully_open()
    }

    fn get_egt(&self) -> ThermodynamicTemperature {
        self.egt
    }
}

struct Running {
    air_intake_flap: AirIntakeFlap,
    egt: ThermodynamicTemperature,
}
impl Running {
    fn new(air_intake_flap: AirIntakeFlap, egt: ThermodynamicTemperature) -> Running {
        Running {
            air_intake_flap,
            egt,
        }
    }

    fn calculate_slow_cooldown_to_running_temperature(
        &self,
        context: &UpdateContext,
    ) -> ThermodynamicTemperature {
        let mut rng = rand::thread_rng();
        let random_target_temperature: f64 = 500. - rng.gen_range(0..13) as f64;

        if self.egt.get::<degree_celsius>() > random_target_temperature {
            self.egt
                - TemperatureInterval::new::<uom::si::temperature_interval::degree_celsius>(
                    0.4 * context.delta.as_secs_f64(),
                )
        } else {
            self.egt
        }
    }
}
impl ApuState for Running {
    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        overhead: &AuxiliaryPowerUnitOverheadPanel,
    ) -> Box<dyn ApuState> {
        self.egt = self.calculate_slow_cooldown_to_running_temperature(context);

        self.air_intake_flap.update(context);

        if overhead.master_is_off() {
            Box::new(Stopping::new(
                self.air_intake_flap,
                self.egt,
                ShutdownReason::Manual,
            ))
        } else {
            self
        }
    }

    fn get_n(&self) -> Ratio {
        Ratio::new::<percent>(100.)
    }

    fn is_air_intake_flap_fully_open(&self) -> bool {
        self.air_intake_flap.is_fully_open()
    }

    fn get_egt(&self) -> ThermodynamicTemperature {
        self.egt
    }
}

struct Stopping {
    air_intake_flap: AirIntakeFlap,
    reason: ShutdownReason,
    since: Duration,
    n: Ratio,
    egt: ThermodynamicTemperature,
}
impl Stopping {
    fn new(
        air_intake_flap: AirIntakeFlap,
        egt: ThermodynamicTemperature,
        reason: ShutdownReason,
    ) -> Stopping {
        Stopping {
            air_intake_flap,
            since: Duration::from_secs(0),
            reason,
            n: Ratio::new::<percent>(100.),
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
        _: &AuxiliaryPowerUnitOverheadPanel,
    ) -> Box<dyn ApuState> {
        self.since = self.since + context.delta;
        self.n = self.calculate_n(context);
        self.egt = calculate_cooldown_egt(self.egt, context);

        if self.n.get::<percent>() <= 7. {
            self.air_intake_flap.close();
        }

        self.air_intake_flap.update(context);

        if self.n.get::<percent>() == 0. {
            Box::new(Shutdown::new(self.air_intake_flap, self.reason, self.egt))
        } else {
            self
        }
    }

    fn get_n(&self) -> Ratio {
        self.n
    }

    fn is_air_intake_flap_fully_open(&self) -> bool {
        self.air_intake_flap.is_fully_open()
    }

    fn get_egt(&self) -> ThermodynamicTemperature {
        self.egt
    }
}

fn calculate_cooldown_egt(
    current_egt: ThermodynamicTemperature,
    context: &UpdateContext,
) -> ThermodynamicTemperature {
    const APU_COOLING_COEFFICIENT: f64 = 2.;

    if current_egt > context.ambient_temperature {
        ThermodynamicTemperature::new::<degree_celsius>(
            (current_egt.get::<degree_celsius>()
                - (APU_COOLING_COEFFICIENT * context.delta.as_secs_f64()))
            .max(context.ambient_temperature.get::<degree_celsius>()),
        )
    } else {
        current_egt
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

    fn master_is_on(&self) -> bool {
        self.master.is_on()
    }

    fn master_is_off(&self) -> bool {
        self.master.is_off()
    }

    fn start_is_on(&self) -> bool {
        self.start.is_on()
    }
}

impl Visitable for AuxiliaryPowerUnit {
    fn accept(&mut self, visitor: &mut Box<dyn crate::visitor::MutableVisitor>) {
        visitor.visit_auxiliary_power_unit(self);
    }
}

#[derive(Debug, PartialEq)]
enum AirIntakeFlapTarget {
    Open,
    Closed,
}

#[derive(Debug)]
struct AirIntakeFlap {
    state: Ratio,
    target: AirIntakeFlapTarget,
    delay: i32,
}
impl AirIntakeFlap {
    fn new() -> AirIntakeFlap {
        let mut rng = rand::thread_rng();
        let delay = 3 + rng.gen_range(0..13);

        AirIntakeFlap {
            state: Ratio::new::<percent>(0.),
            target: AirIntakeFlapTarget::Closed,
            delay,
        }
    }

    fn update(&mut self, context: &UpdateContext) {
        if self.target == AirIntakeFlapTarget::Open && self.state < Ratio::new::<percent>(100.) {
            self.state += Ratio::new::<percent>(
                self.get_flap_change_for_delta(context)
                    .min(100. - self.state.get::<percent>()),
            );
        } else if self.target == AirIntakeFlapTarget::Closed
            && self.state > Ratio::new::<percent>(0.)
        {
            self.state -= Ratio::new::<percent>(
                self.get_flap_change_for_delta(context)
                    .max(self.state.get::<percent>()),
            );
        }
    }

    fn get_flap_change_for_delta(&self, context: &UpdateContext) -> f64 {
        100. * (context.delta.as_secs_f64() / self.delay as f64)
    }

    fn open(&mut self) {
        self.target = AirIntakeFlapTarget::Open;
    }

    fn close(&mut self) {
        self.target = AirIntakeFlapTarget::Closed;
    }

    fn is_fully_open(&self) -> bool {
        self.state == Ratio::new::<percent>(100.)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use uom::si::{length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

    use super::*;

    #[cfg(test)]
    mod apu_tests {
        use approx::{assert_relative_eq, relative_eq};

        use crate::shared::test_helpers::context_with;

        use super::*;

        #[test]
        fn when_apu_master_sw_turned_on_air_intake_flap_opens() {
            let mut apu = AuxiliaryPowerUnit::new_shutdown();
            let mut overhead = AuxiliaryPowerUnitOverheadPanel::new();
            overhead.master.push_on();

            apu.update(
                &context_with().delta(Duration::from_secs(20)).build(),
                &overhead,
            );

            assert_eq!(apu.is_air_intake_flap_fully_open(), true)
        }

        #[test]
        fn when_start_sw_on_when_air_intake_flap_fully_open_starting_sequence_commences() {
            let mut apu = AuxiliaryPowerUnit::new_shutdown();
            let mut overhead = AuxiliaryPowerUnitOverheadPanel::new();
            overhead.master.push_on();
            apu.update(
                &context_with().delta(Duration::from_secs(1_000)).build(),
                &overhead,
            );

            overhead.start.push_on();
            apu.update(
                &context_with().delta(Duration::from_secs(0)).build(),
                &overhead,
            );
            const APPROXIMATE_STARTUP_TIME: u64 = 48;
            apu.update(
                &context_with()
                    .delta(Duration::from_secs(APPROXIMATE_STARTUP_TIME))
                    .build(),
                &overhead,
            );

            assert_eq!(apu.get_n().get::<percent>(), 100.);
        }

        #[test]
        fn when_apu_not_started_egt_is_ambient() {
            const AMBIENT_TEMPERATURE: f64 = 0.;
            let mut apu = AuxiliaryPowerUnit::new_shutdown();
            let overhead = AuxiliaryPowerUnitOverheadPanel::new();
            apu.update(
                &context_with()
                    .delta(Duration::from_secs(1_000))
                    .and()
                    .ambient_temperature(ThermodynamicTemperature::new::<degree_celsius>(
                        AMBIENT_TEMPERATURE,
                    ))
                    .build(),
                &overhead,
            );

            assert_eq!(apu.get_egt().get::<degree_celsius>(), AMBIENT_TEMPERATURE);
        }

        #[test]
        fn when_apu_starting_egt_reaches_above_800_degree_celsius() {
            let mut apu = starting_apu();

            let mut max_egt: f64 = 0.;

            loop {
                apu.update(
                    &context_with().delta(Duration::from_secs(1)).build(),
                    &starting_overhead(),
                );

                let apu_egt = apu.get_egt().get::<degree_celsius>();
                if apu_egt < max_egt {
                    break;
                }

                max_egt = apu_egt;
            }

            assert!(max_egt > 800.);
        }

        #[test]
        fn egt_max_always_33_above_egt_warn() {
            let mut apu = starting_apu();

            for _ in 1..=100 {
                apu.update(
                    &context_with().delta(Duration::from_secs(1)).build(),
                    &starting_overhead(),
                );

                assert_relative_eq!(
                    apu.get_egt_maximum_temperature().get::<degree_celsius>(),
                    apu.get_egt_warning_temperature().get::<degree_celsius>() + 33.
                );
            }
        }

        #[test]
        #[ignore]
        fn start_sw_on_light_turns_off_when_n_above_99_5() {
            // Note should also test 2 seconds after reaching 95 the light turns off?
        }

        #[test]
        #[ignore]
        fn start_sw_avail_light_turns_on_when_n_above_99_5() {
            // Note should also test 2 seconds after reaching 95 the light turns off?
        }

        #[test]
        #[ignore]
        fn when_egt_is_greater_than_egt_max_automatic_shutdown_begins() {
            // Note should also test 2 seconds after reaching 95 the light turns off?
        }

        #[test]
        #[ignore]
        fn when_apu_master_sw_turned_off_avail_on_start_pb_goes_off() {}

        #[test]
        #[ignore]
        fn when_apu_master_sw_turned_off_if_apu_bleed_air_was_used_apu_keeps_running_for_60_second_cooldown(
        ) {
        }

        #[test]
        fn when_apu_shutting_down_at_7_percent_n_air_inlet_flap_closes() {
            let overhead = shutting_down_overhead();
            let mut apu = running_apu();

            loop {
                apu.update(
                    &context_with().delta(Duration::from_secs(1)).build(),
                    &overhead,
                );

                if apu.get_n().get::<percent>() <= 7. {
                    break;
                }
            }

            assert!(!apu.is_air_intake_flap_fully_open());
        }

        fn starting_apu() -> AuxiliaryPowerUnit {
            let mut apu = AuxiliaryPowerUnit::new_shutdown();
            let mut overhead = AuxiliaryPowerUnitOverheadPanel::new();
            overhead.master.push_on();
            apu.update(
                &context_with().delta(Duration::from_secs(1_000)).build(),
                &overhead,
            );

            overhead.start.push_on();

            apu
        }

        fn running_apu() -> AuxiliaryPowerUnit {
            let mut apu = AuxiliaryPowerUnit::new_shutdown();
            let mut overhead = AuxiliaryPowerUnitOverheadPanel::new();

            overhead.master.push_on();
            overhead.start.push_on();

            loop {
                apu.update(
                    &context_with().delta(Duration::from_secs(1)).build(),
                    &overhead,
                );
                if apu.is_running() {
                    break;
                }
            }

            apu
        }

        fn starting_overhead() -> AuxiliaryPowerUnitOverheadPanel {
            let mut overhead = AuxiliaryPowerUnitOverheadPanel::new();
            overhead.master.push_on();
            overhead.start.push_on();

            overhead
        }

        fn shutting_down_overhead() -> AuxiliaryPowerUnitOverheadPanel {
            AuxiliaryPowerUnitOverheadPanel::new()
        }
    }

    #[cfg(test)]
    mod air_intake_flap_tests {
        use crate::shared::test_helpers::context_with;

        use super::*;

        #[test]
        fn starts_opening_when_target_is_open() {
            let mut flap = AirIntakeFlap::new();
            flap.open();

            flap.update(&context_with().delta(Duration::from_secs(5)).build());

            assert!(flap.state.get::<percent>() > 0.);
        }

        #[test]
        fn closes_when_target_is_closed() {
            let mut flap = AirIntakeFlap::new();
            flap.open();
            flap.update(&context_with().delta(Duration::from_secs(5)).build());
            let open_percentage = flap.state.get::<percent>();

            flap.close();
            flap.update(&context_with().delta(Duration::from_secs(2)).build());

            assert!(flap.state.get::<percent>() < open_percentage);
        }

        #[test]
        fn never_closes_beyond_0_percent() {
            let mut flap = AirIntakeFlap::new();
            flap.close();
            flap.update(&context_with().delta(Duration::from_secs(1_000)).build());

            assert_eq!(flap.state.get::<percent>(), 0.);
        }

        #[test]
        fn never_opens_beyond_100_percent() {
            let mut flap = AirIntakeFlap::new();
            flap.open();
            flap.update(&context_with().delta(Duration::from_secs(1_000)).build());

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
            flap.open();
            flap.update(&context_with().delta(Duration::from_secs(1_000)).build());

            assert_eq!(flap.is_fully_open(), true)
        }
    }
}
