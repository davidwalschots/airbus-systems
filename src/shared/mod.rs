mod random;
pub use random::*;

mod update_context;
pub use update_context::*;

use std::time::Duration;
use uom::si::{f64::*, ratio::percent};

use crate::visitor::Visitable;

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
        if self.expression_result && self.delay <= self.true_duration {
            true
        } else {
            false
        }
    }
}

pub struct Engine {
    pub n2: Ratio,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            n2: Ratio::new::<percent>(0.),
        }
    }

    pub fn update(&mut self, _: &UpdateContext) {}
}

impl Visitable for Engine {
    fn accept(&mut self, visitor: &mut Box<dyn crate::visitor::MutableVisitor>) {
        visitor.visit_engine(self);
    }
}

#[cfg(test)]
mod delayed_true_logic_gate_tests {
    use self::test_helpers::context_with;

    use super::*;

    #[test]
    fn when_the_expression_is_false_returns_false() {
        let mut gate = delay_logic_gate(Duration::from_millis(100));
        gate.update(
            &context_with().delta(Duration::from_millis(0)).build(),
            false,
        );
        gate.update(
            &context_with().delta(Duration::from_millis(1_000)).build(),
            false,
        );

        assert_eq!(gate.output(), false);
    }

    #[test]
    fn when_the_expression_is_true_and_delay_hasnt_passed_returns_false() {
        let mut gate = delay_logic_gate(Duration::from_millis(10_000));
        gate.update(
            &context_with().delta(Duration::from_millis(0)).build(),
            false,
        );
        gate.update(
            &context_with().delta(Duration::from_millis(1_000)).build(),
            false,
        );

        assert_eq!(gate.output(), false);
    }

    #[test]
    fn when_the_expression_is_true_and_delay_has_passed_returns_true() {
        let mut gate = delay_logic_gate(Duration::from_millis(100));
        gate.update(
            &context_with().delta(Duration::from_millis(0)).build(),
            true,
        );
        gate.update(
            &context_with().delta(Duration::from_millis(1_000)).build(),
            true,
        );

        assert_eq!(gate.output(), true);
    }

    #[test]
    fn when_the_expression_is_true_and_becomes_false_before_delay_has_passed_returns_false_once_delay_passed(
    ) {
        let mut gate = delay_logic_gate(Duration::from_millis(1_000));
        gate.update(
            &context_with().delta(Duration::from_millis(0)).build(),
            true,
        );
        gate.update(
            &context_with().delta(Duration::from_millis(800)).build(),
            true,
        );
        gate.update(
            &context_with().delta(Duration::from_millis(100)).build(),
            false,
        );
        gate.update(
            &context_with().delta(Duration::from_millis(200)).build(),
            false,
        );

        assert_eq!(gate.output(), false);
    }

    #[test]
    fn does_not_include_delta_at_the_moment_of_expression_becoming_true() {
        let mut gate = delay_logic_gate(Duration::from_millis(1_000));
        gate.update(
            &context_with().delta(Duration::from_millis(900)).build(),
            true,
        );
        gate.update(
            &context_with().delta(Duration::from_millis(200)).build(),
            true,
        );

        assert_eq!(gate.output(), false);
    }

    fn delay_logic_gate(delay: Duration) -> DelayedTrueLogicGate {
        DelayedTrueLogicGate::new(delay)
    }
}
