mod battery;
mod emergency_generator;
mod engine_generator;
mod external_power_source;
mod power_consumption;
mod static_inverter;
mod transformer_rectifier;
pub use battery::Battery;
pub use emergency_generator::EmergencyGenerator;
pub use engine_generator::EngineGenerator;
pub use external_power_source::ExternalPowerSource;
pub use power_consumption::{
    ElectricalBusStateFactory, PowerConsumption, PowerConsumptionHandler, PowerConsumptionState,
    PowerSupply,
};
pub use static_inverter::StaticInverter;
pub use transformer_rectifier::TransformerRectifier;

use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorWriter,
};
use uom::si::{
    electric_current::ampere, electric_potential::volt, f64::*, frequency::hertz, ratio::percent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElectricPowerSource {
    EngineGenerator(usize),
    ApuGenerator,
    External,
    EmergencyGenerator,
    Battery(usize),
    Batteries,
    TransformerRectifier(usize),
    StaticInverter,
}

/// Represents a type of electric current.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Current {
    source: Option<ElectricPowerSource>,
}
impl Current {
    pub fn some(source: ElectricPowerSource) -> Self {
        Current {
            source: Some(source),
        }
    }

    pub fn none() -> Self {
        Current { source: None }
    }

    pub fn is_powered(&self) -> bool {
        self.source.is_some()
    }

    pub fn is_unpowered(&self) -> bool {
        self.source.is_none()
    }

    fn get_source(&self) -> Option<ElectricPowerSource> {
        self.source
    }
}

/// A source of electric energy. A source is not necessarily something
/// that generates the electric energy. It can also be something that conducts
/// it from another source.
pub trait ElectricSource {
    fn output(&self) -> Current;

    fn is_powered(&self) -> bool {
        self.output().is_powered()
    }

    fn is_unpowered(&self) -> bool {
        self.output().is_unpowered()
    }
}

pub trait Powerable {
    /// Provides input power from the given source. When the source has
    /// output, this element is powered by the source. When the source has no
    /// output, this element is unpowered.
    fn powered_by<T: ElectricSource + ?Sized>(&mut self, source: &T) {
        self.set_input(source.output());
    }

    /// Provides input power from the given source. When the element is already powered,
    /// it will remain powered by the powered source passed to it at an earlier time.
    /// When the element is not yet powered and the source has output, this element is powered by the source.
    /// When the element is not yet powered and the source has no output, this element is unpowered.
    fn or_powered_by<T: ElectricSource + ?Sized>(&mut self, source: &T) {
        if self.get_input().is_unpowered() {
            self.powered_by(source);
        }
    }

    fn or_powered_by_both_batteries(
        &mut self,
        battery_1_contactor: &Contactor,
        battery_2_contactor: &Contactor,
    ) {
        if self.get_input().is_unpowered() {
            let is_battery_1_powered = battery_1_contactor.is_powered();
            let is_battery_2_powered = battery_2_contactor.is_powered();

            if is_battery_1_powered && is_battery_2_powered {
                self.set_input(Current::some(ElectricPowerSource::Batteries));
            } else if is_battery_1_powered {
                self.set_input(Current::some(ElectricPowerSource::Battery(10)));
            } else if is_battery_2_powered {
                self.set_input(Current::some(ElectricPowerSource::Battery(11)));
            } else {
                self.set_input(Current::none());
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
    closed_id: String,
    state: ContactorState,
    input: Current,
}
impl Contactor {
    pub fn new(id: &str) -> Contactor {
        Contactor {
            closed_id: format!("ELEC_CONTACTOR_{}_IS_CLOSED", id),
            state: ContactorState::Open,
            input: Current::none(),
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
impl ElectricSource for Contactor {
    fn output(&self) -> Current {
        if let ContactorState::Closed = self.state {
            self.input
        } else {
            Current::none()
        }
    }
}
impl SimulatorElementVisitable for Contactor {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for Contactor {
    fn write(&self, state: &mut SimulatorWriter) {
        state.write_bool(&self.closed_id, self.is_closed());
    }
}

pub fn combine_electric_sources<T: ElectricSource>(sources: Vec<&T>) -> CombinedElectricSource {
    CombinedElectricSource::new(sources)
}

pub struct CombinedElectricSource {
    current: Current,
}
impl CombinedElectricSource {
    fn new<T: ElectricSource>(sources: Vec<&T>) -> Self {
        let x = sources.iter().map(|x| x.output()).find(|x| x.is_powered());
        CombinedElectricSource {
            current: match x {
                Some(current) => current,
                None => Current::none(),
            },
        }
    }
}
impl ElectricSource for CombinedElectricSource {
    fn output(&self) -> Current {
        self.current
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ElectricalBusType {
    AlternatingCurrent(u8),
    AlternatingCurrentEssential,
    AlternatingCurrentEssentialShed,
    AlternatingCurrentStaticInverter,
    DirectCurrent(u8),
    DirectCurrentEssential,
    DirectCurrentEssentialShed,
    DirectCurrentBattery,
    DirectCurrentHot(u8),
}
impl ElectricalBusType {
    fn get_name(&self) -> String {
        match self {
            ElectricalBusType::AlternatingCurrent(number) => format!("AC_{}", number),
            ElectricalBusType::AlternatingCurrentEssential => String::from("AC_ESS"),
            ElectricalBusType::AlternatingCurrentEssentialShed => String::from("AC_ESS_SHED"),
            ElectricalBusType::AlternatingCurrentStaticInverter => String::from("AC_STAT_INV"),
            ElectricalBusType::DirectCurrent(number) => format!("DC_{}", number),
            ElectricalBusType::DirectCurrentEssential => String::from("DC_ESS"),
            ElectricalBusType::DirectCurrentEssentialShed => String::from("DC_ESS_SHED"),
            ElectricalBusType::DirectCurrentBattery => String::from("DC_BAT"),
            ElectricalBusType::DirectCurrentHot(number) => format!("DC_HOT_{}", number),
        }
    }
}

pub struct ElectricalBus {
    bus_powered_id: String,
    input: Current,
    bus_type: ElectricalBusType,
}
impl ElectricalBus {
    pub fn new(bus_type: ElectricalBusType) -> ElectricalBus {
        ElectricalBus {
            bus_powered_id: format!("ELEC_{}_BUS_IS_POWERED", bus_type.get_name()),
            input: Current::none(),
            bus_type,
        }
    }

    fn get_type(&self) -> ElectricalBusType {
        self.bus_type
    }

    fn get_power_source(&self) -> Option<ElectricPowerSource> {
        self.output().get_source()
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
impl ElectricSource for ElectricalBus {
    fn output(&self) -> Current {
        self.input
    }
}
impl SimulatorElementVisitable for ElectricalBus {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for ElectricalBus {
    fn write(&self, state: &mut SimulatorWriter) {
        state.write_bool(&self.bus_powered_id, self.is_powered());
    }
}

pub struct ElectricalStateWriter {
    current_id: String,
    current_normal_id: String,
    potential_id: String,
    potential_normal_id: String,
    frequency_id: String,
    frequency_normal_id: String,
    load_id: String,
    load_normal_id: String,
}
impl ElectricalStateWriter {
    pub fn new(element_id: &str) -> Self {
        Self {
            current_id: format!("ELEC_{}_CURRENT", element_id),
            current_normal_id: format!("ELEC_{}_CURRENT_NORMAL", element_id),
            potential_id: format!("ELEC_{}_POTENTIAL", element_id),
            potential_normal_id: format!("ELEC_{}_POTENTIAL_NORMAL", element_id),
            frequency_id: format!("ELEC_{}_FREQUENCY", element_id),
            frequency_normal_id: format!("ELEC_{}_FREQUENCY_NORMAL", element_id),
            load_id: format!("ELEC_{}_LOAD", element_id),
            load_normal_id: format!("ELEC_{}_LOAD_NORMAL", element_id),
        }
    }

    pub fn write_direct<T: ProvideCurrent + ProvidePotential>(
        &self,
        source: &T,
        state: &mut SimulatorWriter,
    ) {
        self.write_current(source, state);
        self.write_potential(source, state);
    }

    pub fn write_alternating<T: ProvidePotential + ProvideFrequency>(
        &self,
        source: &T,
        state: &mut SimulatorWriter,
    ) {
        self.write_potential(source, state);
        self.write_frequency(source, state);
    }

    pub fn write_alternating_with_load<T: ProvidePotential + ProvideFrequency + ProvideLoad>(
        &self,
        source: &T,
        state: &mut SimulatorWriter,
    ) {
        self.write_alternating(source, state);
        self.write_load(source, state);
    }

    fn write_current<T: ProvideCurrent>(&self, source: &T, state: &mut SimulatorWriter) {
        state.write_f64(&self.current_id, source.get_current().get::<ampere>());
        state.write_bool(&self.current_normal_id, source.get_current_normal());
    }

    fn write_potential<T: ProvidePotential>(&self, source: &T, state: &mut SimulatorWriter) {
        state.write_f64(&self.potential_id, source.get_potential().get::<volt>());
        state.write_bool(&self.potential_normal_id, source.get_potential_normal());
    }

    fn write_frequency<T: ProvideFrequency>(&self, source: &T, state: &mut SimulatorWriter) {
        state.write_f64(&self.frequency_id, source.get_frequency().get::<hertz>());
        state.write_bool(&self.frequency_normal_id, source.get_frequency_normal());
    }

    fn write_load<T: ProvideLoad>(&self, source: &T, state: &mut SimulatorWriter) {
        state.write_f64(&self.load_id, source.get_load().get::<percent>());
        state.write_bool(&self.load_normal_id, source.get_load_normal());
    }
}

pub trait ProvideCurrent {
    fn get_current(&self) -> ElectricCurrent;
    fn get_current_normal(&self) -> bool;
}

pub trait ProvidePotential {
    fn get_potential(&self) -> ElectricPotential;
    fn get_potential_normal(&self) -> bool;
}

pub trait ProvideFrequency {
    fn get_frequency(&self) -> Frequency;
    fn get_frequency_normal(&self) -> bool;
}

pub trait ProvideLoad {
    fn get_load(&self) -> Ratio;
    fn get_load_normal(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use uom::si::frequency::hertz;

    use super::*;
    struct Powerless {}
    impl ElectricSource for Powerless {
        fn output(&self) -> Current {
            Current::none()
        }
    }

    struct StubApuGenerator {}
    impl ElectricSource for StubApuGenerator {
        fn output(&self) -> Current {
            Current::some(ElectricPowerSource::ApuGenerator)
        }
    }

    struct StubElectricSource {}
    impl ProvideCurrent for StubElectricSource {
        fn get_current(&self) -> ElectricCurrent {
            ElectricCurrent::new::<ampere>(150.)
        }

        fn get_current_normal(&self) -> bool {
            true
        }
    }
    impl ProvidePotential for StubElectricSource {
        fn get_potential(&self) -> ElectricPotential {
            ElectricPotential::new::<volt>(28.)
        }

        fn get_potential_normal(&self) -> bool {
            true
        }
    }
    impl ProvideFrequency for StubElectricSource {
        fn get_frequency(&self) -> Frequency {
            Frequency::new::<hertz>(400.)
        }

        fn get_frequency_normal(&self) -> bool {
            true
        }
    }
    impl ProvideLoad for StubElectricSource {
        fn get_load(&self) -> Ratio {
            Ratio::new::<percent>(50.)
        }

        fn get_load_normal(&self) -> bool {
            true
        }
    }

    #[cfg(test)]
    mod powerable_tests {
        use super::*;

        struct BatteryStub {
            current: Current,
        }

        impl BatteryStub {
            fn new(current: Current) -> BatteryStub {
                BatteryStub { current }
            }
        }

        impl ElectricSource for BatteryStub {
            fn output(&self) -> Current {
                self.current
            }
        }

        struct PowerableUnderTest {
            input: Current,
        }
        impl PowerableUnderTest {
            fn new() -> PowerableUnderTest {
                PowerableUnderTest {
                    input: Current::none(),
                }
            }
        }
        impl Powerable for PowerableUnderTest {
            fn set_input(&mut self, current: Current) {
                self.input = current;
            }

            fn get_input(&self) -> Current {
                self.input
            }
        }

        #[test]
        fn or_powered_by_both_batteries_results_in_both_when_both_connected() {
            let bat_1 = BatteryStub::new(Current::some(ElectricPowerSource::Battery(10)));
            let bat_2 = BatteryStub::new(Current::some(ElectricPowerSource::Battery(11)));

            let expected = Current::some(ElectricPowerSource::Batteries);

            let mut powerable = PowerableUnderTest::new();

            let mut contactor_1 = Contactor::new("BAT1");
            contactor_1.powered_by(&bat_1);
            contactor_1.close_when(true);

            let mut contactor_2 = Contactor::new("BAT2");
            contactor_2.powered_by(&bat_2);
            contactor_2.close_when(true);

            powerable.or_powered_by_both_batteries(&contactor_1, &contactor_2);

            assert_eq!(powerable.get_input(), expected)
        }

        #[test]
        fn or_powered_by_battery_1_results_in_bat_1_output() {
            let expected = Current::some(ElectricPowerSource::Battery(10));

            let bat_1 = BatteryStub::new(expected);
            let bat_2 = BatteryStub::new(Current::none());

            or_powered_by_battery_results_in_expected_output(bat_1, bat_2, expected);
        }

        #[test]
        fn or_powered_by_battery_2_results_in_bat_2_output() {
            let expected = Current::some(ElectricPowerSource::Battery(11));

            let bat_1 = BatteryStub::new(Current::none());
            let bat_2 = BatteryStub::new(expected);

            or_powered_by_battery_results_in_expected_output(bat_1, bat_2, expected);
        }

        fn or_powered_by_battery_results_in_expected_output(
            bat_1: BatteryStub,
            bat_2: BatteryStub,
            expected: Current,
        ) {
            let mut powerable = PowerableUnderTest::new();

            let mut contactor_1 = Contactor::new("BAT1");
            contactor_1.powered_by(&bat_1);
            contactor_1.close_when(true);

            let mut contactor_2 = Contactor::new("BAT2");
            contactor_2.powered_by(&bat_2);
            contactor_2.close_when(true);

            powerable.or_powered_by_both_batteries(&contactor_1, &contactor_2);

            assert_eq!(powerable.get_input(), expected);
        }
    }

    #[cfg(test)]
    mod current_tests {
        use super::*;

        #[test]
        fn some_current_is_powered() {
            assert_eq!(some_current().is_powered(), true);
        }

        #[test]
        fn some_current_is_not_unpowered() {
            assert_eq!(some_current().is_unpowered(), false);
        }

        #[test]
        fn none_current_is_not_powered() {
            assert_eq!(none_current().is_powered(), false);
        }

        #[test]
        fn none_current_is_unpowered() {
            assert_eq!(none_current().is_unpowered(), true);
        }

        fn some_current() -> Current {
            Current::some(ElectricPowerSource::ApuGenerator)
        }

        fn none_current() -> Current {
            Current::none()
        }
    }

    #[cfg(test)]
    mod electrical_bus_type_tests {
        use crate::electrical::ElectricalBusType;

        #[test]
        fn get_name_returns_name() {
            assert_eq!(ElectricalBusType::AlternatingCurrent(2).get_name(), "AC_2");
            assert_eq!(
                ElectricalBusType::AlternatingCurrentEssential.get_name(),
                "AC_ESS"
            );
            assert_eq!(
                ElectricalBusType::AlternatingCurrentEssentialShed.get_name(),
                "AC_ESS_SHED"
            );
            assert_eq!(
                ElectricalBusType::AlternatingCurrentStaticInverter.get_name(),
                "AC_STAT_INV"
            );
            assert_eq!(ElectricalBusType::DirectCurrent(2).get_name(), "DC_2");
            assert_eq!(
                ElectricalBusType::DirectCurrentEssential.get_name(),
                "DC_ESS"
            );
            assert_eq!(
                ElectricalBusType::DirectCurrentEssentialShed.get_name(),
                "DC_ESS_SHED"
            );
            assert_eq!(ElectricalBusType::DirectCurrentBattery.get_name(), "DC_BAT");
            assert_eq!(
                ElectricalBusType::DirectCurrentHot(2).get_name(),
                "DC_HOT_2"
            );
        }
    }

    #[cfg(test)]
    mod electrical_bus_tests {
        use super::*;

        #[test]
        fn writes_its_state() {
            let bus = electrical_bus();
            let mut state = SimulatorWriter::new_for_test();

            bus.write(&mut state);

            assert!(state.len_is(1));
            assert!(state.contains_bool("ELEC_AC_2_BUS_IS_POWERED", false));
        }

        fn electrical_bus() -> ElectricalBus {
            ElectricalBus::new(ElectricalBusType::AlternatingCurrent(2))
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

        fn contactor_has_no_output_when_powered_by_nothing(contactor: Contactor) {
            assert!(contactor.is_unpowered());
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
            contactor.powered_by(&Powerless {});

            assert!(contactor.is_unpowered());
        }

        #[test]
        fn open_contactor_has_no_output_when_powered_by_something() {
            let mut contactor = open_contactor();
            contactor.powered_by(&Powerless {});
            contactor.or_powered_by(&StubApuGenerator {});

            assert!(contactor.is_unpowered());
        }

        #[test]
        fn closed_contactor_has_output_when_powered_by_something_which_is_powered() {
            let mut contactor = closed_contactor();
            contactor.powered_by(&Powerless {});
            contactor.or_powered_by(&StubApuGenerator {});

            assert!(contactor.is_powered());
        }

        #[test]
        fn writes_its_state() {
            let contactor = contactor();
            let mut state = SimulatorWriter::new_for_test();

            contactor.write(&mut state);

            assert!(state.len_is(1));
            assert!(state.contains_bool("ELEC_CONTACTOR_TEST_IS_CLOSED", false));
        }

        fn contactor() -> Contactor {
            Contactor::new("TEST")
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
    mod current_state_writer_tests {
        use super::*;

        #[test]
        fn writes_direct_current_state() {
            let writer = ElectricalStateWriter::new("BAT_2");
            let mut state = SimulatorWriter::new_for_test();

            writer.write_direct(&StubElectricSource {}, &mut state);

            assert!(state.len_is(4));
            assert!(state.contains_f64("ELEC_BAT_2_CURRENT", 150.));
            assert!(state.contains_bool("ELEC_BAT_2_CURRENT_NORMAL", true));
            assert!(state.contains_f64("ELEC_BAT_2_POTENTIAL", 28.));
            assert!(state.contains_bool("ELEC_BAT_2_POTENTIAL_NORMAL", true));
        }

        #[test]
        fn writes_alternating_current_state() {
            let writer = ElectricalStateWriter::new("APU_GEN");
            let mut state = SimulatorWriter::new_for_test();

            writer.write_alternating(&StubElectricSource {}, &mut state);

            assert!(state.len_is(4));
            assert!(state.contains_f64("ELEC_APU_GEN_POTENTIAL", 28.));
            assert!(state.contains_bool("ELEC_APU_GEN_POTENTIAL_NORMAL", true));
            assert!(state.contains_f64("ELEC_APU_GEN_FREQUENCY", 400.));
            assert!(state.contains_bool("ELEC_APU_GEN_FREQUENCY_NORMAL", true));
        }

        #[test]
        fn writes_alternating_current_with_load_state() {
            let writer = ElectricalStateWriter::new("APU_GEN");
            let mut state = SimulatorWriter::new_for_test();

            writer.write_alternating_with_load(&StubElectricSource {}, &mut state);

            assert!(state.len_is(6));
            assert!(state.contains_f64("ELEC_APU_GEN_POTENTIAL", 28.));
            assert!(state.contains_bool("ELEC_APU_GEN_POTENTIAL_NORMAL", true));
            assert!(state.contains_f64("ELEC_APU_GEN_FREQUENCY", 400.));
            assert!(state.contains_bool("ELEC_APU_GEN_FREQUENCY_NORMAL", true));
            assert!(state.contains_f64("ELEC_APU_GEN_LOAD", 50.));
            assert!(state.contains_bool("ELEC_APU_GEN_LOAD_NORMAL", true));
        }
    }
}
