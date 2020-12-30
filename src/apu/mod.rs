use std::time::Duration;

use rand::prelude::*;
use uom::si::{f64::*, ratio::percent, thermodynamic_temperature::degree_celsius};

use crate::{overhead::OnOffPushButton, shared::UpdateContext, visitor::Visitable};

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

#[derive(Debug, PartialEq)]
struct ApuExhaustGasTemperature {
    value: ThermodynamicTemperature,
    warning: ThermodynamicTemperature,
    maximum: ThermodynamicTemperature,
}
impl ApuExhaustGasTemperature {
    const WARNING_MAX_TEMPERATURE: f64 = 1200.;
    const MAX_ABOVE_WARNING: f64 = 33.;

    fn new() -> ApuExhaustGasTemperature {
        ApuExhaustGasTemperature {
            value: ThermodynamicTemperature::new::<degree_celsius>(0.),
            warning: ThermodynamicTemperature::new::<degree_celsius>(
                ApuExhaustGasTemperature::WARNING_MAX_TEMPERATURE,
            ),
            maximum: ThermodynamicTemperature::new::<degree_celsius>(
                ApuExhaustGasTemperature::WARNING_MAX_TEMPERATURE
                    + ApuExhaustGasTemperature::MAX_ABOVE_WARNING,
            ),
        }
    }

    fn recalculate(
        &self,
        n: Ratio,
        state: &AuxiliaryPowerUnitState,
        context: &UpdateContext,
    ) -> ApuExhaustGasTemperature {
        let warning = self.calculate_exhaust_gas_warning_temperature(n);

        ApuExhaustGasTemperature {
            value: self.calculate_egt(n, state, context),
            warning,
            maximum: ThermodynamicTemperature::new::<degree_celsius>(
                warning.get::<degree_celsius>() + ApuExhaustGasTemperature::MAX_ABOVE_WARNING,
            ),
        }
    }

    fn calculate_egt(
        &self,
        n: Ratio,
        state: &AuxiliaryPowerUnitState,
        context: &UpdateContext,
    ) -> ThermodynamicTemperature {
        match state {
            AuxiliaryPowerUnitState::Starting { .. } => self.calculate_startup_egt(n, context),
            AuxiliaryPowerUnitState::Running { .. } => {
                self.calculate_slow_cooldown_to_running_temperature(context)
            }
            _ => self.value,
        }
    }

    fn calculate_startup_egt(&self, n: Ratio, context: &UpdateContext) -> ThermodynamicTemperature {
        const APU_N_TEMP_CONST: f64 = -96.565;
        const APU_N_TEMP_X: f64 = 28.571;
        const APU_N_TEMP_X2: f64 = 0.0884;
        const APU_N_TEMP_X3: f64 = -0.0081;
        const APU_N_TEMP_X4: f64 = 0.00005;

        let n = n.get::<percent>();

        let temperature = (APU_N_TEMP_X4 * n.powi(4))
            + (APU_N_TEMP_X3 * n.powi(3))
            + (APU_N_TEMP_X2 * n.powi(2))
            + (APU_N_TEMP_X * n)
            + APU_N_TEMP_CONST;

        ThermodynamicTemperature::new::<degree_celsius>(
            temperature.max(context.ambient_temperature.get::<degree_celsius>()),
        )
    }

    fn calculate_slow_cooldown_to_running_temperature(
        &self,
        context: &UpdateContext,
    ) -> ThermodynamicTemperature {
        let mut rng = rand::thread_rng();
        let random_target_temperature: f64 = 500. - rng.gen_range(0..13) as f64;

        if self.value.get::<degree_celsius>() > random_target_temperature {
            self.value
                - TemperatureInterval::new::<uom::si::temperature_interval::degree_celsius>(
                    0.4 * context.delta.as_secs_f64(),
                )
        } else {
            self.value
        }
    }

    fn calculate_exhaust_gas_warning_temperature(&self, n: Ratio) -> ThermodynamicTemperature {
        let x = match n.get::<percent>() {
            n if n < 11. => 1200.,
            n if n <= 15. => (-50. * n) + 1750.,
            n if n <= 65. => (-3. * n) + 1045.,
            n => (-30. / 7. * n) + 1128.6,
        };

        ThermodynamicTemperature::new::<degree_celsius>(x)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum AuxiliaryPowerUnitShutdownReason {
    Manual,
    Automatic, // Will be split further later into all kinds of reasons for automatic shutdown.
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum AuxiliaryPowerUnitState {
    Shutdown {
        reason: AuxiliaryPowerUnitShutdownReason,
    },
    Starting {
        since: Duration,
    },
    Running,
    ShuttingDown {
        since: Duration,
        reason: AuxiliaryPowerUnitShutdownReason,
    },
}

#[derive(Debug)]
pub struct AuxiliaryPowerUnit {
    pub n: Ratio,
    air_intake_flap: AirIntakeFlap,
    state: AuxiliaryPowerUnitState,
    egt: ApuExhaustGasTemperature,
}

impl AuxiliaryPowerUnit {
    pub fn new() -> AuxiliaryPowerUnit {
        AuxiliaryPowerUnit {
            n: Ratio::new::<percent>(0.),
            air_intake_flap: AirIntakeFlap::new(),
            state: AuxiliaryPowerUnitState::Shutdown {
                reason: AuxiliaryPowerUnitShutdownReason::Manual,
            },
            egt: ApuExhaustGasTemperature::new(),
        }
    }

    pub fn update(&mut self, context: &UpdateContext, overhead: &AuxiliaryPowerUnitOverheadPanel) {
        if overhead.master_is_on() {
            self.air_intake_flap.open();
        }

        self.air_intake_flap.update(context);

        self.state = self.update_state(context, overhead);
        self.egt = self.egt.recalculate(self.n, &self.state, context);
    }

    fn update_state(
        &mut self,
        context: &UpdateContext,
        overhead: &AuxiliaryPowerUnitOverheadPanel,
    ) -> AuxiliaryPowerUnitState {
        match &self.state {
            AuxiliaryPowerUnitState::Shutdown { .. }
                if self.air_intake_flap.is_fully_open()
                    && overhead.master_is_on()
                    && overhead.start_is_on() =>
            {
                self.n = AuxiliaryPowerUnit::calculate_n(context.delta);
                AuxiliaryPowerUnitState::Starting {
                    since: context.delta,
                }
            }
            AuxiliaryPowerUnitState::Shutdown { reason } => {
                AuxiliaryPowerUnitState::Shutdown { reason: *reason }
            }
            AuxiliaryPowerUnitState::Starting {
                since: time_since_start,
            } if self.n.get::<percent>() < 100. => {
                let time_since_start = *time_since_start + context.delta;
                self.n = AuxiliaryPowerUnit::calculate_n(time_since_start);
                AuxiliaryPowerUnitState::Starting {
                    since: time_since_start,
                }
            }
            AuxiliaryPowerUnitState::Starting { .. } if self.n.get::<percent>() == 100. => {
                AuxiliaryPowerUnitState::Running
            }
            AuxiliaryPowerUnitState::Running { .. } if overhead.master_is_off() => {
                AuxiliaryPowerUnitState::ShuttingDown {
                    since: Duration::from_secs(0),
                    reason: AuxiliaryPowerUnitShutdownReason::Manual,
                }
            }
            AuxiliaryPowerUnitState::ShuttingDown {
                since: time_since_shutdown,
                reason,
            } if self.n.get::<percent>() > 0. => AuxiliaryPowerUnitState::ShuttingDown {
                since: *time_since_shutdown + context.delta,
                reason: *reason,
            },
            AuxiliaryPowerUnitState::ShuttingDown { reason, .. }
                if self.n.get::<percent>() == 0. =>
            {
                AuxiliaryPowerUnitState::Shutdown { reason: *reason }
            }
            x => *x,
        }
    }

    fn calculate_n(time_since_start: Duration) -> Ratio {
        const APU_N_X: f64 = 2.375010484;
        const APU_N_X2: f64 = 0.034236847;
        const APU_N_X3: f64 = -0.007404136;
        const APU_N_X4: f64 = 0.000254;
        const APU_N_X5: f64 = -0.000002438;
        const APU_N_CONST: f64 = 0.;

        // Protect against the formula returning decreasing results when a lot of time is skipped.
        const TIME_LIMIT: f64 = 50.;
        let time_since_start = time_since_start.as_secs_f64().min(TIME_LIMIT);

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

    const AMBIENT_TEMPERATURE: f64 = 0.;

    fn context(delta: Duration) -> UpdateContext {
        UpdateContext::new(
            delta,
            Velocity::new::<knot>(250.),
            Length::new::<foot>(5000.),
            ThermodynamicTemperature::new::<degree_celsius>(AMBIENT_TEMPERATURE),
        )
    }

    #[cfg(test)]
    mod apu_tests {
        use approx::{assert_relative_eq, relative_eq};

        use super::*;

        #[test]
        fn when_apu_master_sw_turned_on_air_intake_flap_opens() {
            let mut apu = AuxiliaryPowerUnit::new();
            let mut overhead = AuxiliaryPowerUnitOverheadPanel::new();
            overhead.master.push_on();

            apu.update(&context(Duration::from_secs(5)), &overhead);

            assert!(apu.air_intake_flap.state.get::<percent>() > 0.);
        }

        #[test]
        fn when_start_sw_on_when_air_intake_flap_fully_open_starting_sequence_commences() {
            let mut apu = AuxiliaryPowerUnit::new();
            let mut overhead = AuxiliaryPowerUnitOverheadPanel::new();
            overhead.master.push_on();
            apu.update(&context(Duration::from_secs(1000)), &overhead);

            overhead.start.push_on();
            const APPROXIMATE_STARTUP_TIME: u64 = 48;
            apu.update(
                &context(Duration::from_secs(APPROXIMATE_STARTUP_TIME)),
                &overhead,
            );

            assert_eq!(apu.n.get::<percent>(), 100.);
        }

        #[test]
        fn when_apu_not_started_egt_is_ambient() {
            let mut apu = AuxiliaryPowerUnit::new();
            let overhead = AuxiliaryPowerUnitOverheadPanel::new();
            apu.update(&context(Duration::from_secs(1000)), &overhead);

            assert_eq!(apu.egt.value.get::<degree_celsius>(), AMBIENT_TEMPERATURE);
        }

        #[test]
        fn when_apu_starting_egt_reaches_above_800_degree_celsius() {
            let mut apu = starting_apu();

            let mut max_egt: f64 = 0.;

            loop {
                apu.update(&context(Duration::from_secs(1)), &starting_overhead());

                let apu_egt = apu.egt.value.get::<degree_celsius>();
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
                apu.update(&context(Duration::from_secs(1)), &starting_overhead());

                assert_relative_eq!(
                    apu.egt.maximum.get::<degree_celsius>(),
                    apu.egt.warning.get::<degree_celsius>() + 33.
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
        #[ignore]
        fn when_apu_shutting_down_at_7_percent_air_inlet_flap_closes() {}

        fn starting_apu() -> AuxiliaryPowerUnit {
            let mut apu = AuxiliaryPowerUnit::new();
            let mut overhead = AuxiliaryPowerUnitOverheadPanel::new();
            overhead.master.push_on();
            apu.update(&context(Duration::from_secs(1000)), &overhead);

            overhead.start.push_on();

            apu
        }

        fn starting_overhead() -> AuxiliaryPowerUnitOverheadPanel {
            let mut overhead = AuxiliaryPowerUnitOverheadPanel::new();
            overhead.master.push_on();
            overhead.start.push_on();

            overhead
        }
    }

    #[cfg(test)]
    mod air_intake_flap_tests {
        use super::*;

        #[test]
        fn starts_opening_when_target_is_open() {
            let mut flap = AirIntakeFlap::new();
            flap.open();

            flap.update(&context(Duration::from_secs(5)));

            assert!(flap.state.get::<percent>() > 0.);
        }

        #[test]
        fn closes_when_target_is_closed() {
            let mut flap = AirIntakeFlap::new();
            flap.open();
            flap.update(&context(Duration::from_secs(5)));
            let open_percentage = flap.state.get::<percent>();

            flap.close();
            flap.update(&context(Duration::from_secs(2)));

            assert!(flap.state.get::<percent>() < open_percentage);
        }

        #[test]
        fn never_closes_beyond_0_percent() {
            let mut flap = AirIntakeFlap::new();
            flap.close();
            flap.update(&context(Duration::from_secs(1000)));

            assert_eq!(flap.state.get::<percent>(), 0.);
        }

        #[test]
        fn never_opens_beyond_100_percent() {
            let mut flap = AirIntakeFlap::new();
            flap.open();
            flap.update(&context(Duration::from_secs(1000)));

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
            flap.update(&context(Duration::from_secs(1000)));

            assert_eq!(flap.is_fully_open(), true)
        }
    }
}
