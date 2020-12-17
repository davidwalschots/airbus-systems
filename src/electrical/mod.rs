use uom::si::{electric_current::ampere, electric_potential::volt, f32::{Frequency, ElectricPotential, ElectricCurrent, Ratio}, frequency::hertz, ratio::percent};

use crate::{overhead::OnOffPushButton, shared::Engine};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PowerSource {
    None,
    EngineGenerator(u8),
    ApuGenerator,
    External
}

/// Represents a type of electric current.
#[derive(Clone, Copy, Debug)]
pub enum Current {
    Alternating(PowerSource, Frequency, ElectricPotential, ElectricCurrent),
    Direct(PowerSource, ElectricPotential, ElectricCurrent),
    None
}

impl Current {
    pub fn is_powered(self) -> bool {
        match self { 
            Current::Alternating(..) => true,
            Current::Direct(..) => true,
            _ => false
        }
    }

    pub fn is_unpowered(self) -> bool {
        if let Current::None = self { true } else { false }
    }

    pub fn get_source(self) -> PowerSource {
        match self {
            Current::Alternating(source, ..) => source,
            Current::Direct(source, ..) => source,
            _ => PowerSource::None
        }
    }
}

pub trait PowerConductor {
    fn output(&self) -> Current;
}

pub trait Powerable {
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
                if !output.is_unpowered() {
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
pub struct Contactor {
    state: ContactorState,
    input: Current,
}

impl Contactor {
    pub fn new() -> Contactor {
        Contactor {
            state: ContactorState::Open,
            input: Current::None,
        }
    }

    pub fn toggle(&mut self, should_be_closed: bool) {
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

pub struct EngineGenerator {
    number: u8,
    output: Current,
}

impl EngineGenerator {
    pub const ENGINE_N2_POWER_OUTPUT_THRESHOLD: f32 = 57.5;

    pub fn new(number: u8) -> EngineGenerator {
        EngineGenerator {
            number,
            output: Current::None,
        }
    }

    pub fn update(&mut self, engine: &Engine, idg_push_button: &OnOffPushButton, gen_push_button: &OnOffPushButton) {
        // TODO: The push buttons being on or off is still a simplification. Of course we should later simulate the
        // IDG itself. It would be disconnected the moment the push button is in the off state. Then the logic below would
        // consider the IDG state itself, instead of the button state.
        if EngineGenerator::engine_above_threshold(engine) && idg_push_button.is_on() && gen_push_button.is_on() {
            self.output = Current::Alternating(PowerSource::EngineGenerator(self.number), Frequency::new::<hertz>(400.), 
                ElectricPotential::new::<volt>(115.), ElectricCurrent::new::<ampere>(782.60));
        } else {
            self.output = Current::None
        }
    }

    fn engine_above_threshold(engine: &Engine) -> bool {
        engine.n2 > Ratio::new::<percent>(EngineGenerator::ENGINE_N2_POWER_OUTPUT_THRESHOLD)
    }
}

impl PowerConductor for EngineGenerator {
    fn output(&self) -> Current {
        self.output
    }
}

pub struct ApuGenerator {
    output: Current
}

impl ApuGenerator {
    pub const APU_SPEED_POWER_OUTPUT_THRESHOLD: f32 = 57.5;

    pub fn new() -> ApuGenerator {
        ApuGenerator {
            output: Current::None
        }
    }

    pub fn update(&mut self, apu: &AuxiliaryPowerUnit) {
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

pub struct AuxiliaryPowerUnit {
    pub speed: Ratio
}

impl AuxiliaryPowerUnit {
    pub fn new() -> AuxiliaryPowerUnit {
        AuxiliaryPowerUnit {
            speed: Ratio::new::<percent>(0.)
        }
    }
}

pub struct ExternalPowerSource {
    pub plugged_in: bool
}

impl ExternalPowerSource {
    pub fn new() -> ExternalPowerSource {
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

pub struct ElectricalBus {
    input: Current,
    failed: bool
}

impl ElectricalBus {
    pub fn new() -> ElectricalBus {
        ElectricalBus {
            input: Current::None,
            failed: false
        }
    }

    pub fn fail(&mut self) {
        self.failed = true;
    }

    pub fn normal(&mut self) {
        self.failed = false;
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

#[cfg(test)]
mod current_tests {
    use uom::si::{electric_current::ampere, electric_potential::volt, frequency::hertz};

    use super::*;

    #[test]
    fn alternating_current_is_powered() {
        assert_eq!(alternating_current().is_powered(), true);
    }

    #[test]
    fn alternating_current_is_not_unpowered() {
        assert_eq!(alternating_current().is_unpowered(), false);
    }

    #[test]
    fn direct_current_is_powered() {
        assert_eq!(direct_current().is_powered(), true);
    }

    #[test]
    fn direct_current_is_not_unpowered() {
        assert_eq!(direct_current().is_unpowered(), false);
    }
    
    #[test]
    fn none_current_is_not_powered() {
        assert_eq!(none_current().is_powered(), false);
    }

    #[test]
    fn none_current_is_unpowered() {
        assert_eq!(none_current().is_unpowered(), true);
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

        assert!(contactor.output().is_unpowered());
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

        assert!(contactor.output().is_unpowered());
    }

    #[test]
    fn open_contactor_has_no_output_when_powered_by_something() {
        let mut contactor = open_contactor();
        let conductors: Vec<&dyn PowerConductor> = vec![&Powerless{}, &Powered{}];
        contactor.powered_by(conductors);

        assert!(contactor.output().is_unpowered());
    }

    #[test]
    fn closed_contactor_has_output_when_powered_by_something_which_is_powered() {
        let mut contactor = closed_contactor();
        let conductors: Vec<&dyn PowerConductor> = vec![&Powerless{}, &Powered{}];
        contactor.powered_by(conductors);

        assert!(contactor.output().is_powered());
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
mod engine_generator_tests {
    use uom::si::{ratio::percent};
    use super::*;

    #[test]
    fn starts_without_output() {
        assert!(engine_generator().output.is_unpowered());
    }

    #[test]
    fn when_engine_n2_above_threshold_provides_output() {
        let mut generator = engine_generator();
        update_below_threshold(&mut generator);
        update_above_threshold(&mut generator);

        assert!(generator.output.is_powered());
    }

    #[test]
    fn when_engine_n2_below_threshold_provides_no_output() {
        let mut generator = engine_generator();
        update_above_threshold(&mut generator);
        update_below_threshold(&mut generator);

        assert!(generator.output.is_unpowered());
    }

    #[test]
    fn when_idg_disconnected_provides_no_output() {
        let mut generator = engine_generator();
        generator.update(&engine_above_threshold(), &OnOffPushButton::new_off(), &OnOffPushButton::new_on());

        assert!(generator.output.is_unpowered());
    }

    #[test]
    fn when_gen_off_provides_no_output() {
        let mut generator = engine_generator();
        generator.update(&engine_above_threshold(), &OnOffPushButton::new_on(), &OnOffPushButton::new_off());

        assert!(generator.output.is_unpowered());
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
        generator.update(&engine_above_threshold(), &OnOffPushButton::new_on(), &OnOffPushButton::new_on());
    }

    fn update_below_threshold(generator: &mut EngineGenerator) {
        generator.update(&engine_below_threshold(), &OnOffPushButton::new_on(), &OnOffPushButton::new_on());
    }

    fn engine_above_threshold() -> Engine {
        engine(Ratio::new::<percent>(EngineGenerator::ENGINE_N2_POWER_OUTPUT_THRESHOLD + 1.))
    }

    fn engine_below_threshold() -> Engine {
        engine(Ratio::new::<percent>(EngineGenerator::ENGINE_N2_POWER_OUTPUT_THRESHOLD - 1.))
    }
}

#[cfg(test)]
mod apu_generator_tests {
    use uom::si::{ratio::percent};
    use super::*;

    #[test]
    fn starts_without_output() {
        assert!(apu_generator().output.is_unpowered());
    }

    #[test]
    fn when_apu_speed_above_threshold_provides_output() {
        let mut generator = apu_generator();
        update_below_threshold(&mut generator);
        update_above_threshold(&mut generator);

        assert!(generator.output.is_powered());
    }

    #[test]
    fn when_apu_speed_below_threshold_provides_no_output() {
        let mut generator = apu_generator();
        update_above_threshold(&mut generator);
        update_below_threshold(&mut generator);

        assert!(generator.output.is_unpowered());
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
        assert!(external_power_source().output().is_unpowered());
    }

    #[test]
    fn when_plugged_in_provides_output() {
        let mut ext_pwr = external_power_source();
        ext_pwr.plugged_in = true;

        assert!(ext_pwr.output().is_powered());
    }

    #[test]
    fn when_not_plugged_in_provides_no_output() {
        let mut ext_pwr = external_power_source();
        ext_pwr.plugged_in = false;

        assert!(ext_pwr.output().is_unpowered());
    }

    fn external_power_source() -> ExternalPowerSource {
        ExternalPowerSource::new()
    }
}
