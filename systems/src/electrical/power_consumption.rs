use std::collections::HashMap;

use super::{ElectricPowerSource, ElectricalBus, ElectricalBusType};
use crate::simulator::{SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor};
use uom::si::{f64::*, power::watt};

pub struct SupplyPowerVisitor<'a> {
    supply: &'a PowerSupply,
}
impl<'a> SupplyPowerVisitor<'a> {
    pub fn new(supply: &'a PowerSupply) -> Self {
        SupplyPowerVisitor { supply }
    }
}
impl<'a> SimulatorElementVisitor for SupplyPowerVisitor<'a> {
    fn visit(&mut self, visited: &mut Box<&mut dyn SimulatorElement>) {
        visited.supply_power(&self.supply);
    }
}

pub struct DeterminePowerConsumptionVisitor<'a, 'b> {
    state: &'a mut PowerConsumptionState<'b>,
}
impl<'a, 'b> DeterminePowerConsumptionVisitor<'a, 'b> {
    pub fn new(state: &'a mut PowerConsumptionState<'b>) -> Self {
        DeterminePowerConsumptionVisitor { state }
    }
}
impl<'a, 'b> SimulatorElementVisitor for DeterminePowerConsumptionVisitor<'a, 'b> {
    fn visit(&mut self, visited: &mut Box<&mut dyn SimulatorElement>) {
        visited.determine_power_consumption(&mut self.state);
    }
}

pub struct WritePowerConsumptionVisitor<'a> {
    state: &'a PowerConsumptionState<'a>,
}
impl<'a> WritePowerConsumptionVisitor<'a> {
    pub fn new(state: &'a PowerConsumptionState) -> Self {
        WritePowerConsumptionVisitor { state }
    }
}
impl<'a> SimulatorElementVisitor for WritePowerConsumptionVisitor<'a> {
    fn visit(&mut self, visited: &mut Box<&mut dyn SimulatorElement>) {
        visited.write_power_consumption(&self.state);
    }
}

pub struct PowerConsumptionState<'a> {
    supply: &'a PowerSupply,
    consumption: HashMap<ElectricPowerSource, Power>,
}
impl<'a> PowerConsumptionState<'a> {
    pub fn new(supply: &'a PowerSupply) -> Self {
        PowerConsumptionState {
            supply,
            consumption: HashMap::new(),
        }
    }

    pub fn add(&mut self, bus_type: &ElectricalBusType, power: Power) {
        match self.supply.get_source_for(bus_type) {
            Some(source) => {
                let existing_power = match self.consumption.get(source) {
                    Some(power) => *power,
                    None => Power::new::<watt>(0.),
                };

                self.consumption.insert(*source, existing_power + power);
            }
            None => {}
        };
    }

    pub fn get_total_consumption_for(&self, source: &ElectricPowerSource) -> Power {
        match self.consumption.get(source) {
            Some(power) => *power,
            None => Power::new::<watt>(0.),
        }
    }
}

#[derive(Debug)]
pub struct PowerSupply {
    state: HashMap<ElectricalBusType, Option<ElectricPowerSource>>,
}
impl PowerSupply {
    pub fn new() -> PowerSupply {
        PowerSupply {
            state: HashMap::new(),
        }
    }

    pub fn add(&mut self, bus: &ElectricalBus) {
        self.state.insert(bus.get_type(), bus.get_power_source());
    }

    pub fn is_powered(&self, bus_type: &ElectricalBusType) -> bool {
        match self.state.get(bus_type) {
            Some(source) => source.is_some(),
            None => false,
        }
    }

    pub fn get_source_for(&self, bus_type: &ElectricalBusType) -> &Option<ElectricPowerSource> {
        match self.state.get(bus_type) {
            Some(source) => source,
            None => &None,
        }
    }
}

pub trait ElectricalBusStateFactory {
    fn create_power_supply(&self) -> PowerSupply;
}

#[derive(Debug)]
pub struct PowerConsumption {
    is_powered_by: Option<ElectricalBusType>,
    power_demand: Power,
    powered_by: Vec<ElectricalBusType>,
}
impl PowerConsumption {
    pub fn from_single(bus_type: ElectricalBusType) -> Self {
        PowerConsumption {
            is_powered_by: None,
            power_demand: Power::new::<watt>(0.),
            powered_by: vec![bus_type],
        }
    }

    pub fn is_powered(&self) -> bool {
        self.is_powered_by.is_some()
    }

    pub fn demand(&mut self, power: Power) {
        self.power_demand = power;
    }

    fn try_powering(&mut self, supply: &PowerSupply) -> Option<(&ElectricalBusType, Power)> {
        let first_powered_bus_type = self.powered_by.iter().find(|bus| supply.is_powered(bus));

        self.is_powered_by = match first_powered_bus_type {
            Some(bus_type) => Some(*bus_type),
            None => None,
        };

        match first_powered_bus_type {
            Some(bus_type) => Some((bus_type, self.power_demand)),
            None => None,
        }
    }

    fn get_demand(&self) -> Option<(ElectricalBusType, Power)> {
        match self.is_powered_by {
            Some(bus_type) => Some((bus_type, self.power_demand)),
            None => None,
        }
    }
}
impl SimulatorElementVisitable for PowerConsumption {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for PowerConsumption {
    fn supply_power(&mut self, supply: &PowerSupply) {
        self.try_powering(supply);
    }

    fn determine_power_consumption(&mut self, state: &mut PowerConsumptionState) {
        match self.get_demand() {
            Some((bus_type, power)) => state.add(&bus_type, power),
            None => {}
        }
    }
}

pub struct PowerConsumptionHandler<'a> {
    supply: &'a PowerSupply,
    power_consumption_state: PowerConsumptionState<'a>,
}
impl<'a> PowerConsumptionHandler<'a> {
    pub fn new(supply: &'a PowerSupply) -> Self {
        PowerConsumptionHandler {
            supply: supply,
            power_consumption_state: PowerConsumptionState::new(&supply),
        }
    }

    pub fn supply_power_to_elements(
        &self,
        visitable: &mut Box<&mut dyn SimulatorElementVisitable>,
    ) {
        let mut visitor = SupplyPowerVisitor::new(&self.supply);
        visitable.accept(&mut Box::new(&mut visitor));
    }

    pub fn determine_power_consumption(
        &mut self,
        visitable: &mut Box<&mut dyn SimulatorElementVisitable>,
    ) {
        let mut visitor = DeterminePowerConsumptionVisitor::new(&mut self.power_consumption_state);
        visitable.accept(&mut Box::new(&mut visitor));
    }

    pub fn write_power_consumption(
        &mut self,
        visitable: &mut Box<&mut dyn SimulatorElementVisitable>,
    ) {
        let mut visitor = WritePowerConsumptionVisitor::new(&self.power_consumption_state);
        visitable.accept(&mut Box::new(&mut visitor));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::electrical::{Current, ElectricSource};

    struct ApuStub {
        used_power: Power,
    }
    impl ApuStub {
        fn new() -> Self {
            ApuStub {
                used_power: Power::new::<watt>(0.),
            }
        }
    }
    impl ElectricSource for ApuStub {
        fn output(&self) -> Current {
            Current::some(ElectricPowerSource::ApuGenerator)
        }
    }
    impl SimulatorElementVisitable for ApuStub {
        fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
            visitor.visit(&mut Box::new(self));
        }
    }
    impl SimulatorElement for ApuStub {
        fn write_power_consumption(&mut self, state: &PowerConsumptionState) {
            self.used_power = state.get_total_consumption_for(&ElectricPowerSource::ApuGenerator);
        }
    }

    #[cfg(test)]
    mod power_supply_tests {
        use crate::electrical::Powerable;

        use super::*;

        fn power_supply() -> PowerSupply {
            PowerSupply::new()
        }

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
            let supply = power_supply();
            assert!(!supply.is_powered(&ElectricalBusType::AlternatingCurrent(1)))
        }

        #[test]
        fn is_powered_returns_true_when_bus_is_powered() {
            let mut supply = power_supply();
            supply.add(&powered_bus(ElectricalBusType::AlternatingCurrent(1)));

            assert!(supply.is_powered(&ElectricalBusType::AlternatingCurrent(1)))
        }

        #[test]
        fn is_powered_returns_false_when_bus_unpowered() {
            let mut supply = power_supply();
            supply.add(&unpowered_bus(ElectricalBusType::AlternatingCurrent(1)));

            assert!(!supply.is_powered(&ElectricalBusType::AlternatingCurrent(1)))
        }
    }

    #[cfg(test)]
    mod power_consumption_tests {
        use super::*;
        use crate::electrical::Powerable;

        fn powered_bus(bus_type: ElectricalBusType) -> ElectricalBus {
            let mut bus = ElectricalBus::new(bus_type);
            bus.powered_by(&ApuStub::new());

            bus
        }

        fn power_supply() -> PowerSupply {
            PowerSupply::new()
        }

        fn power_consumption() -> PowerConsumption {
            PowerConsumption::from_single(ElectricalBusType::AlternatingCurrent(1))
        }

        fn powered_power_consumption() -> PowerConsumption {
            let mut consumption = power_consumption();
            let mut supply = power_supply();
            supply.add(&powered_bus(ElectricalBusType::AlternatingCurrent(1)));
            consumption.try_powering(&supply);

            consumption
        }

        #[test]
        fn is_powered_returns_false_when_not_powered() {
            let consumption = power_consumption();
            assert!(!consumption.is_powered());
        }

        #[test]
        fn is_powered_returns_false_when_powered_by_bus_is_not_powered() {
            let mut supply = power_supply();
            supply.add(&powered_bus(ElectricalBusType::AlternatingCurrent(2)));

            let mut consumption = power_consumption();
            consumption.try_powering(&supply);

            assert!(!consumption.is_powered());
        }

        #[test]
        fn is_powered_returns_true_when_powered_by_bus_is_powered() {
            let mut supply = power_supply();
            supply.add(&powered_bus(ElectricalBusType::AlternatingCurrent(2)));
            supply.add(&powered_bus(ElectricalBusType::AlternatingCurrent(1)));

            let mut consumption = power_consumption();
            consumption.try_powering(&supply);

            assert!(consumption.is_powered());
        }

        #[test]
        fn get_demand_returns_demand_demanded_by_demand() {
            let mut consumption = powered_power_consumption();
            consumption.demand(Power::new::<watt>(100.));

            let demand = consumption.get_demand().unwrap();
            assert_eq!(demand.0, ElectricalBusType::AlternatingCurrent(1));
            assert!((demand.1.get::<watt>() - 100.).abs() < f64::EPSILON);
        }

        #[test]
        fn get_demand_returns_no_demand_when_no_supply() {
            let mut consumption = power_consumption();
            consumption.demand(Power::new::<watt>(100.));

            let demand = consumption.get_demand();
            assert!(demand.is_none());
        }
    }

    #[cfg(test)]
    mod power_consumption_handler_tests {
        use crate::electrical::Powerable;

        use super::*;

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
                        PowerConsumption::from_single(ElectricalBusType::AlternatingCurrent(2)),
                        Power::new::<watt>(500.),
                    ),
                    light: PowerConsumerStub::new(
                        PowerConsumption::from_single(ElectricalBusType::AlternatingCurrent(1)),
                        Power::new::<watt>(1000.),
                    ),
                    screen: PowerConsumerStub::new(
                        PowerConsumption::from_single(ElectricalBusType::AlternatingCurrent(2)),
                        Power::new::<watt>(100.),
                    ),
                    apu: ApuStub::new(),
                }
            }
        }
        impl SimulatorElementVisitable for AircraftStub {
            fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
                self.door.accept(visitor);
                self.light.accept(visitor);
                self.screen.accept(visitor);
                self.apu.accept(visitor);
                visitor.visit(&mut Box::new(self));
            }
        }
        impl SimulatorElement for AircraftStub {}
        impl ElectricalBusStateFactory for AircraftStub {
            fn create_power_supply(&self) -> PowerSupply {
                let mut supply = PowerSupply::new();

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
            power_consumption: PowerConsumption,
        }
        impl PowerConsumerStub {
            fn new(mut power_consumption: PowerConsumption, power: Power) -> Self {
                power_consumption.demand(power);
                PowerConsumerStub {
                    power_consumption: power_consumption,
                }
            }
        }
        impl SimulatorElementVisitable for PowerConsumerStub {
            fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
                self.power_consumption.accept(visitor);
                visitor.visit(&mut Box::new(self));
            }
        }
        impl SimulatorElement for PowerConsumerStub {}

        #[test]
        fn used_power_is_correctly_calculated() {
            let mut aircraft = AircraftStub::new();
            let supply = aircraft.create_power_supply();

            let mut handler = PowerConsumptionHandler::new(&supply);

            handler.supply_power_to_elements(&mut Box::new(&mut aircraft));
            handler.determine_power_consumption(&mut Box::new(&mut aircraft));
            handler.write_power_consumption(&mut Box::new(&mut aircraft));

            assert!((aircraft.apu.used_power.get::<watt>() - 600.).abs() < f64::EPSILON);
        }
    }
}
