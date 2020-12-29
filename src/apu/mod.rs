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

    fn start_is_on(&self) -> bool {
        self.start.is_on()
    }
}

#[derive(Debug, PartialEq)]
enum AuxiliaryPowerUnitState {
    Shutdown,
    Starting,
    Running,
}

#[derive(Debug)]
pub struct AuxiliaryPowerUnit {
    pub n: Ratio,
    air_intake_flap: AirIntakeFlap,
    state: AuxiliaryPowerUnitState,
    time_since_start: Duration,
    exhaust_gas_temperature: ThermodynamicTemperature,
}

impl AuxiliaryPowerUnit {
    pub fn new() -> AuxiliaryPowerUnit {
        AuxiliaryPowerUnit {
            n: Ratio::new::<percent>(0.),
            air_intake_flap: AirIntakeFlap::new(),
            state: AuxiliaryPowerUnitState::Shutdown,
            time_since_start: Duration::from_secs(0),
            exhaust_gas_temperature: ThermodynamicTemperature::new::<degree_celsius>(0.),
        }
    }

    pub fn update(&mut self, context: &UpdateContext, overhead: &AuxiliaryPowerUnitOverheadPanel) {
        if overhead.master_is_on() {
            self.air_intake_flap.open();
        } else {
            self.air_intake_flap.close();
        }

        self.air_intake_flap.update(context);

        if self.air_intake_flap.is_fully_open() && overhead.start_is_on() {
            self.execute_startup_sequence(context);
        }

        self.update_exhaust_gas_temperature(context);
    }

    fn execute_startup_sequence(&mut self, context: &UpdateContext) {
        if self.state == AuxiliaryPowerUnitState::Shutdown {
            self.time_since_start = Duration::from_secs(0);
            self.state = AuxiliaryPowerUnitState::Starting;
        }

        if self.state == AuxiliaryPowerUnitState::Starting {
            self.time_since_start += context.delta;
            self.n = self.calculate_n();
            if self.n == Ratio::new::<percent>(100.) {
                self.state = AuxiliaryPowerUnitState::Running;
                self.time_since_start = Duration::from_secs(0);
            }
        }
    }

    fn update_exhaust_gas_temperature(&mut self, context: &UpdateContext) {
        if self.state == AuxiliaryPowerUnitState::Starting {
            self.exhaust_gas_temperature = self.calculate_startup_egt(context);
        } else if self.state == AuxiliaryPowerUnitState::Running {
            self.exhaust_gas_temperature =
                self.calculate_slow_cooldown_to_running_temperature(context);
        }
    }

    fn calculate_n(&self) -> Ratio {
        const APU_N_X: f64 = 2.375010484;
        const APU_N_X2: f64 = 0.034236847;
        const APU_N_X3: f64 = -0.007404136;
        const APU_N_X4: f64 = 0.000254;
        const APU_N_X5: f64 = -0.000002438;
        const APU_N_CONST: f64 = 0.;

        let time_since_start = self.time_since_start.as_secs_f64();
        if time_since_start > 60. {
            // Protect against the formula returning decreasing results when a lot of time is skipped (if delta > 13s).
            Ratio::new::<percent>(100.)
        } else {
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

    fn calculate_startup_egt(&self, context: &UpdateContext) -> ThermodynamicTemperature {
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

    fn calculate_slow_cooldown_to_running_temperature(
        &self,
        context: &UpdateContext,
    ) -> ThermodynamicTemperature {
        let mut rng = rand::thread_rng();
        let random_target_temperature: f64 = 500. - rng.gen_range(0..13) as f64;

        if self.exhaust_gas_temperature.get::<degree_celsius>() > random_target_temperature {
            self.exhaust_gas_temperature
                - TemperatureInterval::new::<uom::si::temperature_interval::degree_celsius>(
                    0.4 * context.delta.as_secs_f64(),
                )
        } else {
            self.exhaust_gas_temperature
        }
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

            assert_eq!(
                apu.exhaust_gas_temperature.get::<degree_celsius>(),
                AMBIENT_TEMPERATURE
            );
        }

        #[test]
        fn when_apu_starting_max_egt_above_800_degree_celsius() {
            let mut apu = starting_apu();

            let mut max_egt: f64 = 0.;

            loop {
                apu.update(&context(Duration::from_secs(1)), &starting_overhead());

                let apu_egt = apu.exhaust_gas_temperature.get::<degree_celsius>();
                if apu_egt < max_egt {
                    break;
                }

                max_egt = apu_egt;
            }

            assert!(max_egt > 800.);
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
