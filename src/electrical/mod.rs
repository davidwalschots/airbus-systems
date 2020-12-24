use uom::si::{
    electric_charge::ampere_hour,
    electric_current::ampere,
    electric_potential::volt,
    f32::{ElectricCharge, ElectricCurrent, ElectricPotential, Frequency, Ratio},
    frequency::hertz,
    ratio::percent,
};

use crate::{
    overhead::OnOffPushButton,
    shared::{Engine, UpdateContext},
    visitor::Visitable,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PowerSource {
    None,
    EngineGenerator(u8),
    ApuGenerator,
    External,
    EmergencyGenerator,
    Battery(u8),
}

/// Represents a type of electric current.
#[derive(Clone, Copy, Debug)]
pub enum Current {
    Alternating(PowerSource, Frequency, ElectricPotential, ElectricCurrent),
    Direct(PowerSource, ElectricPotential, ElectricCurrent),
    None,
}

impl Current {
    pub fn is_powered(&self) -> bool {
        match self {
            Current::Alternating(..) => true,
            Current::Direct(..) => true,
            _ => false,
        }
    }

    pub fn is_unpowered(&self) -> bool {
        if let Current::None = self {
            true
        } else {
            false
        }
    }

    pub fn source(self) -> PowerSource {
        match self {
            Current::Alternating(source, ..) => source,
            Current::Direct(source, ..) => source,
            _ => PowerSource::None,
        }
    }
}

pub trait PowerConductor {
    fn output(&self) -> Current;

    fn is_powered(&self) -> bool {
        self.output().is_powered()
    }

    fn is_unpowered(&self) -> bool {
        self.output().is_unpowered()
    }
}

pub trait Powerable {
    /// Provides input power from any of the given sources. When none of the sources
    /// has any output, no input is provided.
    fn powered_by<T: PowerConductor + ?Sized>(&mut self, sources: Vec<&T>) {
        self.set_input(
            sources
                .iter()
                .find_map(|x| {
                    let output = x.output();
                    match output {
                        Current::None => None,
                        _ => Some(output),
                    }
                })
                .unwrap_or(Current::None),
        );
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
    Closed,
}

/// Represents a contactor in a electrical power circuit.
#[derive(Debug)]
pub struct Contactor {
    id: String,
    state: ContactorState,
    input: Current,
}

impl Contactor {
    pub fn new(id: String) -> Contactor {
        Contactor {
            id,
            state: ContactorState::Open,
            input: Current::None,
        }
    }

    pub fn close_when(&mut self, should_be_closed: bool) {
        self.state = match self.state {
            ContactorState::Open if should_be_closed => ContactorState::Closed,
            ContactorState::Closed if !should_be_closed => ContactorState::Open,
            _ => self.state,
        };
    }

    pub fn is_open(&self) -> bool {
        if let ContactorState::Open = self.state {
            true
        } else {
            false
        }
    }

    pub fn is_closed(&self) -> bool {
        !self.is_open()
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

    pub fn update(&mut self, engine: &Engine, idg_push_button: &OnOffPushButton) {
        // TODO: The push button being on or off is still a simplification. Of course we should later simulate the
        // IDG itself. It would be disconnected the moment the push button is in the off state. Then the logic below would
        // consider the IDG state itself, instead of the button state.
        if EngineGenerator::engine_above_threshold(engine) && idg_push_button.is_on() {
            self.output = Current::Alternating(
                PowerSource::EngineGenerator(self.number),
                Frequency::new::<hertz>(400.),
                ElectricPotential::new::<volt>(115.),
                ElectricCurrent::new::<ampere>(782.60),
            );
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
    output: Current,
}

impl ApuGenerator {
    pub const APU_SPEED_POWER_OUTPUT_THRESHOLD: f32 = 57.5;

    pub fn new() -> ApuGenerator {
        ApuGenerator {
            output: Current::None,
        }
    }

    pub fn update(&mut self, apu: &AuxiliaryPowerUnit) {
        const POWER_OUTPUT_THRESHOLD: f32 = 57.5;
        if apu.speed > Ratio::new::<percent>(ApuGenerator::APU_SPEED_POWER_OUTPUT_THRESHOLD) {
            self.output = Current::Alternating(
                PowerSource::ApuGenerator,
                Frequency::new::<hertz>(400.),
                ElectricPotential::new::<volt>(115.),
                ElectricCurrent::new::<ampere>(782.60),
            );
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
    pub speed: Ratio,
}

impl AuxiliaryPowerUnit {
    pub fn new() -> AuxiliaryPowerUnit {
        AuxiliaryPowerUnit {
            speed: Ratio::new::<percent>(0.),
        }
    }

    pub fn update(&mut self, context: &UpdateContext) {}
}

impl Visitable for AuxiliaryPowerUnit {
    fn accept(&mut self, visitor: &mut Box<dyn crate::visitor::MutableVisitor>) {
        visitor.visit_auxiliary_power_unit(self);
    }
}

pub struct ExternalPowerSource {
    pub is_connected: bool,
}

impl ExternalPowerSource {
    pub fn new() -> ExternalPowerSource {
        ExternalPowerSource {
            is_connected: false,
        }
    }

    pub fn update(&mut self, context: &UpdateContext) {}
}

impl Visitable for ExternalPowerSource {
    fn accept(&mut self, visitor: &mut Box<dyn crate::visitor::MutableVisitor>) {
        visitor.visit_external_power_source(self);
    }
}

impl PowerConductor for ExternalPowerSource {
    fn output(&self) -> Current {
        if self.is_connected {
            Current::Alternating(
                PowerSource::External,
                Frequency::new::<hertz>(400.),
                ElectricPotential::new::<volt>(115.),
                ElectricCurrent::new::<ampere>(782.60),
            )
        } else {
            Current::None
        }
    }
}

pub struct ElectricalBus {
    input: Current,
    failed: bool,
}

impl ElectricalBus {
    pub fn new() -> ElectricalBus {
        ElectricalBus {
            input: Current::None,
            failed: false,
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

pub struct TransformerRectifier {
    input: Current,
    failed: bool,
}

impl TransformerRectifier {
    pub fn new() -> TransformerRectifier {
        TransformerRectifier {
            input: Current::None,
            failed: false,
        }
    }

    pub fn fail(&mut self) {
        self.failed = true;
    }

    pub fn normal(&mut self) {
        self.failed = false;
    }

    pub fn has_failed(&self) -> bool {
        self.failed
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

impl PowerConductor for TransformerRectifier {
    fn output(&self) -> Current {
        if self.failed {
            Current::None
        } else {
            match self.input {
                Current::Alternating(source, ..) => Current::Direct(
                    source,
                    ElectricPotential::new::<volt>(28.5),
                    ElectricCurrent::new::<ampere>(35.),
                ),
                _ => Current::None,
            }
        }
    }
}

pub struct EmergencyGenerator {
    running: bool,
    is_blue_pressurised: bool,
}

impl EmergencyGenerator {
    pub fn new() -> EmergencyGenerator {
        EmergencyGenerator {
            running: false,
            is_blue_pressurised: false,
        }
    }

    pub fn update(&mut self, is_blue_pressurised: bool) {
        // TODO: The emergency generator is driven by the blue hydraulic circuit. Still to be implemented.
        self.is_blue_pressurised = is_blue_pressurised;
    }

    pub fn attempt_start(&mut self) {
        self.running = true;
    }

    pub fn is_running(&self) -> bool {
        self.is_blue_pressurised && self.running
    }
}

impl PowerConductor for EmergencyGenerator {
    fn output(&self) -> Current {
        if self.is_running() {
            Current::Alternating(
                PowerSource::EmergencyGenerator,
                Frequency::new::<hertz>(400.),
                ElectricPotential::new::<volt>(115.),
                ElectricCurrent::new::<ampere>(43.47),
            ) // 5 kVA.
        } else {
            Current::None
        }
    }
}

pub struct Battery {
    number: u8,
    input: Current,
    charge: ElectricCharge,
}

impl Battery {
    const MAX_ELECTRIC_CHARGE_AMPERE_HOURS: f32 = 23.0;

    pub fn full(number: u8) -> Battery {
        Battery::new(
            number,
            ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS),
        )
    }

    pub fn empty(number: u8) -> Battery {
        Battery::new(number, ElectricCharge::new::<ampere_hour>(0.))
    }

    fn new(number: u8, charge: ElectricCharge) -> Battery {
        Battery {
            number,
            input: Current::None,
            charge,
        }
    }

    pub fn is_full(&self) -> bool {
        self.charge >= ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS)
    }

    pub fn is_empty(&self) -> bool {
        self.charge == ElectricCharge::new::<ampere_hour>(0.)
    }

    // TODO: Charging and depleting battery when used.
}

impl Powerable for Battery {
    fn set_input(&mut self, current: Current) {
        self.input = current
    }

    fn get_input(&self) -> Current {
        self.input
    }
}

impl PowerConductor for Battery {
    fn output(&self) -> Current {
        if let Current::None = self.input {
            if self.charge > ElectricCharge::new::<ampere_hour>(0.) {
                return Current::Direct(
                    PowerSource::Battery(self.number),
                    ElectricPotential::new::<volt>(28.5),
                    ElectricCurrent::new::<ampere>(35.),
                );
            }
        }

        Current::None
    }
}

pub struct StaticInverter {
    input: Current,
}

impl StaticInverter {
    pub fn new() -> StaticInverter {
        StaticInverter {
            input: Current::None,
        }
    }
}

impl Powerable for StaticInverter {
    fn set_input(&mut self, current: Current) {
        self.input = current;
    }

    fn get_input(&self) -> Current {
        self.input
    }
}

impl PowerConductor for StaticInverter {
    fn output(&self) -> Current {
        match self.input {
            Current::Direct(source, ..) => Current::Alternating(
                source,
                Frequency::new::<hertz>(400.),
                ElectricPotential::new::<volt>(115.0),
                ElectricCurrent::new::<ampere>(8.7),
            ),
            _ => Current::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Powerless {}

    impl PowerConductor for Powerless {
        fn output(&self) -> Current {
            Current::None
        }
    }

    struct StubApuGenerator {}

    impl PowerConductor for StubApuGenerator {
        fn output(&self) -> Current {
            Current::Alternating(
                PowerSource::ApuGenerator,
                Frequency::new::<hertz>(400.),
                ElectricPotential::new::<volt>(115.),
                ElectricCurrent::new::<ampere>(782.60),
            )
        }
    }

    fn apu_generator() -> StubApuGenerator {
        StubApuGenerator {}
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
            Current::Alternating(
                PowerSource::ApuGenerator,
                Frequency::new::<hertz>(0.),
                ElectricPotential::new::<volt>(0.),
                ElectricCurrent::new::<ampere>(0.),
            )
        }

        fn direct_current() -> Current {
            Current::Direct(
                PowerSource::ApuGenerator,
                ElectricPotential::new::<volt>(0.),
                ElectricCurrent::new::<ampere>(0.),
            )
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
            contactor.close_when(false);

            assert_eq!(contactor.state, ContactorState::Open);
        }

        #[test]
        fn open_contactor_when_toggled_closed_closes() {
            let mut contactor = open_contactor();
            contactor.close_when(true);

            assert_eq!(contactor.state, ContactorState::Closed);
        }

        #[test]
        fn closed_contactor_when_toggled_open_opens() {
            let mut contactor = closed_contactor();
            contactor.close_when(false);

            assert_eq!(contactor.state, ContactorState::Open);
        }

        #[test]
        fn closed_contactor_when_toggled_closed_stays_closed() {
            let mut contactor = closed_contactor();
            contactor.close_when(true);

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

        fn contactor_has_no_output_when_powered_by_nothing_which_is_powered(
            mut contactor: Contactor,
        ) {
            contactor.powered_by(vec![&Powerless {}]);

            assert!(contactor.output().is_unpowered());
        }

        #[test]
        fn open_contactor_has_no_output_when_powered_by_something() {
            let mut contactor = open_contactor();
            let conductors: Vec<&dyn PowerConductor> = vec![&Powerless {}, &StubApuGenerator {}];
            contactor.powered_by(conductors);

            assert!(contactor.output().is_unpowered());
        }

        #[test]
        fn closed_contactor_has_output_when_powered_by_something_which_is_powered() {
            let mut contactor = closed_contactor();
            let conductors: Vec<&dyn PowerConductor> = vec![&Powerless {}, &StubApuGenerator {}];
            contactor.powered_by(conductors);

            assert!(contactor.output().is_powered());
        }

        fn contactor() -> Contactor {
            Contactor::new(String::from("TEST"))
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
        use super::*;
        use uom::si::ratio::percent;

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
            generator.update(&engine_above_threshold(), &OnOffPushButton::new_off());

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
            generator.update(&engine_above_threshold(), &OnOffPushButton::new_on());
        }

        fn update_below_threshold(generator: &mut EngineGenerator) {
            generator.update(&engine_below_threshold(), &OnOffPushButton::new_on());
        }

        fn engine_above_threshold() -> Engine {
            engine(Ratio::new::<percent>(
                EngineGenerator::ENGINE_N2_POWER_OUTPUT_THRESHOLD + 1.,
            ))
        }

        fn engine_below_threshold() -> Engine {
            engine(Ratio::new::<percent>(
                EngineGenerator::ENGINE_N2_POWER_OUTPUT_THRESHOLD - 1.,
            ))
        }
    }

    #[cfg(test)]
    mod apu_generator_tests {
        use super::*;
        use uom::si::ratio::percent;

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
            generator.update(&apu(Ratio::new::<percent>(
                ApuGenerator::APU_SPEED_POWER_OUTPUT_THRESHOLD + 1.,
            )));
        }

        fn update_below_threshold(generator: &mut ApuGenerator) {
            generator.update(&apu(Ratio::new::<percent>(
                ApuGenerator::APU_SPEED_POWER_OUTPUT_THRESHOLD - 1.,
            )));
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
            ext_pwr.is_connected = true;

            assert!(ext_pwr.output().is_powered());
        }

        #[test]
        fn when_not_plugged_in_provides_no_output() {
            let mut ext_pwr = external_power_source();
            ext_pwr.is_connected = false;

            assert!(ext_pwr.output().is_unpowered());
        }

        fn external_power_source() -> ExternalPowerSource {
            ExternalPowerSource::new()
        }
    }

    #[cfg(test)]
    mod transformer_rectifier_tests {
        use super::*;

        #[test]
        fn starts_without_output() {
            assert!(transformer_rectifier().output().is_unpowered());
        }

        #[test]
        fn when_powered_with_alternating_current_outputs_direct_current() {
            let mut tr = transformer_rectifier();
            tr.powered_by(vec![&apu_generator()]);

            assert!(tr.output().is_powered());
            assert!(
                if let Current::Direct(PowerSource::ApuGenerator, ..) = tr.output() {
                    true
                } else {
                    false
                }
            );
        }

        #[test]
        fn when_powered_with_alternating_current_but_failed_has_no_output() {
            let mut tr = transformer_rectifier();
            tr.powered_by(vec![&apu_generator()]);
            tr.fail();

            assert!(tr.output().is_unpowered());
        }

        #[test]
        fn when_unpowered_has_no_output() {
            let mut tr = transformer_rectifier();
            tr.powered_by(vec![&Powerless {}]);

            assert!(tr.output().is_unpowered());
        }

        fn transformer_rectifier() -> TransformerRectifier {
            TransformerRectifier::new()
        }
    }

    #[cfg(test)]
    mod emergency_generator_tests {
        use super::*;

        #[test]
        fn starts_without_output() {
            assert!(emergency_generator().output().is_unpowered());
        }

        #[test]
        fn when_started_provides_output() {
            let mut emer_gen = emergency_generator();
            emer_gen.attempt_start();
            emer_gen.update(true);

            assert!(emer_gen.output().is_powered());
        }

        #[test]
        fn when_started_without_hydraulic_pressure_is_unpowered() {
            let mut emer_gen = emergency_generator();
            emer_gen.attempt_start();
            emer_gen.update(false);

            assert!(emer_gen.output().is_unpowered());
        }

        fn emergency_generator() -> EmergencyGenerator {
            EmergencyGenerator::new()
        }
    }

    #[cfg(test)]
    mod battery_tests {
        use super::*;

        #[test]
        fn full_battery_is_full_with_output() {
            assert!(full_battery().is_full());
            assert!(!full_battery().is_empty());
            assert!(full_battery().output().is_powered());
        }

        #[test]
        fn empty_battery_is_empty_without_output() {
            assert!(!empty_battery().is_full());
            assert!(empty_battery().is_empty());
            assert!(empty_battery().output().is_unpowered());
        }

        #[test]
        fn when_empty_battery_has_input_doesnt_have_output() {
            let mut battery = empty_battery();
            battery.powered_by(vec![&apu_generator()]);

            assert!(battery.output().is_unpowered());
        }

        #[test]
        fn when_full_battery_has_doesnt_have_output() {
            // Of course battery input at this stage would result in overcharging. However, for the sake of the test we ignore it.
            let mut battery = full_battery();
            battery.powered_by(vec![&apu_generator()]);

            assert!(battery.output().is_unpowered());
        }

        #[test]
        fn charged_battery_without_input_has_output() {
            let mut battery = full_battery();
            battery.powered_by(vec![&Powerless {}]);

            assert!(battery.output().is_powered());
        }

        #[test]
        fn empty_battery_without_input_has_no_output() {
            let mut battery = empty_battery();
            battery.powered_by(vec![&Powerless {}]);

            assert!(battery.output().is_unpowered());
        }

        fn full_battery() -> Battery {
            Battery::full(1)
        }

        fn empty_battery() -> Battery {
            Battery::empty(1)
        }
    }

    #[cfg(test)]
    mod static_inverter_tests {
        use super::*;

        #[test]
        fn starts_without_output() {
            assert!(static_inverter().output().is_unpowered());
        }

        #[test]
        fn when_powered_with_direct_current_outputs_alternating_current() {
            let mut static_inv = static_inverter();
            static_inv.powered_by(vec![&battery()]);

            assert!(static_inv.output().is_powered());
            assert!(
                if let Current::Alternating(PowerSource::Battery(1), ..) = static_inv.output() {
                    true
                } else {
                    false
                }
            );
        }

        #[test]
        fn when_unpowered_has_no_output() {
            let mut static_inv = static_inverter();
            static_inv.powered_by(vec![&Powerless {}]);

            assert!(static_inv.output().is_unpowered());
        }

        fn static_inverter() -> StaticInverter {
            StaticInverter::new()
        }

        fn battery() -> Battery {
            Battery::full(1)
        }
    }
}
