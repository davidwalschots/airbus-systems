use super::{
    Current, ElectricPowerSource, ElectricSource, ElectricalStateWriter, PowerConsumptionState,
    ProvideFrequency, ProvideLoad, ProvidePotential,
};
use crate::{
    engine::Engine,
    overhead::OnOffFaultPushButton,
    simulator::{
        SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorWriteState,
        UpdateContext,
    },
};
use std::cmp::min;
use uom::si::{
    electric_potential::volt, f64::*, frequency::hertz, ratio::percent,
    thermodynamic_temperature::degree_celsius,
};

pub struct EngineGenerator {
    writer: ElectricalStateWriter,
    number: usize,
    idg: IntegratedDriveGenerator,
}
impl EngineGenerator {
    pub fn new(number: usize) -> EngineGenerator {
        EngineGenerator {
            writer: ElectricalStateWriter::new(&format!("ENGINE_GEN_{}", number)),
            number,
            idg: IntegratedDriveGenerator::new(),
        }
    }

    pub fn update(
        &mut self,
        context: &UpdateContext,
        engine: &Engine,
        idg_push_button: &OnOffFaultPushButton,
    ) {
        self.idg.update(context, engine, idg_push_button);
    }
}
impl ElectricSource for EngineGenerator {
    fn output(&self) -> Current {
        if self.idg.provides_stable_power_output() {
            Current::some(ElectricPowerSource::EngineGenerator(self.number))
        } else {
            Current::none()
        }
    }
}
impl ProvidePotential for EngineGenerator {
    fn get_potential(&self) -> ElectricPotential {
        // TODO: Replace with actual values once calculated.
        if self.output().is_powered() {
            ElectricPotential::new::<volt>(115.)
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
impl ProvideFrequency for EngineGenerator {
    fn get_frequency(&self) -> Frequency {
        // TODO: Replace with actual values once calculated.
        if self.output().is_powered() {
            Frequency::new::<hertz>(400.)
        } else {
            Frequency::new::<hertz>(0.)
        }
    }

    fn get_frequency_normal(&self) -> bool {
        // TODO: Replace with actual values once calculated.
        if self.output().is_powered() {
            true
        } else {
            false
        }
    }
}
impl ProvideLoad for EngineGenerator {
    fn get_load(&self) -> Ratio {
        // TODO: Replace with actual values once calculated.
        Ratio::new::<percent>(0.)
    }

    fn get_load_normal(&self) -> bool {
        // TODO: Replace with actual values once calculated.
        true
    }
}
impl SimulatorElementVisitable for EngineGenerator {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for EngineGenerator {
    fn write_power_consumption(&mut self, state: &PowerConsumptionState) {
        let watts =
            state.get_total_consumption_for(&ElectricPowerSource::EngineGenerator(self.number));
        // TODO
    }

    fn write(&self, state: &mut SimulatorWriteState) {
        self.writer.write_alternating_with_load(self, state);
    }
}

pub struct IntegratedDriveGenerator {
    oil_outlet_temperature: ThermodynamicTemperature,
    time_above_threshold_in_milliseconds: u64,
    connected: bool,
}
impl IntegratedDriveGenerator {
    pub const ENGINE_N2_POWER_UP_OUTPUT_THRESHOLD: f64 = 58.;
    pub const ENGINE_N2_POWER_DOWN_OUTPUT_THRESHOLD: f64 = 56.;
    const STABILIZATION_TIME_IN_MILLISECONDS: u64 = 500;

    fn new() -> IntegratedDriveGenerator {
        IntegratedDriveGenerator {
            oil_outlet_temperature: ThermodynamicTemperature::new::<degree_celsius>(0.),
            time_above_threshold_in_milliseconds: 0,
            connected: true,
        }
    }

    fn update(
        &mut self,
        context: &UpdateContext,
        engine: &Engine,
        idg_push_button: &OnOffFaultPushButton,
    ) {
        if idg_push_button.is_off() {
            // The IDG cannot be reconnected.
            self.connected = false;
        }

        self.update_stable_time(context, engine);
        self.update_temperature(context, self.get_target_temperature(context, engine));
    }

    fn provides_stable_power_output(&self) -> bool {
        self.time_above_threshold_in_milliseconds
            == IntegratedDriveGenerator::STABILIZATION_TIME_IN_MILLISECONDS
    }

    fn update_stable_time(&mut self, context: &UpdateContext, engine: &Engine) {
        if !self.connected {
            self.time_above_threshold_in_milliseconds = 0;
            return;
        }

        let mut new_time = self.time_above_threshold_in_milliseconds;
        if engine.n2
            >= Ratio::new::<percent>(IntegratedDriveGenerator::ENGINE_N2_POWER_UP_OUTPUT_THRESHOLD)
            && self.time_above_threshold_in_milliseconds
                < IntegratedDriveGenerator::STABILIZATION_TIME_IN_MILLISECONDS
        {
            new_time = self.time_above_threshold_in_milliseconds + context.delta.as_millis() as u64;
        } else if engine.n2
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

        let target_temperature = target.get::<degree_celsius>();
        let mut temperature = self.oil_outlet_temperature.get::<degree_celsius>();
        temperature += if temperature < target_temperature {
            IDG_HEATING_COEFFICIENT * context.delta.as_secs_f64()
        } else {
            -(IDG_COOLING_COEFFICIENT * context.delta.as_secs_f64())
        };

        temperature = clamp(
            temperature,
            context.ambient_temperature.get::<degree_celsius>(),
            target.get::<degree_celsius>(),
        );

        self.oil_outlet_temperature = ThermodynamicTemperature::new::<degree_celsius>(temperature);
    }

    fn get_target_temperature(
        &self,
        context: &UpdateContext,
        engine: &Engine,
    ) -> ThermodynamicTemperature {
        if !self.connected {
            return context.ambient_temperature;
        }

        let mut target_idg = engine.n2.get::<percent>() * 1.8;
        let ambient_temperature = context.ambient_temperature.get::<degree_celsius>();
        target_idg += ambient_temperature;

        // TODO improve this function with feedback @komp provides.

        ThermodynamicTemperature::new::<degree_celsius>(target_idg)
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
    use crate::engine::Engine;

    fn engine_above_threshold() -> Engine {
        engine(Ratio::new::<percent>(
            IntegratedDriveGenerator::ENGINE_N2_POWER_UP_OUTPUT_THRESHOLD + 1.,
        ))
    }

    fn engine_below_threshold() -> Engine {
        engine(Ratio::new::<percent>(
            IntegratedDriveGenerator::ENGINE_N2_POWER_DOWN_OUTPUT_THRESHOLD - 1.,
        ))
    }

    fn engine(n2: Ratio) -> Engine {
        let mut engine = Engine::new(1);
        engine.n2 = n2;

        engine
    }

    #[cfg(test)]
    mod engine_generator_tests {
        use super::*;
        use crate::{overhead::OnOffFaultPushButton, simulator::test_helpers::context_with};
        use std::time::Duration;

        #[test]
        fn starts_without_output() {
            assert!(engine_generator().is_unpowered());
        }

        #[test]
        fn when_engine_n2_above_threshold_provides_output() {
            let mut generator = engine_generator();
            update_below_threshold(&mut generator);
            update_above_threshold(&mut generator);

            assert!(generator.is_powered());
        }

        #[test]
        fn when_engine_n2_below_threshold_provides_no_output() {
            let mut generator = engine_generator();
            update_above_threshold(&mut generator);
            update_below_threshold(&mut generator);

            assert!(generator.is_unpowered());
        }

        #[test]
        fn when_idg_disconnected_provides_no_output() {
            let mut generator = engine_generator();
            generator.update(
                &context_with().delta(Duration::from_secs(0)).build(),
                &engine_above_threshold(),
                &OnOffFaultPushButton::new_off("TEST"),
            );

            assert!(generator.is_unpowered());
        }

        #[test]
        fn writes_its_state() {
            let engine_gen = engine_generator();
            let mut state = SimulatorWriteState::new();

            engine_gen.write(&mut state);

            assert!(state.len_is(6));
            assert!(state.contains_f64("ELEC_ENGINE_GEN_1_POTENTIAL", 0.));
            assert!(state.contains_bool("ELEC_ENGINE_GEN_1_POTENTIAL_NORMAL", false));
            assert!(state.contains_f64("ELEC_ENGINE_GEN_1_FREQUENCY", 0.));
            assert!(state.contains_bool("ELEC_ENGINE_GEN_1_FREQUENCY_NORMAL", false));
            assert!(state.contains_f64("ELEC_ENGINE_GEN_1_LOAD", 0.));
            assert!(state.contains_bool("ELEC_ENGINE_GEN_1_LOAD_NORMAL", true));
        }

        fn engine_generator() -> EngineGenerator {
            EngineGenerator::new(1)
        }

        fn update_above_threshold(generator: &mut EngineGenerator) {
            generator.update(
                &context_with().delta(Duration::from_secs(1)).build(),
                &engine_above_threshold(),
                &OnOffFaultPushButton::new_on("TEST"),
            );
        }

        fn update_below_threshold(generator: &mut EngineGenerator) {
            generator.update(
                &context_with().delta(Duration::from_secs(1)).build(),
                &engine_below_threshold(),
                &OnOffFaultPushButton::new_on("TEST"),
            );
        }
    }

    #[cfg(test)]
    mod integrated_drive_generator_tests {
        use std::time::Duration;

        use crate::simulator::test_helpers::context_with;

        use super::*;

        fn idg() -> IntegratedDriveGenerator {
            IntegratedDriveGenerator::new()
        }

        #[test]
        fn starts_unstable() {
            assert_eq!(idg().provides_stable_power_output(), false);
        }

        #[test]
        fn becomes_stable_once_engine_above_threshold_for_500_milliseconds() {
            let mut idg = idg();
            idg.update(
                &context_with().delta(Duration::from_millis(500)).build(),
                &engine_above_threshold(),
                &OnOffFaultPushButton::new_on("TEST"),
            );

            assert_eq!(idg.provides_stable_power_output(), true);
        }

        #[test]
        fn does_not_become_stable_before_engine_above_threshold_for_500_milliseconds() {
            let mut idg = idg();
            idg.update(
                &context_with().delta(Duration::from_millis(499)).build(),
                &engine_above_threshold(),
                &OnOffFaultPushButton::new_on("TEST"),
            );

            assert_eq!(idg.provides_stable_power_output(), false);
        }

        #[test]
        fn cannot_reconnect_once_disconnected() {
            let mut idg = idg();
            idg.update(
                &context_with().delta(Duration::from_millis(500)).build(),
                &engine_above_threshold(),
                &OnOffFaultPushButton::new_off("TEST"),
            );

            idg.update(
                &context_with().delta(Duration::from_millis(500)).build(),
                &engine_above_threshold(),
                &OnOffFaultPushButton::new_on("TEST"),
            );

            assert_eq!(idg.provides_stable_power_output(), false);
        }

        #[test]
        fn running_engine_warms_up_idg() {
            let mut idg = idg();
            let starting_temperature = idg.oil_outlet_temperature;
            idg.update(
                &context_with().delta(Duration::from_secs(10)).build(),
                &engine_above_threshold(),
                &OnOffFaultPushButton::new_on("TEST"),
            );

            assert!(idg.oil_outlet_temperature > starting_temperature);
        }

        #[test]
        fn running_engine_does_not_warm_up_idg_when_disconnected() {
            let mut idg = idg();
            let starting_temperature = idg.oil_outlet_temperature;
            idg.update(
                &context_with().delta(Duration::from_secs(10)).build(),
                &engine_above_threshold(),
                &OnOffFaultPushButton::new_off("TEST"),
            );

            assert_eq!(idg.oil_outlet_temperature, starting_temperature);
        }

        #[test]
        fn shutdown_engine_cools_down_idg() {
            let mut idg = idg();
            idg.update(
                &context_with().delta(Duration::from_secs(10)).build(),
                &engine_above_threshold(),
                &OnOffFaultPushButton::new_on("TEST"),
            );
            let starting_temperature = idg.oil_outlet_temperature;

            idg.update(
                &context_with().delta(Duration::from_secs(10)).build(),
                &Engine::new(1),
                &OnOffFaultPushButton::new_on("TEST"),
            );

            assert!(idg.oil_outlet_temperature < starting_temperature);
        }
    }
}
