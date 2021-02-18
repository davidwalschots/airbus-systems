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

use std::collections::HashMap;

use super::{ElectricalBus, ElectricalBusType, ElectricalSystem, Potential, PotentialSource};
use crate::simulation::{SimulationElement, SimulationElementVisitor, UpdateContext};
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
    #[cfg(test)]
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

    #[cfg(test)]
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
