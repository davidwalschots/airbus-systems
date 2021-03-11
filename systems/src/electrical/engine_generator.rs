use super::{
    consumption::PowerConsumptionReport, ElectricalStateWriter, Potential, PotentialOrigin,
    PotentialSource, ProvideFrequency, ProvideLoad, ProvidePotential,
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

pub const INTEGRATED_DRIVE_GENERATOR_STABILIZATION_TIME_IN_MILLISECONDS: u64 = 500;

pub struct EngineGenerator {
    writer: ElectricalStateWriter,
    number: usize,
    idg: IntegratedDriveGenerator,
    output_frequency: Frequency,
    output_potential: ElectricPotential,
    load: Ratio,
}
impl EngineGenerator {
    pub fn new(number: usize) -> EngineGenerator {
        EngineGenerator {
            writer: ElectricalStateWriter::new(&format!("ENG_GEN_{}", number)),
            number,
            idg: IntegratedDriveGenerator::new(number),
            output_frequency: Frequency::new::<hertz>(0.),
            output_potential: ElectricPotential::new::<volt>(0.),
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

    /// Indicates if the provided electricity's potential and frequency
    /// are within normal parameters. Use this to decide if the
    /// generator contactor should close.
    /// Load shouldn't be taken into account, as overloading causes an
    /// overtemperature which over time will trigger a mechanical
    /// disconnect of the generator.
    pub fn output_within_normal_parameters(&self) -> bool {
        self.frequency_normal() && self.potential_normal()
    }

    fn should_provide_output(&self) -> bool {
        self.idg.provides_stable_power_output()
    }
}
impl PotentialSource for EngineGenerator {
    fn output(&self) -> Potential {
        if self.should_provide_output() {
            Potential::single(
                PotentialOrigin::EngineGenerator(self.number),
                self.output_potential,
            )
        } else {
            Potential::none()
        }
    }
}
provide_potential!(EngineGenerator, (110.0..=120.0));
provide_frequency!(EngineGenerator, (390.0..=410.0));
provide_load!(EngineGenerator);
impl SimulationElement for EngineGenerator {
    fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
        self.idg.accept(visitor);

        visitor.visit(self);
    }

    fn process_power_consumption_report<T: PowerConsumptionReport>(&mut self, report: &T) {
        self.output_frequency = if self.should_provide_output() {
            Frequency::new::<hertz>(400.)
        } else {
            Frequency::new::<hertz>(0.)
        };

        self.output_potential = if self.should_provide_output() {
            ElectricPotential::new::<volt>(115.)
        } else {
            ElectricPotential::new::<volt>(0.)
        };

        let power_consumption = report
            .total_consumption_of(PotentialOrigin::EngineGenerator(self.number))
            .get::<watt>();
        let power_factor_correction = 0.8;
        let maximum_true_power = 90000.;
        self.load = Ratio::new::<percent>(
            (power_consumption * power_factor_correction / maximum_true_power) * 100.,
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
            == INTEGRATED_DRIVE_GENERATOR_STABILIZATION_TIME_IN_MILLISECONDS
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
                < INTEGRATED_DRIVE_GENERATOR_STABILIZATION_TIME_IN_MILLISECONDS
        {
            new_time =
                self.time_above_threshold_in_milliseconds + context.delta().as_millis() as u64;
        } else if corrected_n2
            <= Ratio::new::<percent>(
                IntegratedDriveGenerator::ENGINE_N2_POWER_DOWN_OUTPUT_THRESHOLD,
            )
            && self.time_above_threshold_in_milliseconds > 0
        {
            new_time = self.time_above_threshold_in_milliseconds
                - min(
                    context.delta().as_millis() as u64,
                    self.time_above_threshold_in_milliseconds,
                );
        }

        self.time_above_threshold_in_milliseconds = clamp(
            new_time,
            0,
            INTEGRATED_DRIVE_GENERATOR_STABILIZATION_TIME_IN_MILLISECONDS,
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
            context.delta(),
        );
    }

    fn get_target_temperature(
        &self,
        context: &UpdateContext,
        corrected_n2: Ratio,
    ) -> ThermodynamicTemperature {
        if !self.connected {
            return context.ambient_temperature();
        }

        let mut target_idg = corrected_n2.get::<percent>() * 1.8;
        let ambient_temperature = context.ambient_temperature().get::<degree_celsius>();
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
        use crate::{
            electrical::{
                consumption::{PowerConsumer, SuppliedPower},
                ElectricalBusType,
            },
            simulation::{test::SimulationTestBed, Aircraft},
        };

        struct EngineGeneratorTestBed {
            test_bed: SimulationTestBed,
        }
        impl EngineGeneratorTestBed {
            fn new() -> Self {
                Self {
                    test_bed: SimulationTestBed::new(),
                }
            }

            fn run_aircraft<T: Aircraft>(&mut self, aircraft: &mut T) {
                self.test_bed.run_aircraft(aircraft);
            }

            fn frequency_is_normal(&mut self) -> bool {
                self.test_bed.read_bool("ELEC_ENG_GEN_1_FREQUENCY_NORMAL")
            }

            fn potential_is_normal(&mut self) -> bool {
                self.test_bed.read_bool("ELEC_ENG_GEN_1_POTENTIAL_NORMAL")
            }

            fn load_is_normal(&mut self) -> bool {
                self.test_bed.read_bool("ELEC_ENG_GEN_1_LOAD_NORMAL")
            }

            fn load(&mut self) -> Ratio {
                Ratio::new::<percent>(self.test_bed.read_f64("ELEC_ENG_GEN_1_LOAD"))
            }
        }

        struct TestAircraft {
            engine_gen: EngineGenerator,
            running: bool,
            idg_push_button_released: bool,
            consumer: PowerConsumer,
        }
        impl TestAircraft {
            fn new(running: bool) -> Self {
                Self {
                    engine_gen: EngineGenerator::new(1),
                    running,
                    idg_push_button_released: false,
                    consumer: PowerConsumer::from(ElectricalBusType::AlternatingCurrent(1)),
                }
            }

            fn with_shutdown_engine() -> Self {
                TestAircraft::new(false)
            }

            fn with_running_engine() -> Self {
                TestAircraft::new(true)
            }

            fn disconnect_idg(&mut self) {
                self.idg_push_button_released = true;
            }

            fn generator_is_powered(&self) -> bool {
                self.engine_gen.is_powered()
            }

            fn power_demand(&mut self, power: Power) {
                self.consumer.demand(power);
            }

            fn generator_output_within_normal_parameters(&self) -> bool {
                self.engine_gen.output_within_normal_parameters()
            }
        }
        impl Aircraft for TestAircraft {
            fn update_before_power_distribution(&mut self, context: &UpdateContext) {
                self.engine_gen.update(
                    context,
                    &UpdateArguments::new(
                        Ratio::new::<percent>(if self.running { 80. } else { 0. }),
                        self.idg_push_button_released,
                    ),
                );
            }

            fn get_supplied_power(&mut self) -> SuppliedPower {
                let mut supplied_power = SuppliedPower::new();
                if self.engine_gen.is_powered() {
                    supplied_power.add(
                        ElectricalBusType::AlternatingCurrent(1),
                        Potential::single(
                            PotentialOrigin::EngineGenerator(1),
                            ElectricPotential::new::<volt>(115.),
                        ),
                    );
                }

                supplied_power
            }
        }
        impl SimulationElement for TestAircraft {
            fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
                self.engine_gen.accept(visitor);
                self.consumer.accept(visitor);

                visitor.visit(self);
            }
        }

        #[test]
        fn when_engine_running_provides_output() {
            let mut aircraft = TestAircraft::with_running_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(aircraft.generator_is_powered());
        }

        #[test]
        fn when_engine_shutdown_provides_no_output() {
            let mut aircraft = TestAircraft::with_shutdown_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(!aircraft.generator_is_powered());
        }

        #[test]
        fn when_engine_running_but_idg_disconnected_provides_no_output() {
            let mut aircraft = TestAircraft::with_running_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            aircraft.disconnect_idg();
            test_bed.run_aircraft(&mut aircraft);

            assert!(!aircraft.generator_is_powered());
        }

        #[test]
        fn when_engine_shutdown_frequency_not_normal() {
            let mut aircraft = TestAircraft::with_shutdown_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(!test_bed.frequency_is_normal());
        }

        #[test]
        fn when_engine_running_frequency_normal() {
            let mut aircraft = TestAircraft::with_running_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.frequency_is_normal());
        }

        #[test]
        fn when_engine_shutdown_potential_not_normal() {
            let mut aircraft = TestAircraft::with_shutdown_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(!test_bed.potential_is_normal());
        }

        #[test]
        fn when_engine_running_potential_normal() {
            let mut aircraft = TestAircraft::with_running_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.potential_is_normal());
        }

        #[test]
        fn when_engine_shutdown_has_no_load() {
            let mut aircraft = TestAircraft::with_shutdown_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert_eq!(test_bed.load(), Ratio::new::<percent>(0.));
        }

        #[test]
        fn when_engine_running_but_potential_unused_has_no_load() {
            let mut aircraft = TestAircraft::with_running_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert_eq!(test_bed.load(), Ratio::new::<percent>(0.));
        }

        #[test]
        fn when_engine_running_and_potential_used_has_load() {
            let mut aircraft = TestAircraft::with_running_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            aircraft.power_demand(Power::new::<watt>(50000.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.load() > Ratio::new::<percent>(0.));
        }

        #[test]
        fn when_load_below_maximum_it_is_normal() {
            let mut aircraft = TestAircraft::with_running_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            aircraft.power_demand(Power::new::<watt>(90000. / 0.8));
            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.load_is_normal());
        }

        #[test]
        fn when_load_exceeds_maximum_not_normal() {
            let mut aircraft = TestAircraft::with_running_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            aircraft.power_demand(Power::new::<watt>((90000. / 0.8) + 1.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(!test_bed.load_is_normal());
        }

        #[test]
        fn output_within_normal_parameters_when_load_exceeds_maximum() {
            let mut aircraft = TestAircraft::with_running_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            aircraft.power_demand(Power::new::<watt>((90000. / 0.8) + 1.));
            test_bed.run_aircraft(&mut aircraft);

            assert!(aircraft.generator_output_within_normal_parameters());
        }

        #[test]
        fn output_not_within_normal_parameters_when_engine_not_running() {
            let mut aircraft = TestAircraft::with_shutdown_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(!aircraft.generator_output_within_normal_parameters());
        }

        #[test]
        fn output_within_normal_parameters_when_engine_running() {
            let mut aircraft = TestAircraft::with_running_engine();
            let mut test_bed = EngineGeneratorTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(aircraft.generator_output_within_normal_parameters());
        }

        #[test]
        fn writes_its_state() {
            let mut aircraft = TestAircraft::with_running_engine();
            let mut test_bed = SimulationTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_POTENTIAL"));
            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_POTENTIAL_NORMAL"));
            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_FREQUENCY"));
            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_FREQUENCY_NORMAL"));
            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_LOAD"));
            assert!(test_bed.contains_key("ELEC_ENG_GEN_1_LOAD_NORMAL"));
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
