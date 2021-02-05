use crate::{
    engine::Engine,
    overhead::OnOffPushButton,
    simulator::{
        SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, UpdateContext,
    },
};
use std::{cmp::min, collections::HashMap, vec};
use uom::si::{
    electric_charge::ampere_hour, f64::*, power::watt, ratio::percent,
    thermodynamic_temperature::degree_celsius,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElectricPowerSource {
    EngineGenerator(u8),
    ApuGenerator,
    External,
    EmergencyGenerator,
    Battery(u8),
    Batteries,
    TransformerRectifier(u8),
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

pub struct EngineGenerator {
    number: u8,
    idg: IntegratedDriveGenerator,
}
impl EngineGenerator {
    pub fn new(number: u8) -> EngineGenerator {
        EngineGenerator {
            number,
            idg: IntegratedDriveGenerator::new(),
        }
    }

    pub fn update(
        &mut self,
        context: &UpdateContext,
        engine: &Engine,
        idg_push_button: &OnOffPushButton,
    ) {
        self.idg.update(context, engine, idg_push_button);
    }
}
impl ElectricSource for EngineGenerator {
    fn output(&self) -> Current {
        if self.idg.provides_stable_power_output() {
            Current::some(ElectricPowerSource::EngineGenerator(self.number))
        } else {
            Current::none()
        }
    }
}
impl SimulatorElementVisitable for EngineGenerator {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for EngineGenerator {
    fn write_power_consumption(&mut self, state: &PowerConsumptionState) {
        // TODO
    }
}

pub(crate) struct IntegratedDriveGenerator {
    oil_outlet_temperature: ThermodynamicTemperature,
    time_above_threshold_in_milliseconds: u64,
    connected: bool,
}
impl IntegratedDriveGenerator {
    pub const ENGINE_N2_POWER_UP_OUTPUT_THRESHOLD: f64 = 59.5;
    pub const ENGINE_N2_POWER_DOWN_OUTPUT_THRESHOLD: f64 = 56.;
    const STABILIZATION_TIME_IN_MILLISECONDS: u64 = 500;

    fn new() -> IntegratedDriveGenerator {
        IntegratedDriveGenerator {
            oil_outlet_temperature: ThermodynamicTemperature::new::<degree_celsius>(0.),
            time_above_threshold_in_milliseconds: 0,
            connected: true,
        }
    }

    fn update(
        &mut self,
        context: &UpdateContext,
        engine: &Engine,
        idg_push_button: &OnOffPushButton,
    ) {
        if idg_push_button.is_off() {
            // The IDG cannot be reconnected.
            self.connected = false;
        }

        self.update_stable_time(context, engine);
        self.update_temperature(context, self.get_target_temperature(context, engine));
    }

    fn provides_stable_power_output(&self) -> bool {
        self.time_above_threshold_in_milliseconds
            == IntegratedDriveGenerator::STABILIZATION_TIME_IN_MILLISECONDS
    }

    fn update_stable_time(&mut self, context: &UpdateContext, engine: &Engine) {
        if !self.connected {
            self.time_above_threshold_in_milliseconds = 0;
            return;
        }

        let mut new_time = self.time_above_threshold_in_milliseconds;
        if engine.n2
            >= Ratio::new::<percent>(IntegratedDriveGenerator::ENGINE_N2_POWER_UP_OUTPUT_THRESHOLD)
            && self.time_above_threshold_in_milliseconds
                < IntegratedDriveGenerator::STABILIZATION_TIME_IN_MILLISECONDS
        {
            new_time = self.time_above_threshold_in_milliseconds + context.delta.as_millis() as u64;
        } else if engine.n2
            <= Ratio::new::<percent>(
                IntegratedDriveGenerator::ENGINE_N2_POWER_DOWN_OUTPUT_THRESHOLD,
            )
            && self.time_above_threshold_in_milliseconds > 0
        {
            new_time = self.time_above_threshold_in_milliseconds
                - min(
                    context.delta.as_millis() as u64,
                    self.time_above_threshold_in_milliseconds,
                );
        }

        self.time_above_threshold_in_milliseconds = clamp(
            new_time,
            0,
            IntegratedDriveGenerator::STABILIZATION_TIME_IN_MILLISECONDS,
        );
    }

    fn update_temperature(&mut self, context: &UpdateContext, target: ThermodynamicTemperature) {
        const IDG_HEATING_COEFFICIENT: f64 = 1.4;
        const IDG_COOLING_COEFFICIENT: f64 = 0.4;

        let target_temperature = target.get::<degree_celsius>();
        let mut temperature = self.oil_outlet_temperature.get::<degree_celsius>();
        temperature += if temperature < target_temperature {
            IDG_HEATING_COEFFICIENT * context.delta.as_secs_f64()
        } else {
            -(IDG_COOLING_COEFFICIENT * context.delta.as_secs_f64())
        };

        temperature = clamp(
            temperature,
            context.ambient_temperature.get::<degree_celsius>(),
            target.get::<degree_celsius>(),
        );

        self.oil_outlet_temperature = ThermodynamicTemperature::new::<degree_celsius>(temperature);
    }

    fn get_target_temperature(
        &self,
        context: &UpdateContext,
        engine: &Engine,
    ) -> ThermodynamicTemperature {
        if !self.connected {
            return context.ambient_temperature;
        }

        let mut target_idg = engine.n2.get::<percent>() * 1.8;
        let ambient_temperature = context.ambient_temperature.get::<degree_celsius>();
        target_idg += ambient_temperature;

        // TODO improve this function with feedback @komp provides.

        ThermodynamicTemperature::new::<degree_celsius>(target_idg)
    }
}

/// Experimental feature copied from Rust stb lib.
fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    assert!(min <= max);
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

pub struct ExternalPowerSource {
    pub is_connected: bool,
}
impl ExternalPowerSource {
    pub fn new() -> ExternalPowerSource {
        ExternalPowerSource {
            is_connected: false,
        }
    }

    pub fn update(&mut self, _: &UpdateContext) {}
}
impl ElectricSource for ExternalPowerSource {
    fn output(&self) -> Current {
        if self.is_connected {
            Current::some(ElectricPowerSource::External)
        } else {
            Current::none()
        }
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

pub struct TransformerRectifier {
    number: u8,
    input: Current,
    failed: bool,
}
impl TransformerRectifier {
    pub fn new(number: u8) -> TransformerRectifier {
        TransformerRectifier {
            number,
            input: Current::none(),
            failed: false,
        }
    }

    #[cfg(test)]
    pub fn fail(&mut self) {
        self.failed = true;
    }

    pub fn has_failed(&self) -> bool {
        self.failed
    }
}
impl Powerable for TransformerRectifier {
    fn set_input(&mut self, current: Current) {
        self.input = current;
    }

    fn get_input(&self) -> Current {
        self.input
    }
}
impl ElectricSource for TransformerRectifier {
    fn output(&self) -> Current {
        if self.failed {
            Current::none()
        } else if self.input.is_powered() {
            Current::some(ElectricPowerSource::TransformerRectifier(self.number))
        } else {
            Current::none()
        }
    }
}
impl SimulatorElementVisitable for TransformerRectifier {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for TransformerRectifier {
    fn write_power_consumption(&mut self, state: &PowerConsumptionState) {
        // TODO
    }
}

pub struct EmergencyGenerator {
    running: bool,
    is_blue_pressurised: bool,
}
impl EmergencyGenerator {
    pub fn new() -> EmergencyGenerator {
        EmergencyGenerator {
            running: false,
            is_blue_pressurised: false,
        }
    }

    pub fn update(&mut self, is_blue_pressurised: bool) {
        // TODO: The emergency generator is driven by the blue hydraulic circuit. Still to be implemented.
        self.is_blue_pressurised = is_blue_pressurised;
    }

    #[cfg(test)]
    pub fn attempt_start(&mut self) {
        self.running = true;
    }

    pub fn is_running(&self) -> bool {
        self.is_blue_pressurised && self.running
    }
}
impl ElectricSource for EmergencyGenerator {
    fn output(&self) -> Current {
        if self.is_running() {
            Current::some(ElectricPowerSource::EmergencyGenerator)
        } else {
            Current::none()
        }
    }
}
impl SimulatorElementVisitable for EmergencyGenerator {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for EmergencyGenerator {
    fn write_power_consumption(&mut self, state: &PowerConsumptionState) {
        // TODO
    }
}

pub struct Battery {
    number: u8,
    input: Current,
    charge: ElectricCharge,
}
impl Battery {
    const MAX_ELECTRIC_CHARGE_AMPERE_HOURS: f64 = 23.0;

    pub fn full(number: u8) -> Battery {
        Battery::new(
            number,
            ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS),
        )
    }

    #[cfg(test)]
    pub fn empty(number: u8) -> Battery {
        Battery::new(number, ElectricCharge::new::<ampere_hour>(0.))
    }

    fn new(number: u8, charge: ElectricCharge) -> Battery {
        Battery {
            number,
            input: Current::none(),
            charge,
        }
    }

    pub fn is_full(&self) -> bool {
        self.charge >= ElectricCharge::new::<ampere_hour>(Battery::MAX_ELECTRIC_CHARGE_AMPERE_HOURS)
    }
}
impl Powerable for Battery {
    fn set_input(&mut self, current: Current) {
        self.input = current
    }

    fn get_input(&self) -> Current {
        self.input
    }
}
impl ElectricSource for Battery {
    fn output(&self) -> Current {
        if self.input.is_unpowered() && self.charge > ElectricCharge::new::<ampere_hour>(0.) {
            Current::some(ElectricPowerSource::Battery(self.number))
        } else {
            Current::none()
        }
    }
}
impl SimulatorElementVisitable for Battery {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for Battery {
    fn write_power_consumption(&mut self, state: &PowerConsumptionState) {
        // TODO: Charging and depleting battery when used.
    }
}

pub struct StaticInverter {
    input: Current,
}
impl StaticInverter {
    pub fn new() -> StaticInverter {
        StaticInverter {
            input: Current::none(),
        }
    }
}
impl Powerable for StaticInverter {
    fn set_input(&mut self, current: Current) {
        self.input = current;
    }

    fn get_input(&self) -> Current {
        self.input
    }
}
impl ElectricSource for StaticInverter {
    fn output(&self) -> Current {
        if self.input.is_powered() {
            Current::some(ElectricPowerSource::StaticInverter)
        } else {
            Current::none()
        }
    }
}
impl SimulatorElementVisitable for StaticInverter {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for StaticInverter {
    fn write_power_consumption(&mut self, state: &PowerConsumptionState) {
        // TODO
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

    fn apu_generator() -> StubApuGenerator {
        StubApuGenerator {}
    }

    fn engine_above_threshold() -> Engine {
        engine(Ratio::new::<percent>(
            IntegratedDriveGenerator::ENGINE_N2_POWER_UP_OUTPUT_THRESHOLD + 1.,
        ))
    }

    fn engine_below_threshold() -> Engine {
        engine(Ratio::new::<percent>(
            IntegratedDriveGenerator::ENGINE_N2_POWER_DOWN_OUTPUT_THRESHOLD - 1.,
        ))
    }

    fn engine(n2: Ratio) -> Engine {
        let mut engine = Engine::new(1);
        engine.n2 = n2;

        engine
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

    #[cfg(test)]
    mod engine_generator_tests {
        use std::time::Duration;

        use crate::simulator::test_helpers::context_with;

        use super::*;

        #[test]
        fn starts_without_output() {
            assert!(engine_generator().is_unpowered());
        }

        #[test]
        fn when_engine_n2_above_threshold_provides_output() {
            let mut generator = engine_generator();
            update_below_threshold(&mut generator);
            update_above_threshold(&mut generator);

            assert!(generator.is_powered());
        }

        #[test]
        fn when_engine_n2_below_threshold_provides_no_output() {
            let mut generator = engine_generator();
            update_above_threshold(&mut generator);
            update_below_threshold(&mut generator);

            assert!(generator.is_unpowered());
        }

        #[test]
        fn when_idg_disconnected_provides_no_output() {
            let mut generator = engine_generator();
            generator.update(
                &context_with().delta(Duration::from_secs(0)).build(),
                &engine_above_threshold(),
                &OnOffPushButton::new_off(),
            );

            assert!(generator.is_unpowered());
        }

        fn engine_generator() -> EngineGenerator {
            EngineGenerator::new(1)
        }

        fn update_above_threshold(generator: &mut EngineGenerator) {
            generator.update(
                &context_with().delta(Duration::from_secs(1)).build(),
                &engine_above_threshold(),
                &OnOffPushButton::new_on(),
            );
        }

        fn update_below_threshold(generator: &mut EngineGenerator) {
            generator.update(
                &context_with().delta(Duration::from_secs(1)).build(),
                &engine_below_threshold(),
                &OnOffPushButton::new_on(),
            );
        }
    }

    #[cfg(test)]
    mod integrated_drive_generator_tests {
        use std::time::Duration;

        use crate::simulator::test_helpers::context_with;

        use super::*;

        fn idg() -> IntegratedDriveGenerator {
            IntegratedDriveGenerator::new()
        }

        #[test]
        fn starts_unstable() {
            assert_eq!(idg().provides_stable_power_output(), false);
        }

        #[test]
        fn becomes_stable_once_engine_above_threshold_for_500_milliseconds() {
            let mut idg = idg();
            idg.update(
                &context_with().delta(Duration::from_millis(500)).build(),
                &engine_above_threshold(),
                &OnOffPushButton::new_on(),
            );

            assert_eq!(idg.provides_stable_power_output(), true);
        }

        #[test]
        fn does_not_become_stable_before_engine_above_threshold_for_500_milliseconds() {
            let mut idg = idg();
            idg.update(
                &context_with().delta(Duration::from_millis(499)).build(),
                &engine_above_threshold(),
                &OnOffPushButton::new_on(),
            );

            assert_eq!(idg.provides_stable_power_output(), false);
        }

        #[test]
        fn cannot_reconnect_once_disconnected() {
            let mut idg = idg();
            idg.update(
                &context_with().delta(Duration::from_millis(500)).build(),
                &engine_above_threshold(),
                &OnOffPushButton::new_off(),
            );

            idg.update(
                &context_with().delta(Duration::from_millis(500)).build(),
                &engine_above_threshold(),
                &OnOffPushButton::new_on(),
            );

            assert_eq!(idg.provides_stable_power_output(), false);
        }

        #[test]
        fn running_engine_warms_up_idg() {
            let mut idg = idg();
            let starting_temperature = idg.oil_outlet_temperature;
            idg.update(
                &context_with().delta(Duration::from_secs(10)).build(),
                &engine_above_threshold(),
                &OnOffPushButton::new_on(),
            );

            assert!(idg.oil_outlet_temperature > starting_temperature);
        }

        #[test]
        fn running_engine_does_not_warm_up_idg_when_disconnected() {
            let mut idg = idg();
            let starting_temperature = idg.oil_outlet_temperature;
            idg.update(
                &context_with().delta(Duration::from_secs(10)).build(),
                &engine_above_threshold(),
                &OnOffPushButton::new_off(),
            );

            assert_eq!(idg.oil_outlet_temperature, starting_temperature);
        }

        #[test]
        fn shutdown_engine_cools_down_idg() {
            let mut idg = idg();
            idg.update(
                &context_with().delta(Duration::from_secs(10)).build(),
                &engine_above_threshold(),
                &OnOffPushButton::new_on(),
            );
            let starting_temperature = idg.oil_outlet_temperature;

            idg.update(
                &context_with().delta(Duration::from_secs(10)).build(),
                &Engine::new(1),
                &OnOffPushButton::new_on(),
            );

            assert!(idg.oil_outlet_temperature < starting_temperature);
        }
    }

    #[cfg(test)]
    mod external_power_source_tests {
        use super::*;

        #[test]
        fn starts_without_output() {
            assert!(external_power_source().is_unpowered());
        }

        #[test]
        fn when_plugged_in_provides_output() {
            let mut ext_pwr = external_power_source();
            ext_pwr.is_connected = true;

            assert!(ext_pwr.is_powered());
        }

        #[test]
        fn when_not_plugged_in_provides_no_output() {
            let mut ext_pwr = external_power_source();
            ext_pwr.is_connected = false;

            assert!(ext_pwr.is_unpowered());
        }

        fn external_power_source() -> ExternalPowerSource {
            ExternalPowerSource::new()
        }
    }

    #[cfg(test)]
    mod transformer_rectifier_tests {
        use super::*;

        #[test]
        fn starts_without_output() {
            assert!(transformer_rectifier().is_unpowered());
        }

        #[test]
        fn when_powered_outputs_current() {
            let mut tr = transformer_rectifier();
            tr.powered_by(&apu_generator());

            assert!(tr.is_powered());
        }

        #[test]
        fn when_powered_but_failed_has_no_output() {
            let mut tr = transformer_rectifier();
            tr.powered_by(&apu_generator());
            tr.fail();

            assert!(tr.is_unpowered());
        }

        #[test]
        fn when_unpowered_has_no_output() {
            let mut tr = transformer_rectifier();
            tr.powered_by(&Powerless {});

            assert!(tr.is_unpowered());
        }

        fn transformer_rectifier() -> TransformerRectifier {
            TransformerRectifier::new(1)
        }
    }

    #[cfg(test)]
    mod emergency_generator_tests {
        use super::*;

        #[test]
        fn starts_without_output() {
            assert!(emergency_generator().is_unpowered());
        }

        #[test]
        fn when_started_provides_output() {
            let mut emer_gen = emergency_generator();
            emer_gen.attempt_start();
            emer_gen.update(true);

            assert!(emer_gen.is_powered());
        }

        #[test]
        fn when_started_without_hydraulic_pressure_is_unpowered() {
            let mut emer_gen = emergency_generator();
            emer_gen.attempt_start();
            emer_gen.update(false);

            assert!(emer_gen.is_unpowered());
        }

        fn emergency_generator() -> EmergencyGenerator {
            EmergencyGenerator::new()
        }
    }

    #[cfg(test)]
    mod battery_tests {
        use super::*;

        #[test]
        fn full_battery_has_output() {
            assert!(full_battery().is_full());
            assert!(full_battery().is_powered());
        }

        #[test]
        fn empty_battery_has_no_output() {
            assert!(!empty_battery().is_full());
            assert!(empty_battery().is_unpowered());
        }

        #[test]
        fn when_empty_battery_has_input_doesnt_have_output() {
            let mut battery = empty_battery();
            battery.powered_by(&apu_generator());

            assert!(battery.is_unpowered());
        }

        #[test]
        fn when_full_battery_has_doesnt_have_output() {
            // Of course battery input at this stage would result in overcharging. However, for the sake of the test we ignore it.
            let mut battery = full_battery();
            battery.powered_by(&apu_generator());

            assert!(battery.is_unpowered());
        }

        #[test]
        fn charged_battery_without_input_has_output() {
            let mut battery = full_battery();
            battery.powered_by(&Powerless {});

            assert!(battery.is_powered());
        }

        #[test]
        fn empty_battery_without_input_has_no_output() {
            let mut battery = empty_battery();
            battery.powered_by(&Powerless {});

            assert!(battery.is_unpowered());
        }

        fn full_battery() -> Battery {
            Battery::full(1)
        }

        fn empty_battery() -> Battery {
            Battery::empty(1)
        }
    }

    #[cfg(test)]
    mod static_inverter_tests {
        use super::*;

        #[test]
        fn starts_without_output() {
            assert!(static_inverter().is_unpowered());
        }

        #[test]
        fn when_powered_has_output() {
            let mut static_inv = static_inverter();
            static_inv.powered_by(&battery());

            assert!(static_inv.is_powered());
        }

        #[test]
        fn when_unpowered_has_no_output() {
            let mut static_inv = static_inverter();
            static_inv.powered_by(&Powerless {});

            assert!(static_inv.is_unpowered());
        }

        fn static_inverter() -> StaticInverter {
            StaticInverter::new()
        }

        fn battery() -> Battery {
            Battery::full(1)
        }
    }

    #[cfg(test)]
    mod power_supply_tests {
        use super::*;

        fn power_supply() -> PowerSupply {
            PowerSupply::new()
        }

        fn powered_bus(bus_type: ElectricalBusType) -> ElectricalBus {
            let mut bus = unpowered_bus(bus_type);
            bus.set_input(Current::some(ElectricPowerSource::ApuGenerator));

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

        fn powered_bus(bus_type: ElectricalBusType) -> ElectricalBus {
            let mut bus = ElectricalBus::new(bus_type);
            bus.set_input(Current::some(ElectricPowerSource::ApuGenerator));

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
                powered.set_input(Current::some(ElectricPowerSource::ApuGenerator));

                supply.add(&powered);
                supply.add(&ElectricalBus::new(ElectricalBusType::AlternatingCurrent(
                    1,
                )));

                supply
            }
        }

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
                self.used_power =
                    state.get_total_consumption_for(&ElectricPowerSource::ApuGenerator);
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
