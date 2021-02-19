//! Power consumption works as follows:
//! 1. The electrical system determines which electrical buses are powered
//!    by which electric potential origin (generators, external power,
//!    transformer rectifiers, etc).
//! 2. Thereafter, power consumers can ask the electrical system if the buses they receive power
//!    from are powered, and which origin supplies them.
//! 3. A power consumer declares which bus(es) it receives power from in order of priority.
//!    When a power consumer consumes from a bus which has potential, it is considered powered.
//!    Systems can use this information to determine if elements within the system
//!    can perform their work and how much power they consume in doing so.
//! 4. After systems finished their state update. Each power consumer is then asked how much
//!    power they consume from which origin. This is summed to get the total consumption per origin.
//! 5. The total load is passed to the various origins so that they can calculate their
//!    load %, voltage, frequency and current.

use std::{collections::HashMap, time::Duration};

use super::{ElectricalBus, ElectricalBusType, ElectricalSystem, Potential, PotentialSource};
use crate::{
    shared::{random_number, FwcFlightPhase},
    simulation::{SimulationElement, SimulationElementVisitor, SimulatorReader, UpdateContext},
};
use num_traits::FromPrimitive;
use uom::si::{f64::*, power::watt};

pub struct ElectricPower {
    supplied_power: SuppliedPower,
    power_consumption: PowerConsumption,
}
impl ElectricPower {
    pub fn from<T: ElectricalSystem>(electrical_system: &T) -> Self {
        ElectricPower {
            supplied_power: electrical_system.get_supplied_power(),
            power_consumption: PowerConsumption::new(),
        }
    }

    pub fn supply_to<T: SimulationElement>(&self, element: &mut T) {
        let mut visitor = ReceivePowerVisitor::new(&self.supplied_power);
        element.accept(&mut visitor);
    }

    pub fn consume_in<T: SimulationElement>(&mut self, element: &mut T) {
        let mut visitor = ConsumePowerVisitor::new(&mut self.power_consumption);
        element.accept(&mut visitor);
    }

    pub fn report_consumption_to<T: SimulationElement>(
        &mut self,
        element: &mut T,
        context: &UpdateContext,
    ) {
        let mut visitor = ReportPowerConsumptionVisitor::new(&self.power_consumption, context);
        element.accept(&mut visitor);
    }
}

pub struct SuppliedPower {
    state: HashMap<ElectricalBusType, Potential>,
}
impl SuppliedPower {
    pub fn new() -> SuppliedPower {
        SuppliedPower {
            state: HashMap::new(),
        }
    }

    pub fn add(&mut self, bus: &ElectricalBus) {
        self.state.insert(bus.bus_type(), bus.output_potential());
    }

    pub fn potential_of(&self, bus_type: &ElectricalBusType) -> Potential {
        match self.state.get(bus_type) {
            Some(potential) => *potential,
            None => Potential::None,
        }
    }

    pub fn is_powered(&self, bus_type: &ElectricalBusType) -> bool {
        self.potential_of(bus_type).is_powered()
    }

    pub fn source_for(&self, bus_type: &ElectricalBusType) -> Potential {
        match self.state.get(bus_type) {
            Some(source) => *source,
            None => Potential::None,
        }
    }
}
impl Default for SuppliedPower {
    fn default() -> Self {
        Self::new()
    }
}

/// A generic consumer of power.
pub struct PowerConsumer {
    provided_potential: Potential,
    demand: Power,
    powered_by: Vec<ElectricalBusType>,
}
impl PowerConsumer {
    /// Create a power consumer which consumes power from the given bus type.
    pub fn from(bus_type: ElectricalBusType) -> Self {
        PowerConsumer {
            provided_potential: Default::default(),
            demand: Power::new::<watt>(0.),
            powered_by: vec![bus_type],
        }
    }

    #[cfg(test)]
    /// Determine if the power consumer has potential powering
    /// it during this simulation tick.
    /// If this function is called before power has been supplied to it
    /// during this tick, the result of this function will be last frame's state.
    pub fn is_powered(&self) -> bool {
        self.provided_potential.is_powered()
    }

    /// Set the amount of power that is demanded by the consumer when powered.
    pub fn demand(&mut self, power: Power) {
        self.demand = power;
    }
}
impl SimulationElement for PowerConsumer {
    fn receive_power(&mut self, supplied_power: &SuppliedPower) {
        self.provided_potential = self
            .powered_by
            .iter()
            .find_map(|bus_type| {
                let potential = supplied_power.potential_of(bus_type);
                if potential.is_powered() {
                    Some(potential)
                } else {
                    None
                }
            })
            .unwrap_or_default();
    }

    fn consume_power(&mut self, state: &mut PowerConsumption) {
        state.add(&self.provided_potential, self.demand);
    }
}

/// A special type of power consumer which changes its consumption
/// based on the phase of the flight.
pub struct FlightPhasePowerConsumer {
    consumer: PowerConsumer,
    base_demand: [Power; PowerConsumerFlightPhase::TaxiIn as usize + 1],
    current_flight_phase: PowerConsumerFlightPhase,
    update_after: Duration,
}
impl FlightPhasePowerConsumer {
    pub fn from(bus_type: ElectricalBusType) -> Self {
        Self {
            consumer: PowerConsumer::from(bus_type),
            base_demand: Default::default(),
            current_flight_phase: PowerConsumerFlightPhase::BeforeStart,
            update_after: Duration::from_secs(0),
        }
    }

    pub fn update(&mut self, context: &UpdateContext) {
        if self.update_after <= context.delta {
            self.update_after = Duration::from_secs_f64(5. + ((random_number() % 26) as f64));
            let base_demand = self.base_demand[self.current_flight_phase as usize].get::<watt>();
            self.consumer.demand(Power::new::<watt>(
                base_demand * ((90. + ((random_number() % 21) as f64)) / 100.),
            ));
        } else {
            self.update_after -= context.delta;
        }
    }

    pub fn demand(
        mut self,
        demand: [(PowerConsumerFlightPhase, Power); PowerConsumerFlightPhase::TaxiIn as usize + 1],
    ) -> Self {
        for (phase, power) in &demand {
            self.base_demand[*phase as usize] = *power;
        }

        self
    }
}
impl SimulationElement for FlightPhasePowerConsumer {
    fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
        self.consumer.accept(visitor);

        visitor.visit(self);
    }

    fn read(&mut self, reader: &mut SimulatorReader) {
        let flight_phase: Option<FwcFlightPhase> =
            FromPrimitive::from_f64(reader.read_f64("A32NX_FWC_FLIGHT_PHASE"));
        if let Some(phase) = flight_phase {
            self.current_flight_phase = PowerConsumerFlightPhase::from(phase);
        }
    }
}

#[derive(Copy, Clone)]
pub enum PowerConsumerFlightPhase {
    BeforeStart = 0,
    AfterStart = 1,
    Takeoff = 2,
    Flight = 3,
    Landing = 4,
    TaxiIn = 5,
}
impl From<FwcFlightPhase> for PowerConsumerFlightPhase {
    fn from(phase: FwcFlightPhase) -> Self {
        match phase {
            FwcFlightPhase::ElecPwr => PowerConsumerFlightPhase::BeforeStart,
            FwcFlightPhase::FirstEngineStarted => PowerConsumerFlightPhase::AfterStart,
            FwcFlightPhase::FirstEngineTakeOffPower => PowerConsumerFlightPhase::Takeoff,
            FwcFlightPhase::AtOrAboveEightyKnots => PowerConsumerFlightPhase::Takeoff,
            FwcFlightPhase::LiftOff => PowerConsumerFlightPhase::Takeoff,
            FwcFlightPhase::AtOrAbove1500Feet => PowerConsumerFlightPhase::Flight,
            FwcFlightPhase::AtOrBelow800Feet => PowerConsumerFlightPhase::Landing,
            FwcFlightPhase::TouchDown => PowerConsumerFlightPhase::Landing,
            FwcFlightPhase::AtOrBelowEightyKnots => PowerConsumerFlightPhase::TaxiIn,
            FwcFlightPhase::EnginesShutdown => PowerConsumerFlightPhase::BeforeStart,
        }
    }
}

pub trait PowerConsumptionReport {
    fn total_consumption_of(&self, potential: &Potential) -> Power;
}

pub struct PowerConsumption {
    consumption: HashMap<Potential, Power>,
}
impl PowerConsumption {
    pub fn new() -> Self {
        PowerConsumption {
            consumption: HashMap::new(),
        }
    }

    pub fn add(&mut self, potential: &Potential, power: Power) {
        match potential {
            Potential::None => {}
            potential => {
                let x = self.consumption.entry(*potential).or_default();
                *x += power;
            }
        };
    }
}
impl PowerConsumptionReport for PowerConsumption {
    fn total_consumption_of(&self, potential: &Potential) -> Power {
        match self.consumption.get(potential) {
            Some(power) => *power,
            None => Power::new::<watt>(0.),
        }
    }
}
impl Default for PowerConsumption {
    fn default() -> Self {
        Self::new()
    }
}

struct ReceivePowerVisitor<'a> {
    supplied_power: &'a SuppliedPower,
}
impl<'a> ReceivePowerVisitor<'a> {
    pub fn new(supplied_power: &'a SuppliedPower) -> Self {
        ReceivePowerVisitor { supplied_power }
    }
}
impl<'a> SimulationElementVisitor for ReceivePowerVisitor<'a> {
    fn visit<T: SimulationElement>(&mut self, visited: &mut T) {
        visited.receive_power(&self.supplied_power);
    }
}

struct ConsumePowerVisitor<'a> {
    consumption: &'a mut PowerConsumption,
}
impl<'a> ConsumePowerVisitor<'a> {
    pub fn new(consumption: &'a mut PowerConsumption) -> Self {
        ConsumePowerVisitor { consumption }
    }
}
impl<'a> SimulationElementVisitor for ConsumePowerVisitor<'a> {
    fn visit<T: SimulationElement>(&mut self, visited: &mut T) {
        visited.consume_power(&mut self.consumption);
    }
}

struct ReportPowerConsumptionVisitor<'a> {
    consumption: &'a PowerConsumption,
    context: &'a UpdateContext,
}
impl<'a> ReportPowerConsumptionVisitor<'a> {
    pub fn new(consumption: &'a PowerConsumption, context: &'a UpdateContext) -> Self {
        ReportPowerConsumptionVisitor {
            consumption,
            context,
        }
    }
}
impl<'a> SimulationElementVisitor for ReportPowerConsumptionVisitor<'a> {
    fn visit<T: SimulationElement>(&mut self, visited: &mut T) {
        visited.process_power_consumption_report(self.consumption, &self.context);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::electrical::{Potential, PotentialSource};

    struct ApuStub {
        consumed_power: Power,
    }
    impl ApuStub {
        fn new() -> Self {
            ApuStub {
                consumed_power: Power::new::<watt>(0.),
            }
        }

        fn consumed_power(&self) -> Power {
            self.consumed_power
        }
    }
    impl PotentialSource for ApuStub {
        fn output_potential(&self) -> Potential {
            Potential::ApuGenerator(1)
        }
    }
    impl SimulationElement for ApuStub {
        fn process_power_consumption_report<T: PowerConsumptionReport>(
            &mut self,
            report: &T,
            _: &UpdateContext,
        ) {
            self.consumed_power = report.total_consumption_of(&Potential::ApuGenerator(1));
        }
    }

    #[cfg(test)]
    mod supplied_power_tests {
        use super::*;
        use crate::electrical::PotentialTarget;

        fn powered_bus(bus_type: ElectricalBusType) -> ElectricalBus {
            let mut bus = unpowered_bus(bus_type);
            bus.powered_by(&ApuStub::new());

            bus
        }

        fn unpowered_bus(bus_type: ElectricalBusType) -> ElectricalBus {
            ElectricalBus::new(bus_type)
        }

        #[test]
        fn is_powered_returns_false_when_bus_not_found() {
            let supplied_power = SuppliedPower::new();
            assert!(!supplied_power.is_powered(&ElectricalBusType::AlternatingCurrent(1)))
        }

        #[test]
        fn is_powered_returns_true_when_bus_is_powered() {
            let mut supplied_power = SuppliedPower::new();
            supplied_power.add(&powered_bus(ElectricalBusType::AlternatingCurrent(1)));

            assert!(supplied_power.is_powered(&ElectricalBusType::AlternatingCurrent(1)))
        }

        #[test]
        fn is_powered_returns_false_when_bus_unpowered() {
            let mut supplied_power = SuppliedPower::new();
            supplied_power.add(&unpowered_bus(ElectricalBusType::AlternatingCurrent(1)));

            assert!(!supplied_power.is_powered(&ElectricalBusType::AlternatingCurrent(1)))
        }
    }

    #[cfg(test)]
    mod power_consumer_tests {
        use super::*;
        use crate::electrical::PotentialTarget;

        fn powered_bus(bus_type: ElectricalBusType) -> ElectricalBus {
            let mut bus = ElectricalBus::new(bus_type);
            bus.powered_by(&ApuStub::new());

            bus
        }

        fn powered_consumer() -> PowerConsumer {
            let mut consumer = PowerConsumer::from(ElectricalBusType::AlternatingCurrent(1));
            let mut supplied_power = SuppliedPower::new();
            supplied_power.add(&powered_bus(ElectricalBusType::AlternatingCurrent(1)));
            consumer.receive_power(&supplied_power);

            consumer
        }

        #[test]
        fn is_powered_returns_false_when_not_powered() {
            let consumer = PowerConsumer::from(ElectricalBusType::AlternatingCurrent(1));
            assert!(!consumer.is_powered());
        }

        #[test]
        fn is_powered_returns_false_when_powered_by_bus_which_is_not_powered() {
            let mut supplied_power = SuppliedPower::new();
            supplied_power.add(&powered_bus(ElectricalBusType::AlternatingCurrent(2)));

            let mut consumer = PowerConsumer::from(ElectricalBusType::AlternatingCurrent(1));
            consumer.receive_power(&supplied_power);

            assert!(!consumer.is_powered());
        }

        #[test]
        fn is_powered_returns_true_when_powered_by_bus_which_is_powered() {
            let mut supplied_power = SuppliedPower::new();
            supplied_power.add(&powered_bus(ElectricalBusType::AlternatingCurrent(2)));
            supplied_power.add(&powered_bus(ElectricalBusType::AlternatingCurrent(1)));

            let mut consumption = PowerConsumer::from(ElectricalBusType::AlternatingCurrent(1));
            consumption.receive_power(&supplied_power);

            assert!(consumption.is_powered());
        }

        #[test]
        fn consume_power_adds_power_consumption_when_powered() {
            let mut consumption = PowerConsumption::new();
            let mut consumer = powered_consumer();
            let expected = Power::new::<watt>(100.);

            consumer.demand(expected);
            consumer.consume_power(&mut consumption);

            assert_eq!(
                consumption.total_consumption_of(&Potential::ApuGenerator(1)),
                expected
            );
        }

        #[test]
        fn consume_power_does_not_add_power_consumption_when_unpowered() {
            let mut consumption = PowerConsumption::new();
            let mut consumer = PowerConsumer::from(ElectricalBusType::AlternatingCurrent(1));

            consumer.demand(Power::new::<watt>(100.));
            consumer.consume_power(&mut consumption);

            assert_eq!(
                consumption.total_consumption_of(&Potential::ApuGenerator(1)),
                Power::new::<watt>(0.)
            );
        }
    }

    #[cfg(test)]
    mod flight_phase_power_consumer_tests {
        use crate::{
            electrical::PotentialTarget,
            simulation::{
                context, test::TestReaderWriter, SimulatorReaderWriter,
                SimulatorToSimulationVisitor,
            },
        };

        use super::*;

        fn powered_bus(bus_type: ElectricalBusType) -> ElectricalBus {
            let mut bus = ElectricalBus::new(bus_type);
            bus.powered_by(&ApuStub::new());

            bus
        }

        fn powered_consumer() -> FlightPhasePowerConsumer {
            let mut consumer =
                FlightPhasePowerConsumer::from(ElectricalBusType::AlternatingCurrent(1));
            let mut supplied_power = SuppliedPower::new();
            supplied_power.add(&powered_bus(ElectricalBusType::AlternatingCurrent(1)));
            consumer.accept(&mut ReceivePowerVisitor::new(&supplied_power));

            consumer
        }

        fn apply_flight_phase(consumer: &mut FlightPhasePowerConsumer, phase: FwcFlightPhase) {
            let mut test_reader_writer = TestReaderWriter::new();
            test_reader_writer.write("A32NX_FWC_FLIGHT_PHASE", phase as i32 as f64);
            let mut reader = SimulatorReader::new(&mut test_reader_writer);

            consumer.accept(&mut SimulatorToSimulationVisitor::new(&mut reader))
        }

        fn consume_power(consumer: &mut FlightPhasePowerConsumer) -> PowerConsumption {
            let mut power_consumption = PowerConsumption::new();
            consumer.accept(&mut ConsumePowerVisitor::new(&mut power_consumption));

            power_consumption
        }

        #[test]
        fn when_flight_phase_doesnt_have_demand_usage_is_zero() {
            let mut consumer = powered_consumer().demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(0.),
                ),
                (PowerConsumerFlightPhase::AfterStart, Power::new::<watt>(0.)),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(0.)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(0.)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(0.)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(0.)),
            ]);

            apply_flight_phase(&mut consumer, FwcFlightPhase::FirstEngineStarted);
            consumer.update(&context());
            let power_consumption = consume_power(&mut consumer);

            assert_eq!(
                power_consumption.total_consumption_of(&Potential::ApuGenerator(1)),
                Power::new::<watt>(0.)
            );
        }

        #[test]
        fn when_flight_phase_does_have_demand_usage_is_close_to_demand() {
            let input = 20000.;
            let mut consumer = powered_consumer().demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(0.),
                ),
                (PowerConsumerFlightPhase::AfterStart, Power::new::<watt>(0.)),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(0.)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(input)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(0.)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(0.)),
            ]);

            apply_flight_phase(&mut consumer, FwcFlightPhase::AtOrAbove1500Feet);
            consumer.update(&context());
            let power_consumption = consume_power(&mut consumer);

            let consumption = power_consumption
                .total_consumption_of(&Potential::ApuGenerator(1))
                .get::<watt>();
            assert!(consumption >= input * 0.9);
            assert!(consumption <= input * 1.1);
        }

        #[test]
        fn when_flight_phase_does_have_demand_but_consumer_unpowered_usage_is_zero() {
            let mut consumer =
                FlightPhasePowerConsumer::from(ElectricalBusType::AlternatingCurrent(1)).demand([
                    (
                        PowerConsumerFlightPhase::BeforeStart,
                        Power::new::<watt>(20000.),
                    ),
                    (
                        PowerConsumerFlightPhase::AfterStart,
                        Power::new::<watt>(20000.),
                    ),
                    (
                        PowerConsumerFlightPhase::Takeoff,
                        Power::new::<watt>(20000.),
                    ),
                    (PowerConsumerFlightPhase::Flight, Power::new::<watt>(20000.)),
                    (
                        PowerConsumerFlightPhase::Landing,
                        Power::new::<watt>(20000.),
                    ),
                    (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(20000.)),
                ]);

            apply_flight_phase(&mut consumer, FwcFlightPhase::FirstEngineStarted);
            consumer.update(&context());
            let power_consumption = consume_power(&mut consumer);

            assert_eq!(
                power_consumption.total_consumption_of(&Potential::ApuGenerator(1)),
                Power::new::<watt>(0.)
            );
        }
    }

    #[cfg(test)]
    mod power_consumption_tests {
        use super::*;

        fn power_consumption() -> PowerConsumption {
            PowerConsumption::new()
        }

        #[test]
        fn total_consumption_of_returns_zero_when_no_consumption() {
            let consumption = power_consumption();
            let expected = Power::new::<watt>(0.);

            assert_eq!(
                consumption.total_consumption_of(&Potential::ApuGenerator(1)),
                expected
            );
        }

        #[test]
        fn total_consumption_of_returns_the_consumption_of_the_requested_potential() {
            let mut consumption = power_consumption();
            let expected = Power::new::<watt>(600.);

            consumption.add(&Potential::ApuGenerator(1), expected);
            consumption.add(&Potential::EngineGenerator(1), Power::new::<watt>(400.));

            assert_eq!(
                consumption.total_consumption_of(&Potential::ApuGenerator(1)),
                expected
            );
        }

        #[test]
        fn total_consumption_of_returns_the_sum_of_consumption_of_the_requested_potential() {
            let mut consumption = power_consumption();
            let expected = Power::new::<watt>(1100.);

            consumption.add(&Potential::ApuGenerator(1), Power::new::<watt>(400.));
            consumption.add(&Potential::ApuGenerator(1), Power::new::<watt>(700.));

            assert_eq!(
                consumption.total_consumption_of(&Potential::ApuGenerator(1)),
                expected
            );
        }
    }

    #[cfg(test)]
    mod power_consumption_handler_tests {
        use super::*;
        use crate::electrical::{ElectricalSystem, PotentialTarget};
        use crate::simulation::context;

        struct AircraftStub {
            door: PowerConsumerStub,
            light: PowerConsumerStub,
            screen: PowerConsumerStub,
            apu: ApuStub,
        }
        impl AircraftStub {
            fn new() -> Self {
                AircraftStub {
                    door: PowerConsumerStub::new(
                        PowerConsumer::from(ElectricalBusType::AlternatingCurrent(2)),
                        Power::new::<watt>(500.),
                    ),
                    light: PowerConsumerStub::new(
                        PowerConsumer::from(ElectricalBusType::AlternatingCurrent(1)),
                        Power::new::<watt>(1000.),
                    ),
                    screen: PowerConsumerStub::new(
                        PowerConsumer::from(ElectricalBusType::AlternatingCurrent(2)),
                        Power::new::<watt>(100.),
                    ),
                    apu: ApuStub::new(),
                }
            }

            fn apu_consumed_power(&self) -> Power {
                self.apu.consumed_power()
            }
        }
        impl SimulationElement for AircraftStub {
            fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
                self.door.accept(visitor);
                self.light.accept(visitor);
                self.screen.accept(visitor);
                self.apu.accept(visitor);
                visitor.visit(self);
            }
        }
        impl ElectricalSystem for AircraftStub {
            fn get_supplied_power(&self) -> SuppliedPower {
                let mut supply = SuppliedPower::new();

                let mut powered = ElectricalBus::new(ElectricalBusType::AlternatingCurrent(2));
                powered.powered_by(&ApuStub::new());

                supply.add(&powered);
                supply.add(&ElectricalBus::new(ElectricalBusType::AlternatingCurrent(
                    1,
                )));

                supply
            }
        }

        struct PowerConsumerStub {
            power_consumption: PowerConsumer,
        }
        impl PowerConsumerStub {
            fn new(mut power_consumption: PowerConsumer, power: Power) -> Self {
                power_consumption.demand(power);
                PowerConsumerStub { power_consumption }
            }
        }
        impl SimulationElement for PowerConsumerStub {
            fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
                self.power_consumption.accept(visitor);
                visitor.visit(self);
            }
        }

        #[test]
        fn reported_consumption_is_correct() {
            let mut aircraft = AircraftStub::new();
            let mut electric_power = ElectricPower::from(&aircraft);

            electric_power.supply_to(&mut aircraft);
            electric_power.consume_in(&mut aircraft);
            electric_power.report_consumption_to(&mut aircraft, &context());

            assert_eq!(aircraft.apu_consumed_power(), Power::new::<watt>(600.));
        }
    }
}
