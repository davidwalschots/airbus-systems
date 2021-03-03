//! Provides things one needs for the electrical system of an aircraft.

mod battery;
pub mod consumption;
mod emergency_generator;
mod engine_generator;
mod external_power_source;
mod static_inverter;
mod transformer_rectifier;
use std::{fmt::Display, hash::Hash};

pub use battery::{Battery, BatteryChargeLimiter, BatteryChargeLimiterArguments};
pub use emergency_generator::EmergencyGenerator;
pub use engine_generator::{EngineGenerator, EngineGeneratorUpdateArguments};
pub use external_power_source::ExternalPowerSource;
use itertools::Itertools;
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
    TransformerRectifier(usize),
    StaticInverter,
}

#[derive(Clone, Copy, Debug)]
pub struct OriginWithRawPotentialPair {
    origin: PotentialOrigin,
    raw: ElectricPotential,
}
impl OriginWithRawPotentialPair {
    fn new(origin: PotentialOrigin, raw: ElectricPotential) -> Self {
        Self { origin, raw }
    }

    pub fn origin(&self) -> PotentialOrigin {
        self.origin
    }

    pub fn raw(&self) -> ElectricPotential {
        self.raw
    }
}
impl PartialEq for OriginWithRawPotentialPair {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin
    }
}
impl Eq for OriginWithRawPotentialPair {}
impl Hash for OriginWithRawPotentialPair {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.origin.hash(state);
    }
}

/// Within an electrical system, electric potential is made available by an origin.
/// These origins are contained in this type. By knowing the origin of potential
/// for all power consumers one can determine the amount of electric current provided
/// by the origin to the whole aircraft.
///
/// Note that this type shouldn't be confused with uom's `ElectricPotential`, which provides
/// the base unit (including volt) for defining the amount of potential.
/// The raw `ElectricPotential` is included in this type.
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
/// `Potential::some(PotentialOrigin::EngineGenerator(1), ElectricPotential::new::<volt>(115.))`
/// when it is.
#[derive(Clone, Copy, Debug)]
pub struct Potential {
    // As this struct is passed around quite a bit, we use a fixed sized
    // array so copying is cheaper. Creation of Potential, with two merges
    // and a clone is much cheaper: 286ns instead of 500ns with a HashSet.
    // Three elements is the maximum we expect in the A320 (BAT1, BAT2,
    // and TR1 or TR2). Should another aircraft require more one can simply
    // increase the number here and in the code below.
    elements: [Option<OriginWithRawPotentialPair>; 3],
}
impl Potential {
    pub fn none() -> Self {
        Self {
            elements: [None, None, None],
        }
    }

    pub fn single(origin: PotentialOrigin, raw: ElectricPotential) -> Self {
        Self {
            elements: [
                Some(OriginWithRawPotentialPair::new(origin, raw)),
                None,
                None,
            ],
        }
    }

    pub fn merge(&self, other: &Potential) -> Self {
        let mut elements = self
            .elements
            .iter()
            .filter_map(|&x| x)
            .chain(other.elements.iter().filter_map(|&x| x))
            .unique();

        let merged = Self {
            elements: [elements.next(), elements.next(), elements.next()],
        };

        debug_assert!(elements.count() == 0);

        merged
    }

    pub fn is_single(&self, origin: PotentialOrigin) -> bool {
        self.elements.iter().filter_map(|&x| x).count() == 1
            && self
                .elements
                .iter()
                .filter_map(|&x| x)
                .filter(|&x| x.origin == origin)
                .count()
                == 1
    }

    pub fn is_single_engine_generator(&self) -> bool {
        self.elements.iter().filter_map(|&x| x).count() == 1
            && self
                .elements
                .iter()
                .filter_map(|&x| x)
                .filter(|&x| matches!(x.origin, PotentialOrigin::EngineGenerator(_)))
                .count()
                == 1
    }

    pub fn is_pair(&self, a: PotentialOrigin, b: PotentialOrigin) -> bool {
        self.elements.iter().filter_map(|&x| x).count() == 2
            && self
                .elements
                .iter()
                .filter_map(|&x| x)
                .filter(|&x| x.origin == a || x.origin == b)
                .count()
                == 2
    }

    /// Indicates if the instance provides electric potential.
    pub fn is_powered(&self) -> bool {
        self.elements.iter().filter_map(|&x| x).count() > 0
    }

    /// Indicates if the instance does not provide electric potential.
    pub fn is_unpowered(&self) -> bool {
        !self.is_powered()
    }

    pub fn origins_with_raw_potential(&self) -> &[Option<OriginWithRawPotentialPair>] {
        &self.elements
    }
}
impl PotentialSource for Potential {
    fn output(&self) -> Potential {
        self.clone()
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
        self.input = self
            .input
            .merge(&battery_1_contactor.output())
            .merge(&battery_2_contactor.output())
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
            Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(115.),
            )
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
        fn merge_combines_different_potentials() {
            let result = Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(115.),
            )
            .merge(&Potential::single(
                PotentialOrigin::EngineGenerator(1),
                ElectricPotential::new::<volt>(115.),
            ));

            assert!(result.is_pair(
                PotentialOrigin::ApuGenerator(1),
                PotentialOrigin::EngineGenerator(1)
            ));
        }

        #[test]
        fn merge_combines_similar_potentials() {
            let result = Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(115.),
            )
            .merge(&Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(115.),
            ));

            assert!(result.is_single(PotentialOrigin::ApuGenerator(1)));
        }

        #[test]
        fn merge_ignores_none() {
            let result = Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(115.),
            )
            .merge(&Potential::none());

            assert!(result.is_single(PotentialOrigin::ApuGenerator(1)));
        }

        #[test]
        fn merge_keeps_the_left_side_raw_potential() {
            let result = Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(115.),
            )
            .merge(&Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(10.),
            ));

            assert!(if let Some(pair) = result
                .origins_with_raw_potential()
                .iter()
                .filter_map(|&x| x)
                .next()
            {
                pair.raw() == ElectricPotential::new::<volt>(115.)
            } else {
                false
            });
        }

        #[test]
        #[should_panic]
        fn merge_panics_when_merging_more_than_three_origins() {
            Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(115.),
            )
            .merge(&Potential::single(
                PotentialOrigin::EngineGenerator(1),
                ElectricPotential::new::<volt>(115.),
            ))
            .merge(&Potential::single(
                PotentialOrigin::EngineGenerator(2),
                ElectricPotential::new::<volt>(115.),
            ))
            .merge(&Potential::single(
                PotentialOrigin::EngineGenerator(3),
                ElectricPotential::new::<volt>(115.),
            ));
        }

        #[test]
        fn is_single_returns_false_when_none() {
            assert!(!Potential::none().is_single(PotentialOrigin::External));
        }

        #[test]
        fn is_single_returns_false_when_single_of_different_origin() {
            assert!(!Potential::single(
                PotentialOrigin::EngineGenerator(1),
                ElectricPotential::new::<volt>(115.)
            )
            .is_single(PotentialOrigin::External));
        }

        #[test]
        fn is_single_returns_true_when_single_of_given_origin() {
            assert!(Potential::single(
                PotentialOrigin::External,
                ElectricPotential::new::<volt>(115.)
            )
            .is_single(PotentialOrigin::External));
        }

        #[test]
        fn is_single_returns_false_when_pair() {
            assert!(!Potential::single(
                PotentialOrigin::External,
                ElectricPotential::new::<volt>(115.)
            )
            .merge(&Potential::single(
                PotentialOrigin::EngineGenerator(1),
                ElectricPotential::new::<volt>(115.)
            ))
            .is_single(PotentialOrigin::External));
        }

        #[test]
        fn is_single_engine_generator_returns_false_when_none() {
            assert!(!Potential::none().is_single_engine_generator());
        }

        #[test]
        fn is_single_engine_generator_returns_false_when_different_origin() {
            assert!(!Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(115.)
            )
            .is_single_engine_generator());
        }

        #[test]
        fn is_single_engine_generator_returns_true_when_engine_generator() {
            assert!(Potential::single(
                PotentialOrigin::EngineGenerator(2),
                ElectricPotential::new::<volt>(115.)
            )
            .is_single_engine_generator());
        }

        #[test]
        fn is_single_engine_generator_returns_false_when_pair_of_engine_generators() {
            assert!(!Potential::single(
                PotentialOrigin::EngineGenerator(1),
                ElectricPotential::new::<volt>(115.)
            )
            .merge(&Potential::single(
                PotentialOrigin::EngineGenerator(2),
                ElectricPotential::new::<volt>(115.)
            ))
            .is_single_engine_generator());
        }

        #[test]
        fn is_pair_returns_false_when_none() {
            assert!(!Potential::none().is_pair(
                PotentialOrigin::EngineGenerator(2),
                PotentialOrigin::EngineGenerator(1)
            ));
        }

        #[test]
        fn is_pair_returns_false_when_single() {
            assert!(!Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(115.)
            )
            .is_pair(
                PotentialOrigin::EngineGenerator(2),
                PotentialOrigin::EngineGenerator(1)
            ));
        }

        #[test]
        fn is_pair_returns_false_when_pair_with_different_origins() {
            assert!(!Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(115.)
            )
            .merge(&Potential::single(
                PotentialOrigin::EngineGenerator(2),
                ElectricPotential::new::<volt>(115.)
            ))
            .is_pair(
                PotentialOrigin::EngineGenerator(2),
                PotentialOrigin::EngineGenerator(1)
            ));
        }

        #[test]
        fn is_pair_returns_true_when_pair_of_given_origins_irregardless_of_order() {
            assert!(Potential::single(
                PotentialOrigin::EngineGenerator(1),
                ElectricPotential::new::<volt>(115.)
            )
            .merge(&Potential::single(
                PotentialOrigin::EngineGenerator(2),
                ElectricPotential::new::<volt>(115.)
            ))
            .is_pair(
                PotentialOrigin::EngineGenerator(2),
                PotentialOrigin::EngineGenerator(1)
            ));
        }

        fn some_potential() -> Potential {
            Potential::single(
                PotentialOrigin::ApuGenerator(1),
                ElectricPotential::new::<volt>(115.),
            )
        }

        fn none_potential() -> Potential {
            Potential::none()
        }
    }

    #[cfg(test)]
    mod origin_with_raw_potential_pair_tests {
        use super::*;
        use std::{collections::hash_map::DefaultHasher, hash::Hasher};

        #[test]
        fn equality_is_based_on_potential_origin_and_ignores_electric_potential() {
            assert_eq!(
                OriginWithRawPotentialPair::new(
                    PotentialOrigin::EngineGenerator(1),
                    ElectricPotential::new::<volt>(50.),
                ),
                OriginWithRawPotentialPair::new(
                    PotentialOrigin::EngineGenerator(1),
                    ElectricPotential::new::<volt>(115.),
                )
            );
        }

        #[test]
        fn not_equal_when_numbered_potential_origin_is_different() {
            assert_ne!(
                OriginWithRawPotentialPair::new(
                    PotentialOrigin::EngineGenerator(1),
                    ElectricPotential::new::<volt>(115.),
                ),
                OriginWithRawPotentialPair::new(
                    PotentialOrigin::EngineGenerator(2),
                    ElectricPotential::new::<volt>(115.),
                )
            );
        }

        #[test]
        fn hashes_only_potential_origin() {
            let mut first_item_hasher = DefaultHasher::new();
            OriginWithRawPotentialPair::new(
                PotentialOrigin::EngineGenerator(1),
                ElectricPotential::new::<volt>(115.),
            )
            .hash(&mut first_item_hasher);

            let mut second_item_hasher = DefaultHasher::new();
            OriginWithRawPotentialPair::new(
                PotentialOrigin::EngineGenerator(1),
                ElectricPotential::new::<volt>(40.),
            )
            .hash(&mut second_item_hasher);

            assert_eq!(first_item_hasher.finish(), second_item_hasher.finish());
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
        fn or_powered_by_both_batteries_results_in_both() {
            let potential = ElectricPotential::new::<volt>(28.);
            let bat_1 =
                BatteryStub::new(Potential::single(PotentialOrigin::Battery(10), potential));
            let bat_2 =
                BatteryStub::new(Potential::single(PotentialOrigin::Battery(11), potential));

            let mut bus = electrical_bus();

            let mut contactor_1 = Contactor::new("BAT1");
            contactor_1.powered_by(&bat_1);
            contactor_1.close_when(true);

            let mut contactor_2 = Contactor::new("BAT2");
            contactor_2.powered_by(&bat_2);
            contactor_2.close_when(true);

            bus.or_powered_by_both_batteries(&contactor_1, &contactor_2);

            assert!(bus
                .input_potential()
                .is_pair(PotentialOrigin::Battery(10), PotentialOrigin::Battery(11)));
        }

        #[test]
        fn or_powered_by_both_batteries_results_in_both_irregardless_of_voltage() {
            let bat_1 = BatteryStub::new(Potential::single(
                PotentialOrigin::Battery(10),
                ElectricPotential::new::<volt>(28.),
            ));
            let bat_2 = BatteryStub::new(Potential::single(
                PotentialOrigin::Battery(11),
                ElectricPotential::new::<volt>(25.),
            ));

            let mut bus = electrical_bus();
            execute_or_powered_by_both_batteries(&mut bus, bat_1, bat_2);

            assert!(bus
                .input_potential()
                .is_pair(PotentialOrigin::Battery(10), PotentialOrigin::Battery(11)));
        }

        #[test]
        fn or_powered_by_battery_1_results_in_bat_1_output() {
            let bat_1 = BatteryStub::new(Potential::single(
                PotentialOrigin::Battery(10),
                ElectricPotential::new::<volt>(28.),
            ));
            let bat_2 = BatteryStub::new(Potential::none());

            let mut bus = electrical_bus();
            execute_or_powered_by_both_batteries(&mut bus, bat_1, bat_2);

            assert!(bus
                .input_potential()
                .is_single(PotentialOrigin::Battery(10)));
        }

        #[test]
        fn or_powered_by_battery_2_results_in_bat_2_output() {
            let bat_1 = BatteryStub::new(Potential::none());
            let bat_2 = BatteryStub::new(Potential::single(
                PotentialOrigin::Battery(11),
                ElectricPotential::new::<volt>(28.),
            ));

            let mut bus = electrical_bus();
            execute_or_powered_by_both_batteries(&mut bus, bat_1, bat_2);

            assert!(bus
                .input_potential()
                .is_single(PotentialOrigin::Battery(11)));
        }

        #[test]
        fn or_powered_by_none_results_in_unpowered_output() {
            let bat_1 = BatteryStub::new(Potential::none());
            let bat_2 = BatteryStub::new(Potential::none());

            let mut bus = electrical_bus();
            execute_or_powered_by_both_batteries(&mut bus, bat_1, bat_2);

            assert!(bus.input_potential().is_unpowered());
        }

        fn execute_or_powered_by_both_batteries(
            bus: &mut ElectricalBus,
            bat_1: BatteryStub,
            bat_2: BatteryStub,
        ) {
            let mut contactor_1 = Contactor::new("BAT1");
            contactor_1.powered_by(&bat_1);
            contactor_1.close_when(true);

            let mut contactor_2 = Contactor::new("BAT2");
            contactor_2.powered_by(&bat_2);
            contactor_2.close_when(true);

            bus.or_powered_by_both_batteries(&contactor_1, &contactor_2);
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
