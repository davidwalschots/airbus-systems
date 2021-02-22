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
                        Power::new::<watt>(1080.1),
                    ),
                    (
                        PowerConsumerFlightPhase::AfterStart,
                        Power::new::<watt>(1306.),
                    ),
                    (
                        PowerConsumerFlightPhase::Takeoff,
                        Power::new::<watt>(1444.3),
                    ),
                    (PowerConsumerFlightPhase::Flight, Power::new::<watt>(1101.7)),
                    (
                        PowerConsumerFlightPhase::Landing,
                        Power::new::<watt>(1444.3),
                    ),
                    (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(1355.2)),
                ]),
            dc_bus_2_consumer: FlightPhasePowerConsumer::from(ElectricalBusType::DirectCurrent(2))
                .demand([
                    (
                        PowerConsumerFlightPhase::BeforeStart,
                        Power::new::<watt>(2171.2),
                    ),
                    (
                        PowerConsumerFlightPhase::AfterStart,
                        Power::new::<watt>(1806.1),
                    ),
                    (
                        PowerConsumerFlightPhase::Takeoff,
                        Power::new::<watt>(1615.2),
                    ),
                    (PowerConsumerFlightPhase::Flight, Power::new::<watt>(1598.7)),
                    (
                        PowerConsumerFlightPhase::Landing,
                        Power::new::<watt>(1601.2),
                    ),
                    (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(1821.2)),
                ]),
            dc_ess_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentEssential,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(673.1),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(598.),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(653.2)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(599.7)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(653.2)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(608.1)),
            ]),
            dc_ess_shed_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentEssentialShed,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(899.7),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(731.8),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(827.7)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(767.8)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(827.7)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(731.8)),
            ]),
            dc_bat_bus_consumer: FlightPhasePowerConsumer::from(
                ElectricalBusType::DirectCurrentBattery,
            )
            .demand([
                (
                    PowerConsumerFlightPhase::BeforeStart,
                    Power::new::<watt>(30.7),
                ),
                (
                    PowerConsumerFlightPhase::AfterStart,
                    Power::new::<watt>(40.7),
                ),
                (PowerConsumerFlightPhase::Takeoff, Power::new::<watt>(96.7)),
                (PowerConsumerFlightPhase::Flight, Power::new::<watt>(96.7)),
                (PowerConsumerFlightPhase::Landing, Power::new::<watt>(96.7)),
                (PowerConsumerFlightPhase::TaxiIn, Power::new::<watt>(96.7)),
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
