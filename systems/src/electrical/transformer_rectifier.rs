use crate::simulator::{SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor};

use super::{Current, ElectricPowerSource, ElectricSource, PowerConsumptionState, Powerable};

pub struct TransformerRectifier {
    number: u8,
    input: Current,
    failed: bool,
}
impl TransformerRectifier {
    pub fn new(number: u8) -> TransformerRectifier {
        TransformerRectifier {
            number,
            input: Current::none(),
            failed: false,
        }
    }

    #[cfg(test)]
    pub fn fail(&mut self) {
        self.failed = true;
    }
}
impl Powerable for TransformerRectifier {
    fn set_input(&mut self, current: Current) {
        self.input = current;
    }

    fn get_input(&self) -> Current {
        self.input
    }
}
impl ElectricSource for TransformerRectifier {
    fn output(&self) -> Current {
        if self.failed {
            Current::none()
        } else if self.input.is_powered() {
            Current::some(ElectricPowerSource::TransformerRectifier(self.number))
        } else {
            Current::none()
        }
    }
}
impl SimulatorElementVisitable for TransformerRectifier {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for TransformerRectifier {
    fn write_power_consumption(&mut self, state: &PowerConsumptionState) {
        // TODO
    }
}

#[cfg(test)]
mod transformer_rectifier_tests {
    use super::*;

    struct Powerless {}
    impl ElectricSource for Powerless {
        fn output(&self) -> Current {
            Current::none()
        }
    }

    struct StubApuGenerator {}
    impl ElectricSource for StubApuGenerator {
        fn output(&self) -> Current {
            Current::some(ElectricPowerSource::ApuGenerator)
        }
    }

    fn apu_generator() -> StubApuGenerator {
        StubApuGenerator {}
    }

    #[test]
    fn starts_without_output() {
        assert!(transformer_rectifier().is_unpowered());
    }

    #[test]
    fn when_powered_outputs_current() {
        let mut tr = transformer_rectifier();
        tr.powered_by(&apu_generator());

        assert!(tr.is_powered());
    }

    #[test]
    fn when_powered_but_failed_has_no_output() {
        let mut tr = transformer_rectifier();
        tr.powered_by(&apu_generator());
        tr.fail();

        assert!(tr.is_unpowered());
    }

    #[test]
    fn when_unpowered_has_no_output() {
        let mut tr = transformer_rectifier();
        tr.powered_by(&Powerless {});

        assert!(tr.is_unpowered());
    }

    fn transformer_rectifier() -> TransformerRectifier {
        TransformerRectifier::new(1)
    }
}
