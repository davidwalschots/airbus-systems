use super::{
    ElectricalStateWriter, Potential, PotentialSource, PotentialTarget, PowerConsumption,
    PowerConsumptionReport, ProvideCurrent, ProvidePotential,
};
use crate::simulation::{SimulationElement, SimulatorWriter, UpdateContext};
use uom::si::{electric_current::ampere, electric_potential::volt, f64::*};

pub struct TransformerRectifier {
    writer: ElectricalStateWriter,
    number: usize,
    input: Potential,
    failed: bool,
    potential: ElectricPotential,
    current: ElectricCurrent,
}
impl TransformerRectifier {
    pub fn new(number: usize) -> TransformerRectifier {
        TransformerRectifier {
            writer: ElectricalStateWriter::new(&format!("TR_{}", number)),
            number,
            input: Potential::None,
            failed: false,
            potential: ElectricPotential::new::<volt>(0.),
            current: ElectricCurrent::new::<ampere>(0.),
        }
    }

    pub fn fail(&mut self) {
        self.failed = true;
    }

    pub fn input_potential(&self) -> Potential {
        self.input
    }
}
potential_target!(TransformerRectifier);
impl PotentialSource for TransformerRectifier {
    fn output_potential(&self) -> Potential {
        if self.failed {
            Potential::None
        } else if self.input.is_powered() {
            Potential::TransformerRectifier(self.number)
        } else {
            Potential::None
        }
    }
}
impl ProvideCurrent for TransformerRectifier {
    fn current(&self) -> ElectricCurrent {
        self.current
    }

    fn current_normal(&self) -> bool {
        self.current > ElectricCurrent::new::<ampere>(5.)
    }
}
impl ProvidePotential for TransformerRectifier {
    fn potential(&self) -> ElectricPotential {
        self.potential
    }

    fn potential_normal(&self) -> bool {
        let volts = self.potential.get::<volt>();
        (25.0..=31.0).contains(&volts)
    }
}
impl SimulationElement for TransformerRectifier {
    fn write(&self, writer: &mut SimulatorWriter) {
        self.writer.write_direct(self, writer);
    }

    fn consume_power_in_converters(&mut self, consumption: &mut PowerConsumption) {
        let dc_consumption = consumption.total_consumption_of(&self.output_potential());

        // Add the DC consumption to the TRs input (AC) consumption.
        consumption.add(&self.input, dc_consumption);
    }

    fn process_power_consumption_report<T: PowerConsumptionReport>(
        &mut self,
        report: &T,
        _: &UpdateContext,
    ) {
        self.potential = if self.output_potential().is_powered() {
            ElectricPotential::new::<volt>(28.)
        } else {
            ElectricPotential::new::<volt>(0.)
        };

        let consumption = report.total_consumption_of(&self.output_potential());
        self.current = consumption / self.potential;
    }
}

#[cfg(test)]
mod transformer_rectifier_tests {
    use crate::simulation::test::TestReaderWriter;

    use super::*;

    struct Powerless {}
    impl PotentialSource for Powerless {
        fn output_potential(&self) -> Potential {
            Potential::None
        }
    }

    struct StubApuGenerator {}
    impl PotentialSource for StubApuGenerator {
        fn output_potential(&self) -> Potential {
            Potential::ApuGenerator(1)
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
    fn when_powered_outputs_potential() {
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
        let mut test_writer = TestReaderWriter::new();
        let mut writer = SimulatorWriter::new(&mut test_writer);

        transformer_rectifier.write(&mut writer);

        assert!(test_writer.len_is(4));
        assert!(test_writer.contains_f64("ELEC_TR_1_CURRENT", 0.));
        assert!(test_writer.contains_bool("ELEC_TR_1_CURRENT_NORMAL", false));
        assert!(test_writer.contains_f64("ELEC_TR_1_POTENTIAL", 0.));
        assert!(test_writer.contains_bool("ELEC_TR_1_POTENTIAL_NORMAL", false));
    }

    fn transformer_rectifier() -> TransformerRectifier {
        TransformerRectifier::new(1)
    }
}
