use uom::si::{f32::{Ratio, Time}, ratio::percent, time::second};

use crate::{electrical::{ApuGenerator, AuxiliaryPowerUnit, Contactor, ElectricalBus, EngineGenerator, EmergencyGenerator, ExternalPowerSource, PowerConductor, Powerable, TransformerRectifier}, overhead::{self, NormalAltnPushButton, OnOffPushButton}, shared::{DelayedTrueLogicGate, Engine, UpdateContext}};

pub struct A320ElectricalCircuit {
    engine_1_gen: EngineGenerator,
    engine_1_gen_contactor: Contactor,
    engine_2_gen: EngineGenerator,
    engine_2_gen_contactor: Contactor,
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
    ac_ess_feed_contactor_delay_logic_gate: DelayedTrueLogicGate,
    // The electrical diagram lists separate contactors for each transformer rectifier.
    // As there is no button affecting the contactor, nor any logic that we know of, for now
    // the contactors are just assumed to be part of the transformer rectifiers.
    tr_1: TransformerRectifier,
    tr_2: TransformerRectifier,
    tr_ess: TransformerRectifier,
    ac_ess_to_tr_ess_contactor: Contactor,
    emergency_gen: EmergencyGenerator,
    emergency_gen_contactor: Contactor,
}

impl A320ElectricalCircuit {
    const AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS: f32 = 3.;

    pub fn new() -> A320ElectricalCircuit {
        A320ElectricalCircuit {
            engine_1_gen: EngineGenerator::new(1),
            engine_1_gen_contactor: Contactor::new(),
            engine_2_gen: EngineGenerator::new(2),
            engine_2_gen_contactor: Contactor::new(),
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
            tr_1: TransformerRectifier::new(),
            tr_2: TransformerRectifier::new(),
            tr_ess: TransformerRectifier::new(),
            ac_ess_to_tr_ess_contactor: Contactor::new(),
            emergency_gen: EmergencyGenerator::new(),
            emergency_gen_contactor: Contactor::new(),
        }
    }

    pub fn update(&mut self, context: &UpdateContext, engine1: &Engine, engine2: &Engine, apu: &AuxiliaryPowerUnit,
        ext_pwr: &ExternalPowerSource, hydraulic: &A320HydraulicCircuit, elec_overhead: &A320ElectricalOverheadPanel) {
        self.engine_1_gen.update(engine1, &elec_overhead.idg_1);
        self.engine_2_gen.update(engine2, &elec_overhead.idg_2);
        self.apu_gen.update(apu);
        self.emergency_gen.update(hydraulic.is_blue_pressurised());

        let gen_1_is_powered = self.engine_1_gen.output().is_powered();
        let gen_2_is_powered = self.engine_2_gen.output().is_powered();
        let apu_gen_is_powered = self.apu_gen.output().is_powered();

        self.engine_1_gen_contactor.toggle(elec_overhead.gen_1.is_on() && gen_1_is_powered);
        self.engine_2_gen_contactor.toggle(elec_overhead.gen_2.is_on() && gen_2_is_powered);

        let no_engine_gen_is_powered = !gen_1_is_powered && !gen_2_is_powered;
        let only_one_engine_gen_is_powered = gen_1_is_powered ^ gen_2_is_powered;
        let ext_pwr_is_powered = ext_pwr.output().is_powered();
        self.apu_gen_contactor.toggle(elec_overhead.apu_gen.is_on() && apu_gen_is_powered && !ext_pwr_is_powered && (no_engine_gen_is_powered || only_one_engine_gen_is_powered));
        self.ext_pwr_contactor.toggle(elec_overhead.ext_pwr.is_on() && (no_engine_gen_is_powered || only_one_engine_gen_is_powered));

        let apu_or_ext_pwr_is_powered = ext_pwr_is_powered || apu_gen_is_powered;
        self.bus_tie_1_contactor.toggle((only_one_engine_gen_is_powered && !apu_or_ext_pwr_is_powered) || (apu_or_ext_pwr_is_powered && !gen_1_is_powered));
        self.bus_tie_2_contactor.toggle((only_one_engine_gen_is_powered && !apu_or_ext_pwr_is_powered) || (apu_or_ext_pwr_is_powered && !gen_2_is_powered));
        
        self.apu_gen_contactor.powered_by(vec!(&self.apu_gen));
        self.ext_pwr_contactor.powered_by(vec!(ext_pwr));

        self.engine_1_gen_contactor.powered_by(vec!(&self.engine_1_gen));
        self.bus_tie_1_contactor.powered_by(vec!(&self.engine_1_gen_contactor, &self.apu_gen_contactor, &self.ext_pwr_contactor));

        self.engine_2_gen_contactor.powered_by(vec!(&self.engine_2_gen));
        self.bus_tie_2_contactor.powered_by(vec!(&self.engine_2_gen_contactor, &self.apu_gen_contactor, &self.ext_pwr_contactor));
        
        self.bus_tie_1_contactor.or_powered_by(vec!(&self.bus_tie_2_contactor));
        self.bus_tie_2_contactor.or_powered_by(vec!(&self.bus_tie_1_contactor));

        self.ac_bus_1.powered_by(vec!(&self.engine_1_gen_contactor, &self.bus_tie_1_contactor));
        self.ac_bus_2.powered_by(vec!(&self.engine_2_gen_contactor, &self.bus_tie_2_contactor));

        self.tr_1.powered_by(vec!(&self.ac_bus_1));
        self.tr_2.powered_by(vec!(&self.ac_bus_2));

        self.ac_ess_feed_contactor_delay_logic_gate.update(context, self.ac_bus_1.output().is_unpowered());

        self.ac_ess_feed_contactor_1.toggle(self.ac_bus_1.output().is_powered() && (!self.ac_ess_feed_contactor_delay_logic_gate.output() && elec_overhead.ac_ess_feed.is_normal()));
        self.ac_ess_feed_contactor_2.toggle(self.ac_bus_2.output().is_powered() && (self.ac_ess_feed_contactor_delay_logic_gate.output() || elec_overhead.ac_ess_feed.is_altn()));

        self.ac_ess_feed_contactor_1.powered_by(vec!(&self.ac_bus_1));
        self.ac_ess_feed_contactor_2.powered_by(vec!(&self.ac_bus_2));

        self.ac_ess_bus.powered_by(vec!(&self.ac_ess_feed_contactor_1, &self.ac_ess_feed_contactor_2));

        self.emergency_gen_contactor.toggle(self.ac_bus_1.output().is_unpowered() && self.ac_bus_2.output().is_unpowered());
        self.emergency_gen_contactor.powered_by(vec!(&self.emergency_gen));
        
        let ac_ess_to_tr_ess_contactor_power_sources: Vec<&dyn PowerConductor> = vec!(&self.ac_ess_bus, &self.emergency_gen_contactor);
        self.ac_ess_to_tr_ess_contactor.powered_by(ac_ess_to_tr_ess_contactor_power_sources);
        self.ac_ess_to_tr_ess_contactor.toggle(A320ElectricalCircuit::has_failed_or_is_unpowered(&self.tr_1) || A320ElectricalCircuit::has_failed_or_is_unpowered(&self.tr_2));

        self.ac_ess_bus.or_powered_by(vec!(&self.ac_ess_to_tr_ess_contactor));

        self.tr_ess.powered_by(vec!(&self.ac_ess_to_tr_ess_contactor, &self.emergency_gen_contactor));
    }

    fn has_failed_or_is_unpowered(tr: &TransformerRectifier) -> bool {
        tr.has_failed() || tr.output().is_unpowered()
    }
}

pub struct A320ElectricalOverheadPanel {
    bat_1: OnOffPushButton,
    bat_2: OnOffPushButton,
    idg_1: OnOffPushButton,
    idg_2: OnOffPushButton,
    gen_1: OnOffPushButton,
    gen_2: OnOffPushButton,
    apu_gen: OnOffPushButton,
    bus_tie: OnOffPushButton,
    ac_ess_feed: NormalAltnPushButton,
    galy_and_cab: OnOffPushButton,
    ext_pwr: OnOffPushButton,
    commercial: OnOffPushButton    
}

impl A320ElectricalOverheadPanel {
    pub fn new() -> A320ElectricalOverheadPanel {
        A320ElectricalOverheadPanel {
            bat_1: OnOffPushButton::new_on(),
            bat_2: OnOffPushButton::new_on(),
            idg_1: OnOffPushButton::new_on(),
            idg_2: OnOffPushButton::new_on(),
            gen_1: OnOffPushButton::new_on(),
            gen_2: OnOffPushButton::new_on(),
            apu_gen: OnOffPushButton::new_on(),
            bus_tie: OnOffPushButton::new_on(),
            ac_ess_feed: NormalAltnPushButton::new_normal(),
            galy_and_cab: OnOffPushButton::new_on(),
            ext_pwr: OnOffPushButton::new_on(),
            commercial: OnOffPushButton::new_on()
        }
    }
}

pub struct A320HydraulicCircuit {
    // Until hydraulic is implemented, we'll fake it with this boolean.
    blue_pressurised: bool,
}

impl A320HydraulicCircuit {
    pub fn new() -> A320HydraulicCircuit {
        A320HydraulicCircuit {
            blue_pressurised: true
        }
    }

    fn is_blue_pressurised(&self) -> bool {
        self.blue_pressurised
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
        // As the DelayedTrueLogicGate doesn't include the time before the expression (AC bus not providing power) becomes true,
        // we have to execute one update beforehand which already sets the expression to true.
        timed_update_with_running_engines(&mut circuit, Time::new::<second>(0.));
        timed_update_with_running_engines(&mut circuit, Time::new::<second>(A320ElectricalCircuit::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS - 0.01));

        assert!(circuit.ac_ess_bus.output().is_unpowered());
    }

    /// # Source
    /// Discord (komp#1821):
    /// > The fault light will extinguish after 3 seconds. That's the time delay before automatic switching is activated in case of AC BUS 1 loss.
    #[test]
    fn after_three_seconds_of_ac_bus_1_being_unpowered_ac_bus_2_powers_ac_ess_bus() {
        let mut circuit = electrical_circuit();
        circuit.ac_bus_1.fail();
        // AC ESS BUS is powered by AC BUS 2 only after a delay.
        update_circuit_waiting_for_ac_ess_feed_transition(&mut circuit);

        assert_eq!(circuit.ac_ess_bus.output().get_source(), PowerSource::EngineGenerator(2));
    }

    /// # Source
    /// Discord (komp#1821):
    /// > When AC BUS 1 is available again, it will switch back automatically without delay, unless the AC ESS FEED button is on ALTN.
    #[test]
    fn ac_bus_1_powers_ac_ess_bus_immediately_when_ac_bus_1_becomes_powered_after_ac_bus_2_was_powering_ac_ess_bus() {
        let mut circuit = electrical_circuit();
        circuit.ac_bus_1.fail();
        // AC ESS BUS is powered by AC BUS 2 only after a delay.
        update_circuit_waiting_for_ac_ess_feed_transition(&mut circuit);
        circuit.ac_bus_1.normal();
        timed_update_with_running_engines(&mut circuit, Time::new::<second>(0.01));

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

    #[test]
    fn when_gen_1_push_button_off_and_engine_running_gen_1_contactor_is_open() {
        let mut circuit = electrical_circuit();
        let mut overhead = overhead_panel();
        overhead.gen_1.push_off();
        update_circuit_with_overhead(&mut circuit, &overhead, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.engine_1_gen_contactor.is_open());
    }

    #[test]
    fn when_gen_2_push_button_off_and_engine_running_gen_2_contactor_is_open() {
        let mut circuit = electrical_circuit();
        let mut overhead = overhead_panel();
        overhead.gen_2.push_off();
        update_circuit_with_overhead(&mut circuit, &overhead, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.engine_2_gen_contactor.is_open());
    }

    #[test]
    fn when_ac_ess_feed_push_button_altn_ac_bus_2_powers_ac_ess_bus() {
        let mut circuit = electrical_circuit();
        let mut overhead = overhead_panel();
        overhead.ac_ess_feed.push_altn();
        update_circuit_with_overhead(&mut circuit, &overhead, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert_eq!(circuit.ac_ess_bus.output().get_source(), PowerSource::EngineGenerator(2));
    }

    #[test]
    fn when_only_apu_running_but_apu_gen_push_button_off_nothing_powers_ac_bus_1_and_2() {
        let mut circuit = electrical_circuit();
        let mut overhead = overhead_panel();
        overhead.apu_gen.push_off();
        update_circuit_with_overhead(&mut circuit, &overhead, &stopped_engine(), &stopped_engine(), &running_apu(), &disconnected_external_power());

        assert!(circuit.ac_bus_1.output().is_unpowered());
        assert!(circuit.ac_bus_2.output().is_unpowered());
    }

    #[test]
    fn when_only_external_power_connected_but_ext_pwr_push_button_off_nothing_powers_ac_bus_1_and_2() {
        let mut circuit = electrical_circuit();
        let mut overhead = overhead_panel();
        overhead.ext_pwr.push_off();
        update_circuit_with_overhead(&mut circuit, &overhead, &stopped_engine(), &stopped_engine(), &stopped_apu(), &connected_external_power());

        assert!(circuit.ac_bus_1.output().is_unpowered());
        assert!(circuit.ac_bus_2.output().is_unpowered());
    }

    #[test]
    fn when_ac_bus_1_powered_tr_1_is_powered() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.tr_1.output().is_powered());
    }

    #[test]
    fn when_ac_bus_1_unpowered_tr_1_is_unpowered() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &stopped_engine(), &stopped_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.tr_1.output().is_unpowered());
    }

    #[test]
    fn when_ac_bus_2_powered_tr_2_is_powered() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.tr_2.output().is_powered());
    }

    #[test]
    fn when_ac_bus_2_unpowered_tr_2_is_unpowered() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &stopped_engine(), &stopped_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.tr_2.output().is_unpowered());
    }

    #[test]
    fn when_tr_1_failed_ess_tr_powered() {
        let mut circuit = electrical_circuit();
        circuit.tr_1.fail();
        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.tr_ess.output().is_powered())
    }

    #[test]
    fn when_tr_1_unpowered_ess_tr_powered() {
        let mut circuit = electrical_circuit();
        circuit.ac_bus_1.fail();
        // AC ESS BUS which powers TR1 is only supplied with power after the delay.
        update_circuit_waiting_for_ac_ess_feed_transition(&mut circuit);

        assert!(circuit.tr_ess.output().is_powered())
    }

    #[test]
    fn when_tr_2_failed_ess_tr_powered() {
        let mut circuit = electrical_circuit();
        circuit.tr_2.fail();
        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.tr_ess.output().is_powered())
    }

    #[test]
    fn when_tr_2_unpowered_ess_tr_powered() {
        let mut circuit = electrical_circuit();
        circuit.ac_bus_2.fail();
        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.tr_ess.output().is_powered())
    }

    #[test]
    fn when_tr_1_and_2_normal_ess_tr_unpowered() {
        let mut circuit = electrical_circuit();
        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.tr_ess.output().is_unpowered())
    }

    #[test]
    fn when_ac_bus_1_and_ac_bus_2_are_lost_a_running_emergency_gen_powers_tr_ess() {
        let mut circuit = electrical_circuit();
        circuit.emergency_gen.attempt_start();
        circuit.ac_bus_1.fail();
        circuit.ac_bus_2.fail();

        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.tr_ess.output().is_powered());
        assert_eq!(circuit.tr_ess.output().get_source(), PowerSource::EmergencyGenerator);
    }

    #[test]
    fn when_ac_bus_1_and_ac_bus_2_are_lost_a_running_emergency_gen_powers_ac_ess_bus() {
        let mut circuit = electrical_circuit();
        circuit.emergency_gen.attempt_start();
        circuit.ac_bus_1.fail();
        circuit.ac_bus_2.fail();

        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.ac_ess_bus.output().is_powered());
        assert_eq!(circuit.ac_ess_bus.output().get_source(), PowerSource::EmergencyGenerator);
    }

    #[test]
    fn when_ac_bus_1_and_ac_bus_2_are_lost_neither_ac_ess_feed_contactor_is_closed() {
        let mut circuit = electrical_circuit();
        circuit.ac_bus_1.fail();
        circuit.ac_bus_2.fail();
        
        update_circuit(&mut circuit, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

        assert!(circuit.ac_ess_feed_contactor_1.is_open());
        assert!(circuit.ac_ess_feed_contactor_2.is_open());
    }

    fn electrical_circuit() -> A320ElectricalCircuit {
        A320ElectricalCircuit::new()
    }

    fn overhead_panel() -> A320ElectricalOverheadPanel {
        A320ElectricalOverheadPanel::new()
    }

    fn update_circuit_waiting_for_ac_ess_feed_transition(circuit: &mut A320ElectricalCircuit) {
        // As the DelayedTrueLogicGate doesn't include the time before the expression (AC bus not providing power) becomes true,
        // we have to execute one update beforehand which already sets the expression to true.
        timed_update_with_running_engines(circuit, Time::new::<second>(0.));
        timed_update_with_running_engines(circuit, Time::new::<second>(A320ElectricalCircuit::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS));
    }

    fn timed_update_with_running_engines(circuit: &mut A320ElectricalCircuit, delta: Time) {
        timed_update_circuit(circuit, delta, &running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());
    }

    fn timed_update_circuit(circuit: &mut A320ElectricalCircuit, delta: Time, engine1: &Engine, engine2: &Engine, apu: &AuxiliaryPowerUnit, ext_pwr: &ExternalPowerSource) {
        let context = UpdateContext::new(delta);
        circuit.update(&context, &engine1, &engine2, &apu, &ext_pwr, &A320HydraulicCircuit::new(), &A320ElectricalOverheadPanel::new());
    }

    fn update_circuit(circuit: &mut A320ElectricalCircuit, engine1: &Engine, engine2: &Engine, apu: &AuxiliaryPowerUnit, ext_pwr: &ExternalPowerSource) {
        timed_update_circuit(circuit, Time::new::<second>(1.), engine1, engine2, apu, ext_pwr);
    }

    fn update_circuit_with_overhead(circuit: &mut A320ElectricalCircuit, overhead: &A320ElectricalOverheadPanel, engine1: &Engine, engine2: &Engine, apu: &AuxiliaryPowerUnit, ext_pwr: &ExternalPowerSource) {
        let context = UpdateContext::new(Time::new::<second>(1.));
        circuit.update(&context, engine1, engine2, apu, ext_pwr, &A320HydraulicCircuit::new(), overhead);
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