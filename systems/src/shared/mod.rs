use crate::simulation::UpdateContext;
use num_derive::FromPrimitive;
use std::time::Duration;
use uom::si::{f64::*, thermodynamic_temperature::degree_celsius};

mod random;
pub use random::*;

#[derive(FromPrimitive)]
pub(crate) enum FwcFlightPhase {
    ElecPwr = 1,
    FirstEngineStarted = 2,
    FirstEngineTakeOffPower = 3,
    AtOrAboveEightyKnots = 4,
    LiftOff = 5,
    AtOrAbove1500Feet = 6,
    AtOrBelow800Feet = 7,
    TouchDown = 8,
    AtOrBelowEightyKnots = 9,
    EnginesShutdown = 10,
}

/// The delay logic gate delays the true result of a given expression by the given amount of time.
/// False results are output immediately.
pub struct DelayedTrueLogicGate {
    delay: Duration,
    expression_result: bool,
    true_duration: Duration,
}
impl DelayedTrueLogicGate {
    pub fn new(delay: Duration) -> DelayedTrueLogicGate {
        DelayedTrueLogicGate {
            delay,
            expression_result: false,
            true_duration: Duration::from_millis(0),
        }
    }

    pub fn update(&mut self, context: &UpdateContext, expression_result: bool) {
        // We do not include the delta representing the moment before the expression_result became true.
        if self.expression_result && expression_result {
            self.true_duration += context.delta;
        } else {
            self.true_duration = Duration::from_millis(0);
        }

        self.expression_result = expression_result;
    }

    pub fn output(&self) -> bool {
        self.expression_result && self.delay <= self.true_duration
    }
}

/// Given a current and target temperature, takes a coefficient and delta to
/// determine the new temperature after a certain duration has passed.
pub(crate) fn calculate_towards_target_temperature(
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

#[cfg(test)]
mod delayed_true_logic_gate_tests {
    use super::*;
    use crate::simulation::test::SimulationTestBed;
    use crate::simulation::{Aircraft, SimulationElement};

    struct TestAircraft {
        gate: DelayedTrueLogicGate,
        expression_result: bool,
    }
    impl TestAircraft {
        fn new(gate: DelayedTrueLogicGate) -> Self {
            Self {
                gate,
                expression_result: false,
            }
        }

        fn set_expression(&mut self, value: bool) {
            self.expression_result = value;
        }

        fn gate_output(&self) -> bool {
            self.gate.output()
        }
    }
    impl Aircraft for TestAircraft {
        fn update_before_power_distribution(&mut self, context: &UpdateContext) {
            self.gate.update(context, self.expression_result);
        }
    }
    impl SimulationElement for TestAircraft {}

    #[test]
    fn when_the_expression_is_false_returns_false() {
        let mut aircraft = TestAircraft::new(DelayedTrueLogicGate::new(Duration::from_millis(100)));
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_millis(0));

        test_bed.run_aircraft(&mut aircraft);
        test_bed
            .delta(Duration::from_millis(1_000))
            .run_aircraft(&mut aircraft);

        assert_eq!(aircraft.gate_output(), false);
    }

    #[test]
    fn when_the_expression_is_true_and_delay_hasnt_passed_returns_false() {
        let mut aircraft =
            TestAircraft::new(DelayedTrueLogicGate::new(Duration::from_millis(10_000)));
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_millis(0));

        aircraft.set_expression(true);

        test_bed.run_aircraft(&mut aircraft);
        test_bed
            .delta(Duration::from_millis(1_000))
            .run_aircraft(&mut aircraft);

        assert_eq!(aircraft.gate_output(), false);
    }

    #[test]
    fn when_the_expression_is_true_and_delay_has_passed_returns_true() {
        let mut aircraft = TestAircraft::new(DelayedTrueLogicGate::new(Duration::from_millis(100)));
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_millis(0));

        aircraft.set_expression(true);

        test_bed.run_aircraft(&mut aircraft);
        test_bed
            .delta(Duration::from_millis(1_000))
            .run_aircraft(&mut aircraft);

        assert_eq!(aircraft.gate_output(), true);
    }

    #[test]
    fn when_the_expression_is_true_and_becomes_false_before_delay_has_passed_returns_false_once_delay_passed(
    ) {
        let mut aircraft =
            TestAircraft::new(DelayedTrueLogicGate::new(Duration::from_millis(1_000)));
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_millis(0));

        aircraft.set_expression(true);
        test_bed.run_aircraft(&mut aircraft);
        test_bed = test_bed.delta(Duration::from_millis(800));
        test_bed.run_aircraft(&mut aircraft);

        aircraft.set_expression(false);
        test_bed = test_bed.delta(Duration::from_millis(100));
        test_bed.run_aircraft(&mut aircraft);
        test_bed = test_bed.delta(Duration::from_millis(200));
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(aircraft.gate_output(), false);
    }

    #[test]
    fn does_not_include_delta_at_the_moment_of_expression_becoming_true() {
        let mut aircraft =
            TestAircraft::new(DelayedTrueLogicGate::new(Duration::from_millis(1_000)));
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_millis(900));

        aircraft.set_expression(true);
        test_bed.run_aircraft(&mut aircraft);

        test_bed = test_bed.delta(Duration::from_millis(200));
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(aircraft.gate_output(), false);
    }
}

#[cfg(test)]
mod calculate_towards_target_temperature_tests {
    use ntest::assert_about_eq;

    use super::*;

    #[test]
    fn when_current_equals_target_returns_current() {
        let temperature = ThermodynamicTemperature::new::<degree_celsius>(10.);
        let result = calculate_towards_target_temperature(
            temperature,
            temperature,
            1.,
            Duration::from_secs(1),
        );

        assert_eq!(result, temperature);
    }

    #[test]
    fn when_current_less_than_target_moves_towards_target() {
        let result = calculate_towards_target_temperature(
            ThermodynamicTemperature::new::<degree_celsius>(10.),
            ThermodynamicTemperature::new::<degree_celsius>(15.),
            1.,
            Duration::from_secs(1),
        );

        assert_about_eq!(result.get::<degree_celsius>(), 11.);
    }

    #[test]
    fn when_current_slightly_less_than_target_does_not_overshoot_target() {
        let result = calculate_towards_target_temperature(
            ThermodynamicTemperature::new::<degree_celsius>(14.9),
            ThermodynamicTemperature::new::<degree_celsius>(15.),
            1.,
            Duration::from_secs(1),
        );

        assert_about_eq!(result.get::<degree_celsius>(), 15.);
    }

    #[test]
    fn when_current_more_than_target_moves_towards_target() {
        let result = calculate_towards_target_temperature(
            ThermodynamicTemperature::new::<degree_celsius>(15.),
            ThermodynamicTemperature::new::<degree_celsius>(10.),
            1.,
            Duration::from_secs(1),
        );

        assert_about_eq!(result.get::<degree_celsius>(), 14.);
    }

    #[test]
    fn when_current_slightly_more_than_target_does_not_undershoot_target() {
        let result = calculate_towards_target_temperature(
            ThermodynamicTemperature::new::<degree_celsius>(10.1),
            ThermodynamicTemperature::new::<degree_celsius>(10.),
            1.,
            Duration::from_secs(1),
        );

        assert_about_eq!(result.get::<degree_celsius>(), 10.);
    }
}
