use super::{
    Current, ElectricPowerSource, ElectricSource, ElectricalStateWriter, PowerConsumptionState,
    Powerable, ProvideCurrent, ProvidePotential,
};
use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorWriteState,
};
use uom::si::{electric_current::ampere, electric_potential::volt, f64::*};

pub struct TransformerRectifier {
    writer: ElectricalStateWriter,
    number: usize,
    input: Current,
    failed: bool,
}
impl TransformerRectifier {
    pub fn new(number: usize) -> TransformerRectifier {
        TransformerRectifier {
            writer: ElectricalStateWriter::new(&format!("TR_{}", number)),
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
impl ProvideCurrent for TransformerRectifier {
    fn get_current(&self) -> ElectricCurrent {
        // TODO: Replace with actual values once calculated.
        if self.output().is_powered() {
            ElectricCurrent::new::<ampere>(150.)
        } else {
            ElectricCurrent::new::<ampere>(0.)
        }
    }

    fn get_current_normal(&self) -> bool {
        // TODO: Replace with actual values once calculated.
        if self.output().is_powered() {
            true
        } else {
            false
        }
    }
}
impl ProvidePotential for TransformerRectifier {
    fn get_potential(&self) -> ElectricPotential {
        // TODO: Replace with actual values once calculated.
        if self.output().is_powered() {
            ElectricPotential::new::<volt>(28.)
        } else {
            ElectricPotential::new::<volt>(0.)
        }
    }

    fn get_potential_normal(&self) -> bool {
        // TODO: Replace with actual values once calculated.
        if self.output().is_powered() {
            true
        } else {
            false
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
        self.writer.write_direct(self, state);
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

    #[test]
    fn writes_its_state() {
        let transformer_rectifier = transformer_rectifier();
        let mut state = SimulatorWriteState::new();

        transformer_rectifier.write(&mut state);

        assert!(state.len_is(4));
        assert!(state.contains_f64("ELEC_TR_1_CURRENT", 0.));
        assert!(state.contains_bool("ELEC_TR_1_CURRENT_NORMAL", false));
        assert!(state.contains_f64("ELEC_TR_1_POTENTIAL", 0.));
        assert!(state.contains_bool("ELEC_TR_1_POTENTIAL_NORMAL", false));
    }

    fn transformer_rectifier() -> TransformerRectifier {
        TransformerRectifier::new(1)
    }
}
