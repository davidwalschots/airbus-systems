use uom::si::{electric_current::ampere, electric_potential::volt, f32::{Frequency, ElectricPotential, ElectricCurrent, Ratio}, frequency::hertz, ratio::percent};

#[derive(Clone, Copy, Debug, PartialEq)]
enum PowerSource {
    None,
    EngineGenerator(u8),
    ApuGenerator,
    External
}

/// Represents a type of electric current.
#[derive(Clone, Copy, Debug)]
enum Current {
    Alternating(PowerSource, Frequency, ElectricPotential, ElectricCurrent),
    Direct(PowerSource, ElectricPotential, ElectricCurrent),
    None
}

impl Current {
    fn is_alternating(self) -> bool {
        if let Current::Alternating(..) = self { true } else { false }
    }

    fn is_direct(self) -> bool {
        if let Current::Direct(..) = self { true } else { false }
    }

    fn is_none(self) -> bool {
        if let Current::None = self { true } else { false }
    }

    fn get_source(self) -> PowerSource {
        match self {
            Current::Alternating(source, ..) => source,
            Current::Direct(source, ..) => source,
            _ => PowerSource::None
        }
    }
}

trait PowerConductor {
    fn output(&self) -> Current;
}

trait Powerable {
    /// Provides input power from any of the given sources. When none of the sources
    /// has any output, no input is provided.
    fn powered_by<T: PowerConductor + ?Sized>(&mut self, sources: Vec<&T>) {
        self.set_input(sources.iter().find_map(|x| {
            let output = x.output();
            match output {
                Current::None => None,
                _ => Some(output)
            }
        }).unwrap_or(Current::None));
    }

    /// Provides input power from any of the given sources. When none of the sources
    /// has any output, the already provided input is maintained.
    /// This function is useful for situations where power can flow bidirectionally between
    /// conductors, such as from ENG1 to AC BUS 2 and ENG2 to AC BUS 1.
    fn or_powered_by<T: PowerConductor + ?Sized>(&mut self, sources: Vec<&T>) {
        if let Current::None = self.get_input() {
            for source in sources {
                let output = source.output();
                if !output.is_none() {
                    self.set_input(output);
                }
            }
        }
    }

    fn set_input(&mut self, current: Current);
    fn get_input(&self) -> Current;
}

/// Represents the state of a contactor.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ContactorState {
    Open,
    Closed
}

/// Represents a contactor in a electrical power circuit.
#[derive(Debug)]
struct Contactor {
    state: ContactorState,
    input: Current,
}

impl Contactor {
    fn new() -> Contactor {
        Contactor {
            state: ContactorState::Open,
            input: Current::None,
        }
    }

    fn toggle(&mut self, should_be_closed: bool) {
        self.state = match self.state {
            ContactorState::Open if should_be_closed => ContactorState::Closed,
            ContactorState::Closed if !should_be_closed => ContactorState::Open,
            _ => self.state
        };
    }
}

impl Powerable for Contactor {
    fn set_input(&mut self, current: Current) {
        self.input = current;
    }

    fn get_input(&self) -> Current {
        self.input
    }
}

impl PowerConductor for Contactor {
    fn output(&self) -> Current {
        if let ContactorState::Closed = self.state {
            self.input
        } else {
            Current::None
        }
    }
}

struct A320ElectricalCircuit {
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
    ac_ess_feed_contactor_2: Contactor
}

impl A320ElectricalCircuit {
    fn new() -> A320ElectricalCircuit {
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
            ac_ess_feed_contactor_2: Contactor::new()
        }
    }

    fn update(&mut self, engine1: &Engine, engine2: &Engine, apu: &AuxiliaryPowerUnit, ext_pwr: &ExternalPowerSource) {
        self.engine_1_gen.update(engine1);
        self.engine_2_gen.update(engine2);
        self.apu_gen.update(apu);

        self.toggle_contactors(ext_pwr);        
        self.power_circuit(ext_pwr);
    }

    fn toggle_contactors(&mut self, ext_pwr: &ExternalPowerSource) {
        let gen_1_has_output = self.engine_1_gen.output().is_alternating();
        let gen_2_has_output = self.engine_2_gen.output().is_alternating();
        let apu_gen_has_output = self.apu_gen.output().is_alternating();

        self.engine_1_contactor.toggle(gen_1_has_output);
        self.engine_2_contactor.toggle(gen_2_has_output);

        let no_engine_gen_has_output = !gen_1_has_output && !gen_2_has_output;
        let only_one_engine_gen_has_output = gen_1_has_output ^ gen_2_has_output;
        let ext_pwr_has_output = ext_pwr.output().is_alternating();
        self.apu_gen_contactor.toggle(apu_gen_has_output && !ext_pwr_has_output && (no_engine_gen_has_output || only_one_engine_gen_has_output));
        self.ext_pwr_contactor.toggle(ext_pwr_has_output && (no_engine_gen_has_output || only_one_engine_gen_has_output));

        let apu_or_ext_pwr_has_output = ext_pwr_has_output || apu_gen_has_output;
        self.bus_tie_1_contactor.toggle((only_one_engine_gen_has_output && !apu_or_ext_pwr_has_output) || (apu_or_ext_pwr_has_output && !gen_1_has_output));
        self.bus_tie_2_contactor.toggle((only_one_engine_gen_has_output && !apu_or_ext_pwr_has_output) || (apu_or_ext_pwr_has_output && !gen_2_has_output));

        // Simulate failure of ac bus 1 or any contactors before it...
        self.ac_ess_feed_contactor_1.toggle(!self.ac_bus_1.failed);
        self.ac_ess_feed_contactor_2.toggle(self.ac_bus_1.failed && !self.ac_bus_2.failed);
    }

    fn power_circuit(&mut self, ext_pwr: &ExternalPowerSource) {
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

        self.ac_ess_feed_contactor_1.powered_by(vec!(&self.ac_bus_1));
        self.ac_ess_feed_contactor_2.powered_by(vec!(&self.ac_bus_2));

        self.ac_ess_bus.powered_by(vec!(&self.ac_ess_feed_contactor_1, &self.ac_ess_feed_contactor_2));
    }
}

struct EngineGenerator {
    number: u8,
    output: Current,
}

impl EngineGenerator {
    const ENGINE_N2_POWER_OUTPUT_THRESHOLD: f32 = 57.5;

    fn new(number: u8) -> EngineGenerator {
        EngineGenerator {
            number,
            output: Current::None,
        }
    }

    fn update(&mut self, engine: &Engine) {
        if engine.n2 > Ratio::new::<percent>(EngineGenerator::ENGINE_N2_POWER_OUTPUT_THRESHOLD) {
            self.output = Current::Alternating(PowerSource::EngineGenerator(self.number), Frequency::new::<hertz>(400.), 
                ElectricPotential::new::<volt>(115.), ElectricCurrent::new::<ampere>(782.60));
        } else {
            self.output = Current::None
        }
    }
}

impl PowerConductor for EngineGenerator {
    fn output(&self) -> Current {
        self.output
    }
}

struct Engine {
    n2: Ratio
}

impl Engine {
    fn new() -> Engine {
        Engine {
            n2: Ratio::new::<percent>(0.)
        }
    }
}

struct ApuGenerator {
    output: Current
}

impl ApuGenerator {
    const APU_SPEED_POWER_OUTPUT_THRESHOLD: f32 = 57.5;

    fn new() -> ApuGenerator {
        ApuGenerator {
            output: Current::None
        }
    }

    fn update(&mut self, apu: &AuxiliaryPowerUnit) {
        const POWER_OUTPUT_THRESHOLD: f32 = 57.5;
        if apu.speed > Ratio::new::<percent>(ApuGenerator::APU_SPEED_POWER_OUTPUT_THRESHOLD) {
            self.output = Current::Alternating(PowerSource::ApuGenerator, Frequency::new::<hertz>(400.),
                ElectricPotential::new::<volt>(115.), ElectricCurrent::new::<ampere>(782.60));
        } else {
            self.output = Current::None
        }
    }
}

impl PowerConductor for ApuGenerator {
    fn output(&self) -> Current {
        self.output
    }
}

struct AuxiliaryPowerUnit {
    speed: Ratio
}

impl AuxiliaryPowerUnit {
    fn new() -> AuxiliaryPowerUnit {
        AuxiliaryPowerUnit {
            speed: Ratio::new::<percent>(0.)
        }
    }
}

struct ExternalPowerSource {
    plugged_in: bool
}

impl ExternalPowerSource {
    fn new() -> ExternalPowerSource {
        ExternalPowerSource {
            plugged_in: false
        }
    }
}

impl PowerConductor for ExternalPowerSource {
    fn output(&self) -> Current {
        if self.plugged_in { 
            Current::Alternating(PowerSource::External, Frequency::new::<hertz>(400.), 
                ElectricPotential::new::<volt>(115.), ElectricCurrent::new::<ampere>(782.60))
        } else {
            Current::None
        }
    }
}

struct ElectricalBus {
    input: Current,
    failed: bool
}

impl ElectricalBus {
    fn new() -> ElectricalBus {
        ElectricalBus {
            input: Current::None,
            failed: false
        }
    }

    fn fail(&mut self) {
        self.failed = true;
    }
}

impl Powerable for ElectricalBus {
    fn set_input(&mut self, current: Current) {
        self.input = current;
    }

    fn get_input(&self) -> Current {
        self.input
    }
}

impl PowerConductor for ElectricalBus {
    fn output(&self) -> Current {
        if !self.failed {
            self.input
        } else {
            Current::None
        }
    }
}

mod tests {
    use super::*;

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

    #[cfg(test)]
    mod current_tests {
        use uom::si::{electric_current::ampere, electric_potential::volt, frequency::hertz};

        use super::*;

        #[test]
        fn alternating_current_is_alternating() {
            assert_eq!(alternating_current().is_alternating(), true);
        }

        #[test]
        fn alternating_current_is_not_direct() {
            assert_eq!(alternating_current().is_direct(), false);
        }

        #[test]
        fn alternating_current_is_not_none() {
            assert_eq!(alternating_current().is_none(), false);
        }

        #[test]
        fn direct_current_is_not_alternating() {
            assert_eq!(direct_current().is_alternating(), false);
        }

        #[test]
        fn direct_current_is_direct() {
            assert_eq!(direct_current().is_direct(), true);
        }

        #[test]
        fn direct_current_is_not_none() {
            assert_eq!(direct_current().is_none(), false);
        }
        
        #[test]
        fn none_current_is_not_alternating() {
            assert_eq!(none_current().is_alternating(), false);
        }

        #[test]
        fn none_current_is_not_direct() {
            assert_eq!(none_current().is_direct(), false);
        }

        #[test]
        fn none_current_is_none() {
            assert_eq!(none_current().is_none(), true);
        }

        fn alternating_current() -> Current {
            Current::Alternating(PowerSource::ApuGenerator, Frequency::new::<hertz>(0.), ElectricPotential::new::<volt>(0.), ElectricCurrent::new::<ampere>(0.))
        }

        fn direct_current() -> Current {
            Current::Direct(PowerSource::ApuGenerator, ElectricPotential::new::<volt>(0.), ElectricCurrent::new::<ampere>(0.))
        }

        fn none_current() -> Current {
            Current::None
        }
    }

    #[cfg(test)]
    mod contactor_tests {
        use super::*;
    
        #[test]
        fn contactor_starts_open() {
            assert_eq!(contactor().state, ContactorState::Open);
        }
    
        #[test]
        fn open_contactor_when_toggled_open_stays_open() {
            let mut contactor = open_contactor();
            contactor.toggle(false);
    
            assert_eq!(contactor.state, ContactorState::Open);
        }
    
        #[test]
        fn open_contactor_when_toggled_closed_closes() {
            let mut contactor = open_contactor();
            contactor.toggle(true);
    
            assert_eq!(contactor.state, ContactorState::Closed);
        }
    
        #[test]
        fn closed_contactor_when_toggled_open_opens() {
            let mut contactor = closed_contactor();
            contactor.toggle(false);
    
            assert_eq!(contactor.state, ContactorState::Open);
        }
    
        #[test]
        fn closed_contactor_when_toggled_closed_stays_closed() {
            let mut contactor = closed_contactor();
            contactor.toggle(true);
    
            assert_eq!(contactor.state, ContactorState::Closed);
        }
    
        #[test]
        fn open_contactor_has_no_output_when_powered_by_nothing() {
            contactor_has_no_output_when_powered_by_nothing(open_contactor());
        }
    
        #[test]
        fn closed_contactor_has_no_output_when_powered_by_nothing() {
            contactor_has_no_output_when_powered_by_nothing(closed_contactor());
        }
    
        fn contactor_has_no_output_when_powered_by_nothing(mut contactor: Contactor) {
            let nothing: Vec<&dyn PowerConductor> = vec![];
            contactor.powered_by(nothing);
    
            assert!(contactor.output().is_none());
        }
    
        #[test]
        fn open_contactor_has_no_output_when_powered_by_nothing_which_is_powered() {
            contactor_has_no_output_when_powered_by_nothing_which_is_powered(open_contactor());
        }
    
        #[test]
        fn closed_contactor_has_no_output_when_powered_by_nothing_which_is_powered() {
            contactor_has_no_output_when_powered_by_nothing_which_is_powered(closed_contactor());
        }
    
        fn contactor_has_no_output_when_powered_by_nothing_which_is_powered(mut contactor: Contactor) {
            contactor.powered_by(vec![&Powerless{}]);
    
            assert!(contactor.output().is_none());
        }
    
        #[test]
        fn open_contactor_has_no_output_when_powered_by_something() {
            let mut contactor = open_contactor();
            let conductors: Vec<&dyn PowerConductor> = vec![&Powerless{}, &Powered{}];
            contactor.powered_by(conductors);
    
            assert!(contactor.output().is_none());
        }
    
        #[test]
        fn closed_contactor_has_output_when_powered_by_something_which_is_powered() {
            let mut contactor = closed_contactor();
            let conductors: Vec<&dyn PowerConductor> = vec![&Powerless{}, &Powered{}];
            contactor.powered_by(conductors);
    
            assert!(contactor.output().is_alternating());
        }
    
        fn contactor() -> Contactor {
            Contactor::new()
        }
    
        fn open_contactor() -> Contactor {
            let mut contactor = contactor();
            contactor.state = ContactorState::Open;
    
            contactor
        }
    
        fn closed_contactor() -> Contactor {
            let mut contactor = contactor();
            contactor.state = ContactorState::Closed;
    
            contactor
        }
    
        struct Powerless {}
    
        impl PowerConductor for Powerless {
            fn output(&self) -> Current {
                Current::None
            }
        }
    
        struct Powered {}
    
        impl PowerConductor for Powered {
            fn output(&self) -> Current {
                Current::Alternating(PowerSource::ApuGenerator, Frequency::new::<hertz>(400.), 
                    ElectricPotential::new::<volt>(115.), ElectricCurrent::new::<ampere>(782.60))
            }
        }
    }
    
    #[cfg(test)]
    mod a320_electrical_circuit_tests {
        use super::*;
    
        #[test]
        fn starts_without_output() {
            assert!(electrical_circuit().ac_bus_1.output().is_none());
            assert!(electrical_circuit().ac_bus_2.output().is_none());
            assert!(electrical_circuit().ac_ess_bus.output().is_none());
        }
    
        #[test]
        fn when_available_engine_1_gen_supplies_ac_bus_1() {
            let mut circuit = electrical_circuit();
            circuit.update(&running_engine(), &stopped_engine(), &stopped_apu(), &disconnected_external_power());

            assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::EngineGenerator(1));
        }

        #[test]
        fn when_available_engine_2_gen_supplies_ac_bus_2() {
            let mut circuit = electrical_circuit();
            circuit.update(&stopped_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

            assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::EngineGenerator(2));
        }

        #[test]
        fn when_only_engine_1_is_running_supplies_ac_bus_2() {
            let mut circuit = electrical_circuit();
            circuit.update(&running_engine(), &stopped_engine(), &stopped_apu(), &disconnected_external_power());

            assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::EngineGenerator(1));
        }

        #[test]
        fn when_only_engine_2_is_running_supplies_ac_bus_1() {
            let mut circuit = electrical_circuit();
            circuit.update(&stopped_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());

            assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::EngineGenerator(2));
        }

        #[test]
        fn when_no_power_source_ac_bus_1_is_unpowered() {
            let mut circuit = electrical_circuit();
            circuit.update(&stopped_engine(), &stopped_engine(), &stopped_apu(), &disconnected_external_power());

            assert!(circuit.ac_bus_1.output().is_none());
        }

        #[test]
        fn when_no_power_source_ac_bus_2_is_unpowered() {
            let mut circuit = electrical_circuit();
            circuit.update(&stopped_engine(), &stopped_engine(), &stopped_apu(), &disconnected_external_power());

            assert!(circuit.ac_bus_2.output().is_none());
        }

        #[test]
        fn when_engine_1_and_apu_running_apu_powers_ac_bus_2() {
            let mut circuit = electrical_circuit();
            circuit.update(&running_engine(), &stopped_engine(), &running_apu(), &disconnected_external_power());

            assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::ApuGenerator);
        }

        #[test]
        fn when_engine_2_and_apu_running_apu_powers_ac_bus_1() {
            let mut circuit = electrical_circuit();
            circuit.update(&stopped_engine(), &running_engine(), &running_apu(), &disconnected_external_power());

            assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::ApuGenerator);
        }        

        #[test]
        fn when_only_apu_running_apu_powers_ac_bus_1_and_2() {
            let mut circuit = electrical_circuit();
            circuit.update(&stopped_engine(), &stopped_engine(), &running_apu(), &disconnected_external_power());

            assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::ApuGenerator);
            assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::ApuGenerator);
        }
        
        #[test]
        fn when_engine_1_running_and_external_power_connected_ext_pwr_powers_ac_bus_2() {
            let mut circuit = electrical_circuit();
            circuit.update(&running_engine(), &stopped_engine(), &stopped_apu(), &connected_external_power());

            assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::External);
        }

        #[test]
        fn when_engine_2_running_and_external_power_connected_ext_pwr_powers_ac_bus_1() {
            let mut circuit = electrical_circuit();
            circuit.update(&stopped_engine(), &running_engine(), &stopped_apu(), &connected_external_power());

            assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::External);
        }

        #[test]
        fn when_only_external_power_connected_ext_pwr_powers_ac_bus_1_and_2() {
            let mut circuit = electrical_circuit();
            circuit.update(&stopped_engine(), &stopped_engine(), &stopped_apu(), &connected_external_power());

            assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::External);
            assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::External);
        }

        #[test]
        fn when_external_power_connected_and_apu_running_external_power_has_priority() {
            let mut circuit = electrical_circuit();
            circuit.update(&stopped_engine(), &stopped_engine(), &running_apu(), &connected_external_power());

            assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::External);
            assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::External);
        }

        #[test]
        fn when_both_engines_running_and_external_power_connected_engines_power_ac_buses() {
            let mut circuit = electrical_circuit();
            circuit.update(&running_engine(), &running_engine(), &stopped_apu(), &connected_external_power());

            assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::EngineGenerator(1));
            assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::EngineGenerator(2));
        }

        #[test]
        fn when_both_engines_running_and_apu_running_engines_power_ac_buses() {
            let mut circuit = electrical_circuit();
            circuit.update(&running_engine(), &running_engine(), &running_apu(), &disconnected_external_power());

            assert_eq!(circuit.ac_bus_1.output().get_source(), PowerSource::EngineGenerator(1));
            assert_eq!(circuit.ac_bus_2.output().get_source(), PowerSource::EngineGenerator(2));
        }

        #[test]
        fn ac_bus_1_powers_ac_ess_bus_whenever_it_is_powered() {
            let mut circuit = electrical_circuit();
            update_with_running_engines(&mut circuit);
            
            assert_eq!(circuit.ac_ess_bus.output().get_source(), PowerSource::EngineGenerator(1));
        }

        #[test]
        fn ac_bus_2_powers_ac_ess_bus_when_ac_bus_1_failed() {
            let mut circuit = electrical_circuit();
            circuit.ac_bus_1.fail();
            update_with_running_engines(&mut circuit);
            
            assert_eq!(circuit.ac_ess_bus.output().get_source(), PowerSource::EngineGenerator(2));
        }

        #[test]
        // For now...
        fn nothing_powers_ac_ess_bus_when_ac_bus_1_and_2_failed() {
            let mut circuit = electrical_circuit();
            circuit.ac_bus_1.fail();
            circuit.ac_bus_2.fail();
            update_with_running_engines(&mut circuit);

            assert!(circuit.ac_ess_bus.output().is_none());
        }
    
        fn electrical_circuit() -> A320ElectricalCircuit {
            A320ElectricalCircuit::new()
        }

        fn update_with_running_engines(circuit: &mut A320ElectricalCircuit) {
            circuit.update(&running_engine(), &running_engine(), &stopped_apu(), &disconnected_external_power());
        }
    }
    
    #[cfg(test)]
    mod engine_generator_tests {
        use uom::si::{ratio::percent};
        use super::*;
    
        #[test]
        fn starts_without_output() {
            assert!(engine_generator().output.is_none());
        }
    
        #[test]
        fn when_engine_n2_above_threshold_provides_output() {
            let mut generator = engine_generator();
            update_below_threshold(&mut generator);
            update_above_threshold(&mut generator);
    
            assert!(generator.output.is_alternating());
        }
    
        #[test]
        fn when_engine_n2_below_threshold_provides_no_output() {
            let mut generator = engine_generator();
            update_above_threshold(&mut generator);
            update_below_threshold(&mut generator);
    
            assert!(generator.output.is_none());
        }
    
        fn engine_generator() -> EngineGenerator {
            EngineGenerator::new(1)
        }
    
        fn engine(n2: Ratio) -> Engine {
            let mut engine = Engine::new();
            engine.n2 = n2;
    
            engine
        }
    
        fn update_above_threshold(generator: &mut EngineGenerator) {
            generator.update(&engine(Ratio::new::<percent>(EngineGenerator::ENGINE_N2_POWER_OUTPUT_THRESHOLD + 1.)));
        }
    
        fn update_below_threshold(generator: &mut EngineGenerator) {
            generator.update(&engine(Ratio::new::<percent>(EngineGenerator::ENGINE_N2_POWER_OUTPUT_THRESHOLD - 1.)));
        }
    }
    
    #[cfg(test)]
    mod apu_generator_tests {
        use uom::si::{ratio::percent};
        use super::*;
    
        #[test]
        fn starts_without_output() {
            assert!(apu_generator().output.is_none());
        }
    
        #[test]
        fn when_apu_speed_above_threshold_provides_output() {
            let mut generator = apu_generator();
            update_below_threshold(&mut generator);
            update_above_threshold(&mut generator);
    
            assert!(generator.output.is_alternating());
        }
    
        #[test]
        fn when_apu_speed_below_threshold_provides_no_output() {
            let mut generator = apu_generator();
            update_above_threshold(&mut generator);
            update_below_threshold(&mut generator);
    
            assert!(generator.output.is_none());
        }
    
        fn apu_generator() -> ApuGenerator {
            ApuGenerator::new()
        }
    
        fn apu(speed: Ratio) -> AuxiliaryPowerUnit {
            let mut apu = AuxiliaryPowerUnit::new();
            apu.speed = speed;
    
            apu
        }
    
        fn update_above_threshold(generator: &mut ApuGenerator) {
            generator.update(&apu(Ratio::new::<percent>(ApuGenerator::APU_SPEED_POWER_OUTPUT_THRESHOLD + 1.)));
        }
    
        fn update_below_threshold(generator: &mut ApuGenerator) {
            generator.update(&apu(Ratio::new::<percent>(ApuGenerator::APU_SPEED_POWER_OUTPUT_THRESHOLD - 1.)));
        }
    }
    
    #[cfg(test)]
    mod external_power_source_tests {
        use super::*;
    
        #[test]
        fn starts_without_output() {
            assert!(external_power_source().output().is_none());
        }
    
        #[test]
        fn when_plugged_in_provides_output() {
            let mut ext_pwr = external_power_source();
            ext_pwr.plugged_in = true;
    
            assert!(ext_pwr.output().is_alternating());
        }
    
        #[test]
        fn when_not_plugged_in_provides_no_output() {
            let mut ext_pwr = external_power_source();
            ext_pwr.plugged_in = false;
    
            assert!(ext_pwr.output().is_none());
        }
    
        fn external_power_source() -> ExternalPowerSource {
            ExternalPowerSource::new()
        }
    }
    
}
