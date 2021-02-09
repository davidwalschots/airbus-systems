use super::{Current, ElectricPowerSource, ElectricSource, PowerConsumptionState, Powerable};
use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorWriteState,
};
use uom::si::{electric_current::ampere, electric_potential::volt, f64::*};

pub struct TransformerRectifier {
    number: usize,
    input: Current,
    failed: bool,
}
impl TransformerRectifier {
    pub fn new(number: usize) -> TransformerRectifier {
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

    fn write(&self, state: &mut SimulatorWriteState) {
        // TODO: Replace with actual values once calculated.
        state.electrical.transformer_rectifiers[self.number - 1].current =
            if self.output().is_powered() {
                ElectricCurrent::new::<ampere>(150.)
            } else {
                ElectricCurrent::new::<ampere>(0.)
            };
        state.electrical.transformer_rectifiers[self.number - 1].current_within_normal_range =
            if self.output().is_powered() {
                true
            } else {
                false
            };
        state.electrical.transformer_rectifiers[self.number - 1].potential =
            if self.output().is_powered() {
                ElectricPotential::new::<volt>(28.)
            } else {
                ElectricPotential::new::<volt>(0.)
            };
        state.electrical.transformer_rectifiers[self.number - 1].potential_within_normal_range =
            if self.output().is_powered() {
                true
            } else {
                false
            };
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
