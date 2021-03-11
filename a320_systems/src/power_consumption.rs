use systems::{
    electrical::{
        consumption::{FlightPhasePowerConsumer, PowerConsumerFlightPhase},
        ElectricalBusType,
    },
    simulation::{SimulationElement, SimulationElementVisitor, UpdateContext},
};
use uom::si::{f64::*, power::watt};

/// This type provides an aggregated form of power consumption.
/// We haven't yet implemented all power consumers and thus need something to
/// consume power, as otherwise electrical load is nearly 0.
pub(super) struct A320PowerConsumption {
    ac_bus_1_consumer: FlightPhasePowerConsumer,
    ac_bus_2_consumer: FlightPhasePowerConsumer,
    ac_ess_bus_consumer: FlightPhasePowerConsumer,
    ac_ess_shed_bus_consumer: FlightPhasePowerConsumer,
    ac_stat_inv_bus_consumer: FlightPhasePowerConsumer,
    dc_bus_1_consumer: FlightPhasePowerConsumer,
    dc_bus_2_consumer: FlightPhasePowerConsumer,
    dc_ess_bus_consumer: FlightPhasePowerConsumer,
    dc_ess_shed_bus_consumer: FlightPhasePowerConsumer,
    dc_bat_bus_consumer: FlightPhasePowerConsumer,
    dc_hot_bus_1_consumer: FlightPhasePowerConsumer,
    dc_hot_bus_2_consumer: FlightPhasePowerConsumer,
}
impl A320PowerConsumption {
    pub fn new() -> Self {
        // The watts in this function are all provided by komp.
        // They include a 0.8 power factor correction.
        Self {
            ac_bus_1_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::AlternatingCurrent(1),
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(26816.3),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(30350.1),
                ),
                (
                    PowerConsumerFlightPhase::Takeoff,
                    Power::new::<watt>(33797.3),
                ),
                (
                    PowerConsumerFlightPhase::Flight,
                    Power::new::<watt>(39032.5),
                ),
                (
                    PowerConsumerFlightPhase::Landing,
                    Power::new::<watt>(30733.3),
                ),
                (
                    PowerConsumerFlightPhase::TaxiIn,
                    Power::new::<watt>(30243.1),
                ),
            ]),
            ac_bus_2_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::AlternatingCurrent(2),
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(31678.2),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(25398.8),
                ),
                (
                    PowerConsumerFlightPhase::Takeoff,
                    Power::new::<watt>(27811.),
                ),
                (
                    PowerConsumerFlightPhase::Flight,
                    Power::new::<watt>(32161.9),
                ),
                (
                    PowerConsumerFlightPhase::Landing,
                    Power::new::<watt>(25782.),
                ),
                (
                    PowerConsumerFlightPhase::TaxiIn,
                    Power::new::<watt>(28138.8),
                ),
            ]),
            ac_ess_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::AlternatingCurrentEssential,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(455.7),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(715.7),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(875.7)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(875.7)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(715.7)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(715.7)),
            ]),
            ac_ess_shed_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::AlternatingCurrentEssentialShed,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(560.5),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(823.5),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(823.5)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(823.5)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(823.5)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(823.5)),
            ]),
            ac_stat_inv_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::AlternatingCurrentStaticInverter,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(135.),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(135.),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(135.)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(135.)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(135.)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(135.)),
            ]),
            dc_bus_1_consumer: FlightPhasePowerConsumer::from(ElectricalBusType::DirectCurrent(1))
                .demand([
                    (
                        PowerConsumerFlightPhase::BeforeStart,
                        Power::new::<watt>(252.),
                    ),
                    (
                        PowerConsumerFlightPhase::AfterStart,
                        Power::new::<watt>(308.),
                    ),
                    (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(364.)),
                    (PowerConsumerFlightPhase::Flight, Power::new::<watt>(280.)),
                    (PowerConsumerFlightPhase::Landing, Power::new::<watt>(364.)),
                    (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(336.)),
                ]),
            dc_bus_2_consumer: FlightPhasePowerConsumer::from(ElectricalBusType::DirectCurrent(2))
                .demand([
                    (
                        PowerConsumerFlightPhase::BeforeStart,
                        Power::new::<watt>(532.),
                    ),
                    (
                        PowerConsumerFlightPhase::AfterStart,
                        Power::new::<watt>(448.),
                    ),
                    (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(392.)),
                    (PowerConsumerFlightPhase::Flight, Power::new::<watt>(392.)),
                    (PowerConsumerFlightPhase::Landing, Power::new::<watt>(392.)),
                    (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(448.)),
                ]),
            dc_ess_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentEssential,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(168.),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(140.),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(168.)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(140.)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(168.)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(140.)),
            ]),
            dc_ess_shed_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentEssentialShed,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(224.),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(168.),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(196.)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(196.)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(196.)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(168.)),
            ]),
            dc_bat_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentBattery,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(0.),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(28.),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(28.)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(28.)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(28.)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(28.)),
            ]),
            dc_hot_bus_1_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentHot(1),
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(108.),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(11.),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(15.3)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(15.3)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(15.3)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(11.)),
            ]),
            dc_hot_bus_2_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentHot(2),
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(24.3),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(24.3),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(24.3)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(24.3)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(24.3)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(24.3)),
            ]),
        }
    }

    pub fn update(&mut self, context: &UpdateContext) {
        self.ac_bus_1_consumer.update(context);
        self.ac_bus_2_consumer.update(context);
        self.ac_ess_bus_consumer.update(context);
        self.ac_ess_shed_bus_consumer.update(context);
        self.ac_stat_inv_bus_consumer.update(context);
        self.dc_bus_1_consumer.update(context);
        self.dc_bus_2_consumer.update(context);
        self.dc_ess_bus_consumer.update(context);
        self.dc_ess_shed_bus_consumer.update(context);
        self.dc_bat_bus_consumer.update(context);
        self.dc_hot_bus_1_consumer.update(context);
        self.dc_hot_bus_2_consumer.update(context);
    }
}
impl SimulationElement for A320PowerConsumption {
    fn accept<T: SimulationElementVisitor>(&mut self, visitor: &mut T) {
        self.ac_bus_1_consumer.accept(visitor);
        self.ac_bus_2_consumer.accept(visitor);
        self.ac_ess_bus_consumer.accept(visitor);
        self.ac_ess_shed_bus_consumer.accept(visitor);
        self.ac_stat_inv_bus_consumer.accept(visitor);
        self.dc_bus_1_consumer.accept(visitor);
        self.dc_bus_2_consumer.accept(visitor);
        self.dc_ess_bus_consumer.accept(visitor);
        self.dc_ess_shed_bus_consumer.accept(visitor);
        self.dc_bat_bus_consumer.accept(visitor);
        self.dc_hot_bus_1_consumer.accept(visitor);
        self.dc_hot_bus_2_consumer.accept(visitor);

        visitor.visit(self);
    }
}
impl Default for A320PowerConsumption {
    fn default() -> Self {
        A320PowerConsumption::new()
    }
}
