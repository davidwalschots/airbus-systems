//! Provides things one needs for the electrical system of an aircraft.

mod battery;
pub mod consumption;
mod emergency_generator;
mod engine_generator;
mod external_power_source;
mod static_inverter;
mod transformer_rectifier;
use std::{fmt::Display, hash::Hash};

pub use battery::Battery;
pub use emergency_generator::EmergencyGenerator;
pub use engine_generator::{EngineGenerator, EngineGeneratorUpdateArguments};
pub use external_power_source::ExternalPowerSource;
pub use static_inverter::StaticInverter;
pub use transformer_rectifier::TransformerRectifier;

use crate::simulation::{SimulationElement, SimulatorWriter};
use uom::si::{
    electric_current::ampere, electric_potential::volt, f64::*, frequency::hertz, ratio::percent,
};

use self::consumption::SuppliedPower;

pub trait ElectricalSystem {
    fn get_supplied_power(&self) -> SuppliedPower;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PotentialOrigin {
    None,
    EngineGenerator(usize),
    ApuGenerator(usize),
    External,
    EmergencyGenerator,
    Battery(usize),
    Batteries,
    TransformerRectifier(usize),
    StaticInverter,
}

/// Within an electrical system, electric potential is made available by an origin.
/// These origins are contained in this type. By knowing the origin of potential
/// for all power consumers one can determine the amount of electric current provided
/// by the origin to the whole aircraft.
///
/// Note that this type shouldn't be confused with uom's `ElectricPotential`, which provides
/// the base unit (including volt) for defining the amount of potential.
/// The raw `ElectricPotential` is included in this type for presentational purposes, and to
/// decide between two potential origins in a parallel circuit.
///
/// The raw `ElectricPotential` is ignored when determining if the potential
/// is powered or not. If we wouldn't ignore it, passing electric potential across the
/// circuit would take multiple simulation ticks, as _V_ for origins (ENG GEN, TR, etc)
/// can only be calculated when electrical consumption is known, which is the case at
/// the end of a simulation tick.
///
/// As the raw `ElectricPotential` is of less importance for the majority of code,
/// it is not taken into account when checking for partial equality.
///
/// For the reasons outlined above when creating e.g. an engine generator, ensure you
/// return `Potential::none()` when the generator isn't supplying potential, and
/// `Potential::engine_generator(usize).with_raw(ElectricPotential)` when it is.
#[derive(Clone, Copy, Debug)]
pub struct Potential {
    origin: PotentialOrigin,
    raw: ElectricPotential,
}
impl Potential {
    fn new_without_raw(origin: PotentialOrigin) -> Self {
        Self {
            origin,
            raw: ElectricPotential::new::<volt>(0.),
        }
    }

    pub fn none() -> Self {
        Self::new_without_raw(PotentialOrigin::None)
    }

    pub fn engine_generator(number: usize) -> Self {
        Self::new_without_raw(PotentialOrigin::EngineGenerator(number))
    }

    pub fn apu_generator(number: usize) -> Self {
        Self::new_without_raw(PotentialOrigin::ApuGenerator(number))
    }

    pub fn external() -> Self {
        Self::new_without_raw(PotentialOrigin::External)
    }

    pub fn emergency_generator() -> Self {
        Self::new_without_raw(PotentialOrigin::EmergencyGenerator)
    }

    pub fn battery(number: usize) -> Self {
        Self::new_without_raw(PotentialOrigin::Battery(number))
    }

    pub fn batteries() -> Self {
        Self::new_without_raw(PotentialOrigin::Batteries)
    }

    pub fn transformer_rectifier(number: usize) -> Self {
        Self::new_without_raw(PotentialOrigin::TransformerRectifier(number))
    }

    pub fn static_inverter() -> Self {
        Self::new_without_raw(PotentialOrigin::StaticInverter)
    }

    pub fn with_raw(mut self, potential: ElectricPotential) -> Self {
        debug_assert!(self.origin != PotentialOrigin::None);

        self.raw = potential;
        self
    }

    /// Indicates if the instance provides electric potential.
    pub fn is_powered(&self) -> bool {
        self.origin != PotentialOrigin::None
    }

    /// Indicates if the instance does not provide electric potential.
    pub fn is_unpowered(&self) -> bool {
        self.origin == PotentialOrigin::None
    }

    pub fn origin(&self) -> PotentialOrigin {
        self.origin
    }

    pub fn raw(&self) -> ElectricPotential {
        self.raw
    }
}
impl PartialEq for Potential {
    fn eq(&self, other: &Self) -> bool {
        self.origin.eq(&other.origin)
    }
}
impl PartialOrd for Potential {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.raw.get::<volt>().partial_cmp(&other.raw.get::<volt>())
    }
}
impl Default for Potential {
    fn default() -> Self {
        Potential::none()
    }
}

/// A source of electric potential. A source is not necessarily the
/// origin of the potential. It can also be a conductor.
pub trait PotentialSource {
    fn output(&self) -> Potential;

    /// Indicates if the instance provides electric potential.
    fn is_powered(&self) -> bool {
        self.output().is_powered()
    }

    /// Indicates if the instance does not provide electric potential.
    fn is_unpowered(&self) -> bool {
        self.output().is_unpowered()
    }
}

/// A target for electric potential.
///
/// # Examples
///
/// To implement this trait, use the `potential_target!` macro when working within
/// the systems crate. When adding a new type outside of the crate, use the example
/// below:
/// ```rust
/// # use systems::electrical::{Potential, PotentialSource, PotentialTarget};
/// # struct MyType {
/// #     input: Potential,
/// # }
/// impl PotentialTarget for MyType {
///     fn powered_by<T: PotentialSource + ?Sized>(&mut self, source: &T) {
///         self.input = source.output();
///     }
///
///     fn or_powered_by<T: PotentialSource + ?Sized>(&mut self, source: &T) {
///         if self.input.is_unpowered() {
///             self.powered_by(source);
///         }
///     }
/// }
/// ```
pub trait PotentialTarget {
    /// Powers the instance with the given source's potential. When the given source has no potential
    /// the instance also won't have potential.
    fn powered_by<T: PotentialSource + ?Sized>(&mut self, source: &T);

    /// Powers the instance with the given source's potential. When the given source has no potential
    /// the instance keeps its existing potential.
    fn or_powered_by<T: PotentialSource + ?Sized>(&mut self, source: &T);
}

/// Represents a contactor in an electrical power circuit.
/// When closed a contactor conducts the potential towards other targets.
#[derive(Debug)]
pub struct Contactor {
    closed_id: String,
    closed: bool,
    input: Potential,
}
impl Contactor {
    pub fn new(id: &str) -> Contactor {
        Contactor {
            closed_id: format!("ELEC_CONTACTOR_{}_IS_CLOSED", id),
            closed: false,
            input: Potential::none(),
        }
    }

    pub fn close_when(&mut self, should_be_closed: bool) {
        self.closed = should_be_closed;
    }

    pub fn is_open(&self) -> bool {
        !self.closed
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }
}
potential_target!(Contactor);
impl PotentialSource for Contactor {
    fn output(&self) -> Potential {
        if self.closed {
            self.input
        } else {
            Potential::none()
        }
    }
}
impl SimulationElement for Contactor {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_bool(&self.closed_id, self.is_closed());
    }
}

/// Combines multiple sources of potential, such that they can be passed
/// to a target of potential as a single unit. The potential origin with the highest
/// voltage is returned.
///
/// # Examples
///
/// This function is most useful when combining sources that are in one
/// struct for use in another struct.
/// ```rust
/// # use systems::electrical::{Contactor, combine_potential_sources, ElectricalBus,
/// #     ElectricalBusType, PotentialTarget, CombinedPotentialSource};
/// struct MainPowerSources {
///     engine_1_gen_contactor: Contactor,
///     bus_tie_1_contactor: Contactor,
/// }
/// impl MainPowerSources {
///     fn new() -> Self {
///         Self {
///             engine_1_gen_contactor: Contactor::new("9XU1"),
///             bus_tie_1_contactor: Contactor::new("11XU1"),
///         }
///     }
///
///     fn ac_bus_1_electric_sources(&self) -> CombinedPotentialSource {
///         combine_potential_sources(vec![
///             &self.engine_1_gen_contactor,
///             &self.bus_tie_1_contactor,
///         ])
///     }
/// }
///
/// let mut ac_bus_1 = ElectricalBus::new(ElectricalBusType::AlternatingCurrent(1));
/// let main_power_sources = MainPowerSources::new();
///
/// ac_bus_1.powered_by(&main_power_sources.ac_bus_1_electric_sources());
/// ```
/// When a potential target can be powered by multiple sources in the same struct and
/// the potential origin doesn't matter as you don't expect the sources to be powered
/// by different potential origins, prefer using the `powered_by` and `or_powered_by`
/// functions as follows:
/// ```rust
/// # use systems::electrical::{Contactor, ElectricalBus,
/// #     ElectricalBusType, PotentialTarget};
/// let mut ac_bus_1 = ElectricalBus::new(ElectricalBusType::AlternatingCurrent(1));
/// let engine_1_gen_contactor = Contactor::new("9XU1");
/// let bus_tie_1_contactor = Contactor::new("11XU1");
///
/// ac_bus_1.powered_by(&engine_1_gen_contactor);
/// ac_bus_1.or_powered_by(&bus_tie_1_contactor);
/// ```
pub fn combine_potential_sources<T: PotentialSource>(sources: Vec<&T>) -> CombinedPotentialSource {
    CombinedPotentialSource::new(sources)
}

/// Refer to [`combine_potential_sources`] for details.
///
/// [`combine_potential_sources`]: fn.combine_potential_sources.html
pub struct CombinedPotentialSource {
    potential: Potential,
}
impl CombinedPotentialSource {
    fn new<T: PotentialSource>(sources: Vec<&T>) -> Self {
        let mut sources: Vec<_> = sources
            .iter()
            .map(|x| x.output())
            .filter(|x| x.is_powered())
            .collect();
        sources.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let x = sources.last();
        CombinedPotentialSource {
            potential: match x {
                Some(potential) => *potential,
                None => Potential::none(),
            },
        }
    }
}
impl PotentialSource for CombinedPotentialSource {
    fn output(&self) -> Potential {
        self.potential
    }
}

/// The common types of electrical buses within Airbus aircraft.
/// These include types such as AC, DC, AC ESS, etc.
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
impl Display for ElectricalBusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElectricalBusType::AlternatingCurrent(number) => write!(f, "AC_{}", number),
            ElectricalBusType::AlternatingCurrentEssential => write!(f, "AC_ESS"),
            ElectricalBusType::AlternatingCurrentEssentialShed => write!(f, "AC_ESS_SHED"),
            ElectricalBusType::AlternatingCurrentStaticInverter => write!(f, "AC_STAT_INV"),
            ElectricalBusType::DirectCurrent(number) => write!(f, "DC_{}", number),
            ElectricalBusType::DirectCurrentEssential => write!(f, "DC_ESS"),
            ElectricalBusType::DirectCurrentEssentialShed => write!(f, "DC_ESS_SHED"),
            ElectricalBusType::DirectCurrentBattery => write!(f, "DC_BAT"),
            ElectricalBusType::DirectCurrentHot(number) => write!(f, "DC_HOT_{}", number),
        }
    }
}

pub struct ElectricalBus {
    bus_powered_id: String,
    input: Potential,
    bus_type: ElectricalBusType,
}
impl ElectricalBus {
    pub fn new(bus_type: ElectricalBusType) -> ElectricalBus {
        ElectricalBus {
            bus_powered_id: format!("ELEC_{}_BUS_IS_POWERED", bus_type.to_string()),
            input: Potential::none(),
            bus_type,
        }
    }

    fn bus_type(&self) -> ElectricalBusType {
        self.bus_type
    }

    #[cfg(test)]
    fn input_potential(&self) -> Potential {
        self.input
    }

    pub fn or_powered_by_both_batteries(
        &mut self,
        battery_1_contactor: &Contactor,
        battery_2_contactor: &Contactor,
    ) {
        if self.input.is_unpowered() {
            self.input = if ElectricalBus::batteries_have_equal_potential(
                battery_1_contactor,
                battery_2_contactor,
            ) {
                Potential::batteries().with_raw(battery_1_contactor.output().raw())
            } else {
                combine_potential_sources(vec![battery_1_contactor, battery_2_contactor]).output()
            }
        }
    }

    fn batteries_have_equal_potential(
        battery_1_contactor: &Contactor,
        battery_2_contactor: &Contactor,
    ) -> bool {
        battery_1_contactor.is_powered()
            && battery_2_contactor.is_powered()
            && battery_1_contactor.output().raw() == battery_2_contactor.output().raw()
    }
}
potential_target!(ElectricalBus);
impl PotentialSource for ElectricalBus {
    fn output(&self) -> Potential {
        self.input
    }
}
impl SimulationElement for ElectricalBus {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_bool(&self.bus_powered_id, self.is_powered());
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
        writer: &mut SimulatorWriter,
    ) {
        self.write_current(source, writer);
        self.write_potential(source, writer);
    }

    pub fn write_alternating<T: ProvidePotential + ProvideFrequency>(
        &self,
        source: &T,
        writer: &mut SimulatorWriter,
    ) {
        self.write_potential(source, writer);
        self.write_frequency(source, writer);
    }

    pub fn write_alternating_with_load<T: ProvidePotential + ProvideFrequency + ProvideLoad>(
        &self,
        source: &T,
        writer: &mut SimulatorWriter,
    ) {
        self.write_alternating(source, writer);
        self.write_load(source, writer);
    }

    fn write_current<T: ProvideCurrent>(&self, source: &T, writer: &mut SimulatorWriter) {
        writer.write_f64(&self.current_id, source.current().get::<ampere>());
        writer.write_bool(&self.current_normal_id, source.current_normal());
    }

    fn write_potential<T: ProvidePotential>(&self, source: &T, writer: &mut SimulatorWriter) {
        writer.write_f64(&self.potential_id, source.potential().get::<volt>());
        writer.write_bool(&self.potential_normal_id, source.potential_normal());
    }

    fn write_frequency<T: ProvideFrequency>(&self, source: &T, writer: &mut SimulatorWriter) {
        writer.write_f64(&self.frequency_id, source.frequency().get::<hertz>());
        writer.write_bool(&self.frequency_normal_id, source.frequency_normal());
    }

    fn write_load<T: ProvideLoad>(&self, source: &T, writer: &mut SimulatorWriter) {
        writer.write_f64(&self.load_id, source.load().get::<percent>());
        writer.write_bool(&self.load_normal_id, source.load_normal());
    }
}

pub trait ProvideCurrent {
    fn current(&self) -> ElectricCurrent;
    fn current_normal(&self) -> bool;
}

pub trait ProvidePotential {
    fn potential(&self) -> ElectricPotential;
    fn potential_normal(&self) -> bool;
}

pub trait ProvideFrequency {
    fn frequency(&self) -> Frequency;
    fn frequency_normal(&self) -> bool;
}

pub trait ProvideLoad {
    fn load(&self) -> Ratio;
    fn load_normal(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use uom::si::frequency::hertz;

    use super::*;
    struct Powerless {}
    impl PotentialSource for Powerless {
        fn output(&self) -> Potential {
            Potential::none()
        }
    }

    struct StubApuGenerator {}
    impl PotentialSource for StubApuGenerator {
        fn output(&self) -> Potential {
            Potential::apu_generator(1).with_raw(ElectricPotential::new::<volt>(115.))
        }
    }

    struct StubElectricSource {}
    impl ProvideCurrent for StubElectricSource {
        fn current(&self) -> ElectricCurrent {
            ElectricCurrent::new::<ampere>(150.)
        }

        fn current_normal(&self) -> bool {
            true
        }
    }
    impl ProvidePotential for StubElectricSource {
        fn potential(&self) -> ElectricPotential {
            ElectricPotential::new::<volt>(28.)
        }

        fn potential_normal(&self) -> bool {
            true
        }
    }
    impl ProvideFrequency for StubElectricSource {
        fn frequency(&self) -> Frequency {
            Frequency::new::<hertz>(400.)
        }

        fn frequency_normal(&self) -> bool {
            true
        }
    }
    impl ProvideLoad for StubElectricSource {
        fn load(&self) -> Ratio {
            Ratio::new::<percent>(50.)
        }

        fn load_normal(&self) -> bool {
            true
        }
    }

    #[cfg(test)]
    mod potential_tests {
        use super::*;

        #[test]
        fn some_potential_is_powered() {
            assert_eq!(some_potential().is_powered(), true);
        }

        #[test]
        fn some_potential_is_not_unpowered() {
            assert_eq!(some_potential().is_unpowered(), false);
        }

        #[test]
        fn none_potential_is_not_powered() {
            assert_eq!(none_potential().is_powered(), false);
        }

        #[test]
        fn none_potential_is_unpowered() {
            assert_eq!(none_potential().is_unpowered(), true);
        }

        #[test]
        fn equality_is_based_on_potential_origin_and_ignores_electric_potential() {
            assert_eq!(Potential::none(), Potential::none());
            assert_eq!(
                Potential::engine_generator(1),
                Potential::engine_generator(1)
            );
            assert_eq!(Potential::apu_generator(1), Potential::apu_generator(1));
            assert_eq!(Potential::external(), Potential::external());
            assert_eq!(
                Potential::emergency_generator(),
                Potential::emergency_generator()
            );
            assert_eq!(Potential::battery(1), Potential::battery(1));
            assert_eq!(Potential::batteries(), Potential::batteries());
            assert_eq!(
                Potential::transformer_rectifier(1),
                Potential::transformer_rectifier(1)
            );
            assert_eq!(Potential::static_inverter(), Potential::static_inverter());
        }

        #[test]
        fn not_equal_when_numbered_potential_origin_is_different() {
            assert_ne!(
                Potential::engine_generator(1),
                Potential::engine_generator(2)
            );
            assert_ne!(Potential::apu_generator(1), Potential::apu_generator(2));
            assert_ne!(Potential::battery(1), Potential::battery(2));
            assert_ne!(
                Potential::transformer_rectifier(1),
                Potential::transformer_rectifier(2)
            );
        }

        fn some_potential() -> Potential {
            Potential::apu_generator(1)
        }

        fn none_potential() -> Potential {
            Potential::none()
        }
    }

    #[cfg(test)]
    mod electrical_bus_type_tests {
        use crate::electrical::ElectricalBusType;

        #[test]
        fn get_name_returns_name() {
            assert_eq!(ElectricalBusType::AlternatingCurrent(2).to_string(), "AC_2");
            assert_eq!(
                ElectricalBusType::AlternatingCurrentEssential.to_string(),
                "AC_ESS"
            );
            assert_eq!(
                ElectricalBusType::AlternatingCurrentEssentialShed.to_string(),
                "AC_ESS_SHED"
            );
            assert_eq!(
                ElectricalBusType::AlternatingCurrentStaticInverter.to_string(),
                "AC_STAT_INV"
            );
            assert_eq!(ElectricalBusType::DirectCurrent(2).to_string(), "DC_2");
            assert_eq!(
                ElectricalBusType::DirectCurrentEssential.to_string(),
                "DC_ESS"
            );
            assert_eq!(
                ElectricalBusType::DirectCurrentEssentialShed.to_string(),
                "DC_ESS_SHED"
            );
            assert_eq!(
                ElectricalBusType::DirectCurrentBattery.to_string(),
                "DC_BAT"
            );
            assert_eq!(
                ElectricalBusType::DirectCurrentHot(2).to_string(),
                "DC_HOT_2"
            );
        }
    }

    #[cfg(test)]
    mod electrical_bus_tests {
        use super::*;
        use crate::simulation::test::SimulationTestBed;

        #[test]
        fn writes_its_state() {
            let mut bus = electrical_bus();
            let mut test_bed = SimulationTestBed::new();
            test_bed.run_without_update(&mut bus);

            assert!(test_bed.contains_key("ELEC_AC_2_BUS_IS_POWERED"));
        }

        struct BatteryStub {
            potential: Potential,
        }

        impl BatteryStub {
            fn new(potential: Potential) -> BatteryStub {
                BatteryStub { potential }
            }
        }

        impl PotentialSource for BatteryStub {
            fn output(&self) -> Potential {
                self.potential
            }
        }

        #[test]
        fn or_powered_by_both_batteries_results_in_both_when_both_connected_with_equal_voltage() {
            let potential = ElectricPotential::new::<volt>(28.);
            let bat_1 = BatteryStub::new(Potential::battery(10).with_raw(potential));
            let bat_2 = BatteryStub::new(Potential::battery(11).with_raw(potential));

            let expected = Potential::batteries().with_raw(potential);

            let mut bus = electrical_bus();

            let mut contactor_1 = Contactor::new("BAT1");
            contactor_1.powered_by(&bat_1);
            contactor_1.close_when(true);

            let mut contactor_2 = Contactor::new("BAT2");
            contactor_2.powered_by(&bat_2);
            contactor_2.close_when(true);

            bus.or_powered_by_both_batteries(&contactor_1, &contactor_2);

            assert_eq!(bus.input_potential(), expected);
            assert_eq!(bus.input_potential().raw(), expected.raw());
        }

        #[test]
        fn or_powered_by_battery_1_with_higher_voltage_results_in_bat_1_output() {
            let expected = Potential::battery(10).with_raw(ElectricPotential::new::<volt>(28.));

            let bat_1 = BatteryStub::new(expected);
            let bat_2 = BatteryStub::new(
                Potential::battery(11).with_raw(ElectricPotential::new::<volt>(25.)),
            );

            or_powered_by_battery_results_in_expected_output(bat_1, bat_2, expected);
        }

        #[test]
        fn or_powered_by_battery_1_results_in_bat_1_output() {
            let expected = Potential::battery(10).with_raw(ElectricPotential::new::<volt>(28.));

            let bat_1 = BatteryStub::new(expected);
            let bat_2 = BatteryStub::new(Potential::none());

            or_powered_by_battery_results_in_expected_output(bat_1, bat_2, expected);
        }

        #[test]
        fn or_powered_by_battery_2_with_higher_voltage_results_in_bat_2_output() {
            let expected = Potential::battery(11).with_raw(ElectricPotential::new::<volt>(28.));

            let bat_1 = BatteryStub::new(
                Potential::battery(10).with_raw(ElectricPotential::new::<volt>(25.)),
            );
            let bat_2 = BatteryStub::new(expected);

            or_powered_by_battery_results_in_expected_output(bat_1, bat_2, expected);
        }

        #[test]
        fn or_powered_by_battery_2_results_in_bat_2_output() {
            let expected = Potential::battery(11).with_raw(ElectricPotential::new::<volt>(28.));

            let bat_1 = BatteryStub::new(Potential::none());
            let bat_2 = BatteryStub::new(expected);

            or_powered_by_battery_results_in_expected_output(bat_1, bat_2, expected);
        }

        #[test]
        fn or_powered_by_none_results_in_none_output() {
            let bat_1 = BatteryStub::new(Potential::none());
            let bat_2 = BatteryStub::new(Potential::none());

            or_powered_by_battery_results_in_expected_output(bat_1, bat_2, Potential::none());
        }

        fn or_powered_by_battery_results_in_expected_output(
            bat_1: BatteryStub,
            bat_2: BatteryStub,
            expected: Potential,
        ) {
            let mut bus = electrical_bus();

            let mut contactor_1 = Contactor::new("BAT1");
            contactor_1.powered_by(&bat_1);
            contactor_1.close_when(true);

            let mut contactor_2 = Contactor::new("BAT2");
            contactor_2.powered_by(&bat_2);
            contactor_2.close_when(true);

            bus.or_powered_by_both_batteries(&contactor_1, &contactor_2);

            assert_eq!(bus.input_potential(), expected);
            assert_eq!(bus.input_potential().raw(), expected.raw());
        }

        fn electrical_bus() -> ElectricalBus {
            ElectricalBus::new(ElectricalBusType::AlternatingCurrent(2))
        }
    }

    #[cfg(test)]
    mod contactor_tests {
        use crate::simulation::test::SimulationTestBed;

        use super::*;

        #[test]
        fn contactor_starts_open() {
            assert!(contactor().is_open());
        }

        #[test]
        fn open_contactor_when_toggled_open_stays_open() {
            let mut contactor = open_contactor();
            contactor.close_when(false);

            assert!(contactor.is_open());
        }

        #[test]
        fn open_contactor_when_toggled_closed_closes() {
            let mut contactor = open_contactor();
            contactor.close_when(true);

            assert!(contactor.is_closed());
        }

        #[test]
        fn closed_contactor_when_toggled_open_opens() {
            let mut contactor = closed_contactor();
            contactor.close_when(false);

            assert!(contactor.is_open());
        }

        #[test]
        fn closed_contactor_when_toggled_closed_stays_closed() {
            let mut contactor = closed_contactor();
            contactor.close_when(true);

            assert!(contactor.is_closed());
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
            let mut contactor = contactor();
            let mut test_bed = SimulationTestBed::new();
            test_bed.run_without_update(&mut contactor);

            assert!(test_bed.contains_key("ELEC_CONTACTOR_TEST_IS_CLOSED"));
        }

        fn contactor() -> Contactor {
            Contactor::new("TEST")
        }

        fn open_contactor() -> Contactor {
            let mut contactor = contactor();
            contactor.closed = false;

            contactor
        }

        fn closed_contactor() -> Contactor {
            let mut contactor = contactor();
            contactor.closed = true;

            contactor
        }
    }

    #[cfg(test)]
    mod current_state_writer_tests {
        use super::*;
        use crate::simulation::{test::SimulationTestBed, Aircraft};

        enum WriteType {
            DirectCurrent,
            AlternatingCurrent,
            AlternatingCurrentWithLoad,
        }

        struct CurrentStateWriterTestAircraft {
            write_type: WriteType,
            writer: ElectricalStateWriter,
        }
        impl CurrentStateWriterTestAircraft {
            fn new(write_type: WriteType) -> Self {
                Self {
                    write_type,
                    writer: ElectricalStateWriter::new("TEST"),
                }
            }
        }
        impl Aircraft for CurrentStateWriterTestAircraft {}
        impl SimulationElement for CurrentStateWriterTestAircraft {
            fn write(&self, writer: &mut SimulatorWriter) {
                match self.write_type {
                    WriteType::DirectCurrent => {
                        self.writer.write_direct(&StubElectricSource {}, writer)
                    }
                    WriteType::AlternatingCurrent => self
                        .writer
                        .write_alternating(&StubElectricSource {}, writer),
                    WriteType::AlternatingCurrentWithLoad => self
                        .writer
                        .write_alternating_with_load(&StubElectricSource {}, writer),
                }
            }
        }

        #[test]
        fn writes_direct_current_state() {
            let mut aircraft = CurrentStateWriterTestAircraft::new(WriteType::DirectCurrent);
            let mut test_bed = SimulationTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.contains_key("ELEC_TEST_CURRENT"));
            assert!(test_bed.contains_key("ELEC_TEST_CURRENT_NORMAL"));
            assert!(test_bed.contains_key("ELEC_TEST_POTENTIAL"));
            assert!(test_bed.contains_key("ELEC_TEST_POTENTIAL_NORMAL"));
        }

        #[test]
        fn writes_alternating_current_state() {
            let mut aircraft = CurrentStateWriterTestAircraft::new(WriteType::AlternatingCurrent);
            let mut test_bed = SimulationTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.contains_key("ELEC_TEST_POTENTIAL"));
            assert!(test_bed.contains_key("ELEC_TEST_POTENTIAL_NORMAL"));
            assert!(test_bed.contains_key("ELEC_TEST_FREQUENCY"));
            assert!(test_bed.contains_key("ELEC_TEST_FREQUENCY_NORMAL"));
        }

        #[test]
        fn writes_alternating_current_with_load_state() {
            let mut aircraft =
                CurrentStateWriterTestAircraft::new(WriteType::AlternatingCurrentWithLoad);
            let mut test_bed = SimulationTestBed::new();

            test_bed.run_aircraft(&mut aircraft);

            assert!(test_bed.contains_key("ELEC_TEST_POTENTIAL"));
            assert!(test_bed.contains_key("ELEC_TEST_POTENTIAL_NORMAL"));
            assert!(test_bed.contains_key("ELEC_TEST_FREQUENCY"));
            assert!(test_bed.contains_key("ELEC_TEST_FREQUENCY_NORMAL"));
            assert!(test_bed.contains_key("ELEC_TEST_LOAD"));
            assert!(test_bed.contains_key("ELEC_TEST_LOAD_NORMAL"));
        }
    }
}
