use uom::si::{electric_current::ampere, electric_potential::volt, f32::{Frequency, ElectricPotential, ElectricCurrent, Ratio}, frequency::hertz, ratio::percent};

/// Represents a type of electric current.
#[derive(Debug)]
enum Current {
    Alternating(Frequency, ElectricPotential, ElectricCurrent),
    Direct(ElectricPotential, ElectricCurrent),
    None
}

impl Current {
    pub fn is_alternating(self) -> bool {
        if let Current::Alternating(..) = self { true } else { false }
    }

    pub fn is_direct(self) -> bool {
        if let Current::Direct(..) = self { true } else { false }
    }

    pub fn is_none(self) -> bool {
        if let Current::None = self { true } else { false }
    }
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
}

impl Contactor {
    fn new() -> Contactor {
        Contactor {
            state: ContactorState::Open
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

struct A320ElectricalCircuit {
    engine_gen_1: EngineGenerator,
    engine_gen_2: EngineGenerator,
    apu_gen: ApuGenerator
}

impl A320ElectricalCircuit {
    fn new() -> A320ElectricalCircuit {
        A320ElectricalCircuit {
            engine_gen_1: EngineGenerator::new(),
            engine_gen_2: EngineGenerator::new(),
            apu_gen: ApuGenerator::new()
        }
    }

    fn update(&mut self, engine1: &Engine, engine2: &Engine, apu: &AuxiliaryPowerUnit) {
        self.engine_gen_1.update(engine1);
        self.engine_gen_2.update(engine2);
        self.apu_gen.update(apu);
    }
}

struct EngineGenerator {
    output: Current
}

impl EngineGenerator {
    fn new() -> EngineGenerator {
        EngineGenerator {
            output: Current::None
        }
    }

    fn update(&mut self, engine: &Engine) {
        const POWER_OUTPUT_THRESHOLD: f32 = 57.5;
        if engine.n2 > Ratio::new::<percent>(POWER_OUTPUT_THRESHOLD) {
            self.output = Current::Alternating(Frequency::new::<hertz>(400.), ElectricPotential::new::<volt>(115.), ElectricCurrent::new::<ampere>(782.60));
        } else {
            self.output = Current::None
        }
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
    fn new() -> ApuGenerator {
        ApuGenerator {
            output: Current::None
        }
    }

    fn update(&mut self, apu: &AuxiliaryPowerUnit) {
        const POWER_OUTPUT_THRESHOLD: f32 = 57.5;
        if apu.speed > Ratio::new::<percent>(POWER_OUTPUT_THRESHOLD) {
            self.output = Current::Alternating(Frequency::new::<hertz>(400.), ElectricPotential::new::<volt>(115.), ElectricCurrent::new::<ampere>(782.60));
        } else {
            self.output = Current::None
        }
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
        Current::Alternating(Frequency::new::<hertz>(0.), ElectricPotential::new::<volt>(0.), ElectricCurrent::new::<ampere>(0.))
    }

    fn direct_current() -> Current {
        Current::Direct(ElectricPotential::new::<volt>(0.), ElectricCurrent::new::<ampere>(0.))
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
        EngineGenerator::new()
    }

    fn engine(n2: Ratio) -> Engine {
        let mut engine = Engine::new();
        engine.n2 = n2;

        engine
    }

    fn update_above_threshold(generator: &mut EngineGenerator) {
        const ABOVE_THRESHOLD: f32 = 57.6;
        generator.update(&engine(Ratio::new::<percent>(ABOVE_THRESHOLD)));
    }

    fn update_below_threshold(generator: &mut EngineGenerator) {
        const BELOW_THRESHOLD: f32 = 57.5;
        generator.update(&engine(Ratio::new::<percent>(BELOW_THRESHOLD)));
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
        const ABOVE_THRESHOLD: f32 = 57.6;
        generator.update(&apu(Ratio::new::<percent>(ABOVE_THRESHOLD)));
    }

    fn update_below_threshold(generator: &mut ApuGenerator) {
        const BELOW_THRESHOLD: f32 = 57.5;
        generator.update(&apu(Ratio::new::<percent>(BELOW_THRESHOLD)));
    }
}
