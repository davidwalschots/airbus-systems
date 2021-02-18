use systems::{
    electrical::{ElectricalBusType, FlightPhasePowerConsumer, PowerConsumerFlightPhase},
    simulation::{SimulationElement, SimulationElementVisitor},
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
                    Power::new::<watt>(21453.04),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(24280.08),
                ),
                (
                    PowerConsumerFlightPhase::Takeoff,
                    Power::new::<watt>(27037.84),
                ),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(31226.)),
                (
                    PowerConsumerFlightPhase::Landing,
                    Power::new::<watt>(24586.64),
                ),
                (
                    PowerConsumerFlightPhase::TaxiIn,
                    Power::new::<watt>(24194.48),
                ),
            ]),
            ac_bus_2_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::AlternatingCurrent(2),
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(25342.56),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(20319.04),
                ),
                (
                    PowerConsumerFlightPhase::Takeoff,
                    Power::new::<watt>(22248.8),
                ),
                (
                    PowerConsumerFlightPhase::Flight,
                    Power::new::<watt>(25729.52),
                ),
                (
                    PowerConsumerFlightPhase::Landing,
                    Power::new::<watt>(20625.6),
                ),
                (
                    PowerConsumerFlightPhase::TaxiIn,
                    Power::new::<watt>(22511.04),
                ),
            ]),
            ac_ess_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::AlternatingCurrentEssential,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(364.56),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(572.56),
                ),
                (
                    PowerConsumerFlightPhase::Takeoff,
                    Power::new::<watt>(700.56),
                ),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(700.56)),
                (
                    PowerConsumerFlightPhase::Landing,
                    Power::new::<watt>(572.56),
                ),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(572.56)),
            ]),
            ac_ess_shed_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::AlternatingCurrentEssentialShed,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(448.4),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(658.8),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(658.8)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(658.8)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(658.8)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(658.8)),
            ]),
            ac_stat_inv_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::AlternatingCurrentStaticInverter,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(108.),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(108.),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(108.)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(108.)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(108.)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(108.)),
            ]),
            dc_bus_1_consumer: FlightPhasePowerConsumer::from(ElectricalBusType::DirectCurrent(1))
                .demand([
                    (
                        PowerConsumerFlightPhase::BeforeStart,
                        Power::new::<watt>(864.08),
                    ),
                    (
                        PowerConsumerFlightPhase::AfterStart,
                        Power::new::<watt>(1044.8),
                    ),
                    (
                        PowerConsumerFlightPhase::Takeoff,
                        Power::new::<watt>(1155.44),
                    ),
                    (PowerConsumerFlightPhase::Flight, Power::new::<watt>(881.36)),
                    (
                        PowerConsumerFlightPhase::Landing,
                        Power::new::<watt>(1155.44),
                    ),
                    (
                        PowerConsumerFlightPhase::TaxiIn,
                        Power::new::<watt>(1084.16),
                    ),
                ]),
            dc_bus_2_consumer: FlightPhasePowerConsumer::from(ElectricalBusType::DirectCurrent(2))
                .demand([
                    (
                        PowerConsumerFlightPhase::BeforeStart,
                        Power::new::<watt>(1736.96),
                    ),
                    (
                        PowerConsumerFlightPhase::AfterStart,
                        Power::new::<watt>(1508.88),
                    ),
                    (
                        PowerConsumerFlightPhase::Takeoff,
                        Power::new::<watt>(1292.16),
                    ),
                    (
                        PowerConsumerFlightPhase::Flight,
                        Power::new::<watt>(1278.96),
                    ),
                    (
                        PowerConsumerFlightPhase::Landing,
                        Power::new::<watt>(1280.96),
                    ),
                    (
                        PowerConsumerFlightPhase::TaxiIn,
                        Power::new::<watt>(1456.96),
                    ),
                ]),
            dc_ess_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentEssential,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(538.48),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(719.2),
                ),
                (
                    PowerConsumerFlightPhase::Takeoff,
                    Power::new::<watt>(522.56),
                ),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(479.76)),
                (
                    PowerConsumerFlightPhase::Landing,
                    Power::new::<watt>(522.56),
                ),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(486.48)),
            ]),
            dc_ess_shed_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentEssentialShed,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(719.76),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(643.04),
                ),
                (
                    PowerConsumerFlightPhase::Takeoff,
                    Power::new::<watt>(662.16),
                ),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(614.24)),
                (
                    PowerConsumerFlightPhase::Landing,
                    Power::new::<watt>(662.16),
                ),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(585.44)),
            ]),
            dc_bat_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentBattery,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(80.),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(80.),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(80.)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(80.)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(80.)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(80.)),
            ]),
            dc_hot_bus_1_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentHot(1),
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(86.4),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(8.8),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(12.24)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(12.24)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(12.24)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(8.8)),
            ]),
            dc_hot_bus_2_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentHot(2),
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(19.44),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(19.44),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(19.44)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(19.44)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(19.44)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(19.44)),
            ]),
        }
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
