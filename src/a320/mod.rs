use uom::si::{f32::{Ratio, Time}, ratio::percent, time::second};

use crate::{electrical::{ApuGenerator, AuxiliaryPowerUnit, Contactor, ElectricalBus, EngineGenerator, ExternalPowerSource, PowerConductor, Powerable}, shared::{DelayedTrueLogicGate, Engine, UpdateContext}};

pub struct A320ElectricalCircuit {
    engine_1_gen: EngineGenerator,
    engine_1_contactor: Contactor,
    engine_2_gen: EngineGenerator,
    engine_2_contactor: Contactor,
    bus_tie_1_contactor: Contactor,
    bus_tie_2_contactor: Contactor,
    apu_gen: ApuGenerator,
    apu_gen_contactor: Contactor,
    ext_pwr_contactor: Contactor,
    ac_bus_1: ElectricalBus,
    ac_bus_2: ElectricalBus,
    ac_ess_bus: ElectricalBus,
    ac_ess_feed_contactor_1: Contactor,
    ac_ess_feed_contactor_2: Contactor,
    ac_ess_feed_contactor_delay_logic_gate: DelayedTrueLogicGate
}

impl A320ElectricalCircuit {
    const AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS: f32 = 3.;

    pub fn new() -> A320ElectricalCircuit {
        A320ElectricalCircuit {
            engine_1_gen: EngineGenerator::new(1),
            engine_1_contactor: Contactor::new(),
            engine_2_gen: EngineGenerator::new(2),
            engine_2_contactor: Contactor::new(),
            bus_tie_1_contactor: Contactor::new(),
            bus_tie_2_contactor: Contactor::new(),
            apu_gen: ApuGenerator::new(),
            apu_gen_contactor: Contactor::new(),
            ext_pwr_contactor: Contactor::new(),
            ac_bus_1: ElectricalBus::new(),
            ac_bus_2: ElectricalBus::new(),
            ac_ess_bus: ElectricalBus::new(),
            ac_ess_feed_contactor_1: Contactor::new(),
            ac_ess_feed_contactor_2: Contactor::new(),
            ac_ess_feed_contactor_delay_logic_gate: DelayedTrueLogicGate::new(Time::new::<second>(A320ElectricalCircuit::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS)),
        }
    }

    pub fn update(&mut self, context: &UpdateContext, engine1: &Engine, engine2: &Engine, apu: &AuxiliaryPowerUnit, ext_pwr: &ExternalPowerSource) {
        self.engine_1_gen.update(engine1);
        self.engine_2_gen.update(engine2);
        self.apu_gen.update(apu);

        let gen_1_is_powered = self.engine_1_gen.output().is_powered();
        let gen_2_is_powered = self.engine_2_gen.output().is_powered();
        let apu_gen_is_powered = self.apu_gen.output().is_powered();

        self.engine_1_contactor.toggle(gen_1_is_powered);
        self.engine_2_contactor.toggle(gen_2_is_powered);

        let no_engine_gen_is_powered = !gen_1_is_powered && !gen_2_is_powered;
        let only_one_engine_gen_is_powered = gen_1_is_powered ^ gen_2_is_powered;
        let ext_pwr_is_powered = ext_pwr.output().is_powered();
        self.apu_gen_contactor.toggle(apu_gen_is_powered && !ext_pwr_is_powered && (no_engine_gen_is_powered || only_one_engine_gen_is_powered));
        self.ext_pwr_contactor.toggle(ext_pwr_is_powered && (no_engine_gen_is_powered || only_one_engine_gen_is_powered));

        let apu_or_ext_pwr_is_powered = ext_pwr_is_powered || apu_gen_is_powered;
        self.bus_tie_1_contactor.toggle((only_one_engine_gen_is_powered && !apu_or_ext_pwr_is_powered) || (apu_or_ext_pwr_is_powered && !gen_1_is_powered));
        self.bus_tie_2_contactor.toggle((only_one_engine_gen_is_powered && !apu_or_ext_pwr_is_powered) || (apu_or_ext_pwr_is_powered && !gen_2_is_powered));
        
        self.apu_gen_contactor.powered_by(vec!(&self.apu_gen));
        self.ext_pwr_contactor.powered_by(vec!(ext_pwr));

        self.engine_1_contactor.powered_by(vec!(&self.engine_1_gen));
        self.bus_tie_1_contactor.powered_by(vec!(&self.engine_1_contactor, &self.apu_gen_contactor, &self.ext_pwr_contactor));

        self.engine_2_contactor.powered_by(vec!(&self.engine_2_gen));
        self.bus_tie_2_contactor.powered_by(vec!(&self.engine_2_contactor, &self.apu_gen_contactor, &self.ext_pwr_contactor));
        
        self.bus_tie_1_contactor.or_powered_by(vec!(&self.bus_tie_2_contactor));
        self.bus_tie_2_contactor.or_powered_by(vec!(&self.bus_tie_1_contactor));

        self.ac_bus_1.powered_by(vec!(&self.engine_1_contactor, &self.bus_tie_1_contactor));
        self.ac_bus_2.powered_by(vec!(&self.engine_2_contactor, &self.bus_tie_2_contactor));

        self.ac_ess_feed_contactor_delay_logic_gate.update(context, self.ac_bus_1.output().is_unpowered());

        self.ac_ess_feed_contactor_1.toggle(!self.ac_ess_feed_contactor_delay_logic_gate.output());
        self.ac_ess_feed_contactor_2.toggle(self.ac_ess_feed_contactor_delay_logic_gate.output());

        self.ac_ess_feed_contactor_1.powered_by(vec!(&self.ac_bus_1));
        self.ac_ess_feed_contactor_2.powered_by(vec!(&self.ac_bus_2));

        self.ac_ess_bus.powered_by(vec!(&self.ac_ess_feed_contactor_1, &self.ac_ess_feed_contactor_2));
    }
}

#[cfg(test)]
mod a320_electrical_circuit_tests {
    use crate::electrical::PowerSource;

    use super::*;

    #[test]
    fn starts_without_output() {
        assert!(electrical_circuit().ac_bus_1.output().is_unpowered());
        assert!(electrical_circuit().ac_bus_2.output().is_unpowered());
        assert!(electrical_circuit().ac_ess_bus.output().is_unpowered());
    }

    #[test]
    fn when_available_engine_1_gen_supplies_ac_bus_1() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &running_engine(), &stopped_engine(), &stopped_apu(), &disconnected_external_power());

        assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::EngineGenerator(1));
    }

    #[test]
    fn when_available_engine_2_gen_supplies_ac_bus_2() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &stopped_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::EngineGenerator(2));
    }

    #[test]
    fn when_only_engine_1_is_running_supplies_ac_bus_2() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &running_engine(), &stopped_engine(), &stopped_apu(), &disconnected_external_power());

        assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::EngineGenerator(1));
    }

    #[test]
    fn when_only_engine_2_is_running_supplies_ac_bus_1() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &stopped_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::EngineGenerator(2));
    }

    #[test]
    fn when_no_power_source_ac_bus_1_is_unpowered() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &stopped_engine(), &stopped_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.ac_bus_1.output().is_unpowered());
    }

    #[test]
    fn when_no_power_source_ac_bus_2_is_unpowered() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &stopped_engine(), &stopped_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.ac_bus_2.output().is_unpowered());
    }

    #[test]
    fn when_engine_1_and_apu_running_apu_powers_ac_bus_2() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &running_engine(), &stopped_engine(), &running_apu(), &disconnected_external_power());

        assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::ApuGenerator);
    }

    #[test]
    fn when_engine_2_and_apu_running_apu_powers_ac_bus_1() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &stopped_engine(), &running_engine(), &running_apu(), &disconnected_external_power());

        assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::ApuGenerator);
    }        

    #[test]
    fn when_only_apu_running_apu_powers_ac_bus_1_and_2() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &stopped_engine(), &stopped_engine(), &running_apu(), &disconnected_external_power());

        assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::ApuGenerator);
        assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::ApuGenerator);
    }
    
    #[test]
    fn when_engine_1_running_and_external_power_connected_ext_pwr_powers_ac_bus_2() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &running_engine(), &stopped_engine(), &stopped_apu(), &connected_external_power());

        assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::External);
    }

    #[test]
    fn when_engine_2_running_and_external_power_connected_ext_pwr_powers_ac_bus_1() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &stopped_engine(), &running_engine(), &stopped_apu(), &connected_external_power());

        assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::External);
    }

    #[test]
    fn when_only_external_power_connected_ext_pwr_powers_ac_bus_1_and_2() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &stopped_engine(), &stopped_engine(), &stopped_apu(), &connected_external_power());

        assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::External);
        assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::External);
    }

    #[test]
    fn when_external_power_connected_and_apu_running_external_power_has_priority() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &stopped_engine(), &stopped_engine(), &running_apu(), &connected_external_power());

        assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::External);
        assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::External);
    }

    #[test]
    fn when_both_engines_running_and_external_power_connected_engines_power_ac_buses() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &connected_external_power());

        assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::EngineGenerator(1));
        assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::EngineGenerator(2));
    }

    #[test]
    fn when_both_engines_running_and_apu_running_engines_power_ac_buses() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &running_engine(), &running_engine(), &running_apu(), &disconnected_external_power());

        assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::EngineGenerator(1));
        assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::EngineGenerator(2));
    }

    #[test]
    fn ac_bus_1_powers_ac_ess_bus_whenever_it_is_powered() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());
        
        assert_eq!(circuit.ac_ess_bus.output().get_source(), PowerSource::EngineGenerator(1));
    }

    #[test]
    fn when_ac_bus_1_becomes_unpowered_nothing_powers_ac_ess_bus_for_three_seconds() {
        let mut circuit = electrical_circuit();
        circuit.ac_bus_1.fail();
        timed_update_circuit(&mut circuit, Time::new::<second>(A320ElectricalCircuit::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS - 0.01),
            &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.ac_ess_bus.output().is_unpowered());
    }

    #[test]
    fn after_three_seconds_of_ac_bus_1_being_unpowered_ac_bus_2_powers_ac_ess_bus() {
        let mut circuit = electrical_circuit();
        circuit.ac_bus_1.fail();
        timed_update_circuit(&mut circuit, Time::new::<second>(A320ElectricalCircuit::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS), 
            &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert_eq!(circuit.ac_ess_bus.output().get_source(), PowerSource::EngineGenerator(2));
    }

    #[test]
    fn ac_bus_1_powers_ac_ess_bus_immediately_when_ac_bus_1_becomes_powered_after_ac_bus_2_was_powering_ac_ess_bus() {
        let mut circuit = electrical_circuit();
        circuit.ac_bus_1.fail();
        timed_update_circuit(&mut circuit, Time::new::<second>(A320ElectricalCircuit::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS), 
            &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());
        circuit.ac_bus_1.normal();
        timed_update_circuit(&mut circuit, Time::new::<second>(0.01), 
            &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert_eq!(circuit.ac_ess_bus.output().get_source(), PowerSource::EngineGenerator(1));
    }

    #[test]
    // For now...
    fn nothing_powers_ac_ess_bus_when_ac_bus_1_and_2_unpowered() {
        let mut circuit = electrical_circuit();
        circuit.ac_bus_1.fail();
        circuit.ac_bus_2.fail();
        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.ac_ess_bus.output().is_unpowered());
    }

    fn electrical_circuit() -> A320ElectricalCircuit {
        A320ElectricalCircuit::new()
    }

    fn timed_update_circuit(circuit: &mut A320ElectricalCircuit, delta: Time, engine1: &Engine, engine2: &Engine, apu: &AuxiliaryPowerUnit, ext_pwr: &ExternalPowerSource) {
        let context = UpdateContext::new(delta);
        circuit.update(&context, &engine1, &engine2, &apu, &ext_pwr);
    }

    fn update_circuit(circuit: &mut A320ElectricalCircuit, engine1: &Engine, engine2: &Engine, apu: &AuxiliaryPowerUnit, ext_pwr: &ExternalPowerSource) {
        timed_update_circuit(circuit, Time::new::<second>(1.), engine1, engine2, apu, ext_pwr);
    }

    fn running_engine() -> Engine {
        let mut engine = Engine::new();
        engine.n2 = Ratio::new::<percent>(EngineGenerator::ENGINE_N2_POWER_OUTPUT_THRESHOLD + 1.);

        engine
    }

    fn stopped_engine() -> Engine {
        let mut engine = Engine::new();
        engine.n2 = Ratio::new::<percent>(0.);

        engine
    }

    fn stopped_apu() -> AuxiliaryPowerUnit {
        let mut apu = AuxiliaryPowerUnit::new();
        apu.speed = Ratio::new::<percent>(0.);

        apu
    }

    fn running_apu() -> AuxiliaryPowerUnit {
        let mut apu = AuxiliaryPowerUnit::new();
        apu.speed = Ratio::new::<percent>(ApuGenerator::APU_SPEED_POWER_OUTPUT_THRESHOLD + 1.);

        apu
    }

    fn disconnected_external_power() -> ExternalPowerSource {
        let ext_pwr = ExternalPowerSource::new();
        
        ext_pwr
    }

    fn connected_external_power() -> ExternalPowerSource {
        let mut ext_pwr = ExternalPowerSource::new();
        ext_pwr.plugged_in = true;
        
        ext_pwr
    }
}