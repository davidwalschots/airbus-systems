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
                self.set_input(Current::some(ElectricPowerSource::Battery(1)));
            } else if is_battery_2_powered {
                self.set_input(Current::some(ElectricPowerSource::Battery(2)));
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
    id: String,
    state: ContactorState,
    input: Current,
}
impl Contactor {
    pub fn new(id: String) -> Contactor {
        Contactor {
            id,
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

pub struct ElectricalBus {
    input: Current,
    bus_type: ElectricalBusType,
}
impl ElectricalBus {
    pub fn new(bus_type: ElectricalBusType) -> ElectricalBus {
        ElectricalBus {
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

#[cfg(test)]
mod tests {
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
            let bat_1 = BatteryStub::new(Current::some(ElectricPowerSource::Battery(1)));
            let bat_2 = BatteryStub::new(Current::some(ElectricPowerSource::Battery(2)));

            let expected = Current::some(ElectricPowerSource::Batteries);

            let mut powerable = PowerableUnderTest::new();

            let mut contactor_1 = Contactor::new(String::from("BAT1"));
            contactor_1.powered_by(&bat_1);
            contactor_1.close_when(true);

            let mut contactor_2 = Contactor::new(String::from("BAT2"));
            contactor_2.powered_by(&bat_2);
            contactor_2.close_when(true);

            powerable.or_powered_by_both_batteries(&contactor_1, &contactor_2);

            assert_eq!(powerable.get_input(), expected)
        }

        #[test]
        fn or_powered_by_battery_1_results_in_bat_1_output() {
            let expected = Current::some(ElectricPowerSource::Battery(1));

            let bat_1 = BatteryStub::new(expected);
            let bat_2 = BatteryStub::new(Current::none());

            or_powered_by_battery_results_in_expected_output(bat_1, bat_2, expected);
        }

        #[test]
        fn or_powered_by_battery_2_results_in_bat_2_output() {
            let expected = Current::some(ElectricPowerSource::Battery(2));

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

            let mut contactor_1 = Contactor::new(String::from("BAT1"));
            contactor_1.powered_by(&bat_1);
            contactor_1.close_when(true);

            let mut contactor_2 = Contactor::new(String::from("BAT2"));
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
}
