use super::{
    consumption::PowerConsumptionReport, ElectricalStateWriter, Potential, PotentialSource,
    ProvideFrequency, ProvideLoad, ProvidePotential,
};
use crate::{
    shared::calculate_towards_target_temperature,
    simulation::{SimulationElement, SimulationElementVisitor, SimulatorWriter, UpdateContext},
};
use std::cmp::min;
use uom::si::{
    electric_potential::volt, f64::*, frequency::hertz, power::watt, ratio::percent,
    thermodynamic_temperature::degree_celsius,
};

pub trait EngineGeneratorUpdateArguments {
    fn engine_corrected_n2(&self, number: usize) -> Ratio;
    fn idg_push_button_released(&self, number: usize) -> bool;
}

pub struct EngineGenerator {
    writer: ElectricalStateWriter,
    number: usize,
    idg: IntegratedDriveGenerator,
    frequency: Frequency,
    potential: ElectricPotential,
    load: Ratio,
}
impl EngineGenerator {
    pub fn new(number: usize) -> EngineGenerator {
        EngineGenerator {
            writer: ElectricalStateWriter::new(&format!("ENG_GEN_{}", number)),
            number,
            idg: IntegratedDriveGenerator::new(number),
            frequency: Frequency::new::<hertz>(0.),
            potential: ElectricPotential::new::<volt>(0.),
            load: Ratio::new::<percent>(0.),
        }
    }

    pub fn update<T: EngineGeneratorUpdateArguments>(
        &mut self,
        context: &UpdateContext,
        arguments: &T,
    ) {
        self.idg.update(context, arguments);
    }
}
impl PotentialSource for EngineGenerator {
    fn output_potential(&self) -> Potential {
        if self.idg.provides_stable_power_output() {
            Potential::EngineGenerator(self.number)
        } else {
            Potential::None
        }
    }
}
impl ProvidePotential for EngineGenerator {
    fn potential(&self) -> ElectricPotential {
        self.potential
    }

    fn potential_normal(&self) -> bool {
        let volts = self.potential.get::<volt>();
        (110.0..=120.0).contains(&volts)
    }
}
impl ProvideFrequency for EngineGenerator {
    fn frequency(&self) -> Frequency {
        self.frequency
    }

    fn frequency_normal(&self) -> bool {
        let hz = self.frequency.get::<hertz>();
        (390.0..=410.0).contains(&hz)
    }
}
impl ProvideLoad for EngineGenerator {
    fn load(&self) -> Ratio {
        self.load
    }

    fn load_normal(&self) -> bool {
        self.load <= Ratio::new::<percent>(100.)
    }
}
impl SimulationElement for EngineGenerator {
    fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
        self.idg.accept(visitor);

        visitor.visit(self);
    }

    fn process_power_consumption_report<T: PowerConsumptionReport>(
        &mut self,
        report: &T,
        _: &UpdateContext,
    ) {
        self.frequency = if self.output_potential().is_powered() {
            Frequency::new::<hertz>(400.)
        } else {
            Frequency::new::<hertz>(0.)
        };
        self.potential = if self.output_potential().is_powered() {
            ElectricPotential::new::<volt>(115.)
        } else {
            ElectricPotential::new::<volt>(0.)
        };

        let power_consumption = report
            .total_consumption_of(&self.output_potential())
            .get::<watt>();
        let power_factor_correction = 0.8;
        let maximum_load = 90000.;
        self.load = Ratio::new::<percent>(
            (power_consumption * power_factor_correction / maximum_load) * 100.,
        );
    }

    fn write(&self, writer: &mut SimulatorWriter) {
        self.writer.write_alternating_with_load(self, writer);
    }
}

struct IntegratedDriveGenerator {
    oil_outlet_temperature_id: String,
    oil_outlet_temperature: ThermodynamicTemperature,
    is_connected_id: String,
    connected: bool,
    number: usize,

    time_above_threshold_in_milliseconds: u64,
}
impl IntegratedDriveGenerator {
    pub const ENGINE_N2_POWER_UP_OUTPUT_THRESHOLD: f64 = 58.;
    pub const ENGINE_N2_POWER_DOWN_OUTPUT_THRESHOLD: f64 = 56.;
    const STABILIZATION_TIME_IN_MILLISECONDS: u64 = 500;

    fn new(number: usize) -> IntegratedDriveGenerator {
        IntegratedDriveGenerator {
            oil_outlet_temperature_id: format!(
                "ELEC_ENG_GEN_{}_IDG_OIL_OUTLET_TEMPERATURE",
                number
            ),
            oil_outlet_temperature: ThermodynamicTemperature::new::<degree_celsius>(0.),
            is_connected_id: format!("ELEC_ENG_GEN_{}_IDG_IS_CONNECTED", number),
            connected: true,
            number,

            time_above_threshold_in_milliseconds: 0,
        }
    }

    pub fn update<T: EngineGeneratorUpdateArguments>(
        &mut self,
        context: &UpdateContext,
        arguments: &T,
    ) {
        if arguments.idg_push_button_released(self.number) {
            // The IDG cannot be reconnected.
            self.connected = false;
        }

        self.update_stable_time(context, arguments.engine_corrected_n2(self.number));
        self.update_temperature(
            context,
            self.get_target_temperature(context, arguments.engine_corrected_n2(self.number)),
        );
    }

    fn provides_stable_power_output(&self) -> bool {
        self.time_above_threshold_in_milliseconds
            == IntegratedDriveGenerator::STABILIZATION_TIME_IN_MILLISECONDS
    }

    fn update_stable_time(&mut self, context: &UpdateContext, corrected_n2: Ratio) {
        if !self.connected {
            self.time_above_threshold_in_milliseconds = 0;
            return;
        }

        let mut new_time = self.time_above_threshold_in_milliseconds;
        if corrected_n2
            >= Ratio::new::<percent>(IntegratedDriveGenerator::ENGINE_N2_POWER_UP_OUTPUT_THRESHOLD)
            && self.time_above_threshold_in_milliseconds
                < IntegratedDriveGenerator::STABILIZATION_TIME_IN_MILLISECONDS
        {
            new_time = self.time_above_threshold_in_milliseconds + context.delta.as_millis() as u64;
        } else if corrected_n2
            <= Ratio::new::<percent>(
                IntegratedDriveGenerator::ENGINE_N2_POWER_DOWN_OUTPUT_THRESHOLD,
            )
            && self.time_above_threshold_in_milliseconds > 0
        {
            new_time = self.time_above_threshold_in_milliseconds
                - min(
                    context.delta.as_millis() as u64,
                    self.time_above_threshold_in_milliseconds,
                );
        }

        self.time_above_threshold_in_milliseconds = clamp(
            new_time,
            0,
            IntegratedDriveGenerator::STABILIZATION_TIME_IN_MILLISECONDS,
        );
    }

    fn update_temperature(&mut self, context: &UpdateContext, target: ThermodynamicTemperature) {
        const IDG_HEATING_COEFFICIENT: f64 = 1.4;
        const IDG_COOLING_COEFFICIENT: f64 = 0.4;

        self.oil_outlet_temperature = calculate_towards_target_temperature(
            self.oil_outlet_temperature,
            target,
            if self.oil_outlet_temperature < target {
                IDG_HEATING_COEFFICIENT
            } else {
                IDG_COOLING_COEFFICIENT
            },
            context.delta,
        );
    }

    fn get_target_temperature(
        &self,
        context: &UpdateContext,
        corrected_n2: Ratio,
    ) -> ThermodynamicTemperature {
        if !self.connected {
            return context.ambient_temperature;
        }

        let mut target_idg = corrected_n2.get::<percent>() * 1.8;
        let ambient_temperature = context.ambient_temperature.get::<degree_celsius>();
        target_idg += ambient_temperature;

        // TODO improve this function with feedback @komp provides.

        ThermodynamicTemperature::new::<degree_celsius>(target_idg)
    }
}
impl SimulationElement for IntegratedDriveGenerator {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_f64(
            &self.oil_outlet_temperature_id,
            self.oil_outlet_temperature.get::<degree_celsius>(),
        );
        writer.write_bool(&self.is_connected_id, self.connected);
    }
}

/// Experimental feature copied from Rust stb lib.
fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    assert!(min <= max);
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UpdateArguments {
        engine_corrected_n2: Ratio,
        idg_push_button_released: bool,
    }
    impl UpdateArguments {
        fn new(engine_corrected_n2: Ratio, idg_push_button_released: bool) -> Self {
            Self {
                engine_corrected_n2,
                idg_push_button_released,
            }
        }
    }
    impl EngineGeneratorUpdateArguments for UpdateArguments {
        fn engine_corrected_n2(&self, _: usize) -> Ratio {
            self.engine_corrected_n2
        }

        fn idg_push_button_released(&self, _: usize) -> bool {
            self.idg_push_button_released
        }
    }

    #[cfg(test)]
    mod engine_generator_tests {
        use super::*;
        use crate::simulation::test::SimulationTestBed;

        #[test]
        fn starts_without_output() {
            assert!(engine_generator().is_unpowered());
        }

        #[test]
        fn when_engine_n2_above_threshold_provides_output() {
            let mut generator = engine_generator();
            let mut test_bed = SimulationTestBed::new();

            update_below_threshold(&mut test_bed, &mut generator);
            update_above_threshold(&mut test_bed, &mut generator);

            assert!(generator.is_powered());
        }

        #[test]
        fn when_engine_n2_below_threshold_provides_no_output() {
            let mut generator = engine_generator();
            let mut test_bed = SimulationTestBed::new();

            update_above_threshold(&mut test_bed, &mut generator);
            update_below_threshold(&mut test_bed, &mut generator);

            assert!(generator.is_unpowered());
        }

        #[test]
        fn when_idg_disconnected_provides_no_output() {
            let mut generator = engine_generator();
            let mut test_bed = SimulationTestBed::new();

            test_bed.run(&mut generator, |gen, context| {
                gen.update(
                    context,
                    &UpdateArguments::new(Ratio::new::<percent>(80.), true),
                )
            });

            assert!(generator.is_unpowered());
        }

        #[test]
        fn writes_its_state() {
            let mut engine_gen = engine_generator();
            let mut test_bed = SimulationTestBed::new();

            test_bed.run_without_update(&mut engine_gen);

            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_POTENTIAL"));
            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_POTENTIAL_NORMAL"));
            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_FREQUENCY"));
            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_FREQUENCY_NORMAL"));
            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_LOAD"));
            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_LOAD_NORMAL"));
        }

        fn engine_generator() -> EngineGenerator {
            EngineGenerator::new(1)
        }

        fn update_above_threshold(
            test_bed: &mut SimulationTestBed,
            generator: &mut EngineGenerator,
        ) {
            test_bed.run(generator, |gen, context| {
                gen.update(
                    context,
                    &UpdateArguments::new(Ratio::new::<percent>(80.), false),
                )
            });
        }

        fn update_below_threshold(
            test_bed: &mut SimulationTestBed,
            generator: &mut EngineGenerator,
        ) {
            test_bed.run(generator, |gen, context| {
                gen.update(
                    context,
                    &UpdateArguments::new(Ratio::new::<percent>(0.), false),
                )
            });
        }
    }

    #[cfg(test)]
    mod integrated_drive_generator_tests {
        use crate::simulation::test::SimulationTestBed;

        use super::*;
        use std::time::Duration;

        fn idg() -> IntegratedDriveGenerator {
            IntegratedDriveGenerator::new(1)
        }

        #[test]
        fn writes_its_state() {
            let mut idg = idg();
            let mut test_bed = SimulationTestBed::new();

            test_bed.run_without_update(&mut idg);

            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_IDG_OIL_OUTLET_TEMPERATURE"));
            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_IDG_IS_CONNECTED"));
        }

        #[test]
        fn starts_unstable() {
            assert_eq!(idg().provides_stable_power_output(), false);
        }

        #[test]
        fn becomes_stable_once_engine_above_threshold_for_500_milliseconds() {
            let mut idg = idg();
            let mut test_bed = SimulationTestBed::new_with_delta(Duration::from_millis(500));

            test_bed.run(&mut idg, |element, context| {
                element.update(
                    context,
                    &UpdateArguments::new(Ratio::new::<percent>(80.), false),
                )
            });

            assert_eq!(idg.provides_stable_power_output(), true);
        }

        #[test]
        fn does_not_become_stable_before_engine_above_threshold_for_500_milliseconds() {
            let mut idg = idg();
            let mut test_bed = SimulationTestBed::new_with_delta(Duration::from_millis(499));

            test_bed.run(&mut idg, |element, context| {
                element.update(
                    context,
                    &UpdateArguments::new(Ratio::new::<percent>(80.), false),
                )
            });

            assert_eq!(idg.provides_stable_power_output(), false);
        }

        #[test]
        fn cannot_reconnect_once_disconnected() {
            let mut idg = idg();
            let mut test_bed = SimulationTestBed::new_with_delta(Duration::from_millis(500));
            test_bed.run(&mut idg, |element, context| {
                element.update(
                    context,
                    &UpdateArguments::new(Ratio::new::<percent>(80.), true),
                )
            });

            test_bed.run(&mut idg, |element, context| {
                element.update(
                    context,
                    &UpdateArguments::new(Ratio::new::<percent>(80.), false),
                )
            });

            assert_eq!(idg.provides_stable_power_output(), false);
        }

        #[test]
        fn running_engine_warms_up_idg() {
            let mut idg = idg();
            let starting_temperature = idg.oil_outlet_temperature;
            let mut test_bed = SimulationTestBed::new_with_delta(Duration::from_secs(10));

            test_bed.run(&mut idg, |element, context| {
                element.update(
                    context,
                    &UpdateArguments::new(Ratio::new::<percent>(80.), false),
                )
            });

            assert!(idg.oil_outlet_temperature > starting_temperature);
        }

        #[test]
        fn running_engine_does_not_warm_up_idg_when_disconnected() {
            let mut idg = idg();
            let starting_temperature = idg.oil_outlet_temperature;
            let mut test_bed = SimulationTestBed::new_with_delta(Duration::from_secs(10));

            test_bed.run(&mut idg, |element, context| {
                element.update(
                    context,
                    &UpdateArguments::new(Ratio::new::<percent>(80.), true),
                )
            });

            assert_eq!(idg.oil_outlet_temperature, starting_temperature);
        }

        #[test]
        fn shutdown_engine_cools_down_idg() {
            let mut idg = idg();
            let mut test_bed = SimulationTestBed::new_with_delta(Duration::from_secs(10));

            test_bed.run(&mut idg, |element, context| {
                element.update(
                    context,
                    &UpdateArguments::new(Ratio::new::<percent>(80.), false),
                )
            });

            let starting_temperature = idg.oil_outlet_temperature;

            test_bed.run(&mut idg, |element, context| {
                element.update(
                    context,
                    &UpdateArguments::new(Ratio::new::<percent>(0.), false),
                )
            });

            assert!(idg.oil_outlet_temperature < starting_temperature);
        }
    }
}
