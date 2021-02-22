use std::{collections::HashMap, time::Duration};
use uom::si::{f64::*, length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

use crate::electrical::consumption::SuppliedPower;

use super::{
    from_bool, to_bool, Aircraft, Simulation, SimulationElement, SimulationElementVisitor,
    SimulationToSimulatorVisitor, SimulatorReaderWriter, SimulatorWriter, UpdateContext,
};

struct TestAircraft<'a, T: SimulationElement, U: Fn(&mut T, &UpdateContext)> {
    element: &'a mut T,
    update_fn: U,
    supplied_power: Option<SuppliedPower>,
    update_before_power_distribution: bool,
}
impl<'a, T: SimulationElement, U: Fn(&mut T, &UpdateContext)> TestAircraft<'a, T, U> {
    fn new(
        element: &'a mut T,
        update_fn: U,
        supplied_power: SuppliedPower,
        update_before_power_distribution: bool,
    ) -> Self {
        Self {
            element,
            update_fn,
            supplied_power: Some(supplied_power),
            update_before_power_distribution,
        }
    }
}
impl<'a, T: SimulationElement, U: Fn(&mut T, &UpdateContext)> Aircraft for TestAircraft<'a, T, U> {
    fn update_before_power_distribution(&mut self, context: &UpdateContext) {
        if self.update_before_power_distribution {
            (self.update_fn)(&mut self.element, context);
        }
    }

    fn update_after_power_distribution(&mut self, context: &UpdateContext) {
        if !self.update_before_power_distribution {
            (self.update_fn)(&mut self.element, context);
        }
    }

    fn get_supplied_power(&mut self) -> SuppliedPower {
        self.supplied_power.take().unwrap()
    }
}
impl<'a, T: SimulationElement, U: Fn(&mut T, &UpdateContext)> SimulationElement
    for TestAircraft<'a, T, U>
{
    fn accept<W: SimulationElementVisitor>(&mut self, visitor: &mut W) {
        visitor.visit(self.element);
    }
}

pub struct TestReaderWriter {
    variables: HashMap<String, f64>,
}
impl TestReaderWriter {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn contains_f64(&self, name: &str, value: f64) -> bool {
        if let Some(val) = self.variables.get(name) {
            (val - value).abs() < f64::EPSILON
        } else {
            false
        }
    }

    pub fn contains_bool(&self, name: &str, value: bool) -> bool {
        self.contains_f64(name, from_bool(value))
    }

    pub fn contains_key(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    pub fn len_is(&self, length: usize) -> bool {
        self.variables.len() == length
    }

    fn write_bool(&mut self, name: &str, value: bool) {
        self.write(name, from_bool(value));
    }

    fn write_f64(&mut self, name: &str, value: f64) {
        self.write(name, value);
    }

    fn read_bool(&mut self, name: &str) -> bool {
        to_bool(self.read(name))
    }

    fn read_f64(&mut self, name: &str) -> f64 {
        self.read(name)
    }
}
impl SimulatorReaderWriter for TestReaderWriter {
    fn read(&mut self, name: &str) -> f64 {
        *self.variables.get(name).unwrap_or(&0.)
    }

    fn write(&mut self, name: &str, value: f64) {
        self.variables.insert(name.to_owned(), value);
    }
}
impl Default for TestReaderWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// The simulation test bed handles the testing of [`SimulationElement`]s
/// by running a full simulation tick on them.
///
/// [`SimulationElement`]: ../trait.SimulationElement.html
pub struct SimulationTestBed {
    reader_writer: TestReaderWriter,
    get_supplied_power_fn: Box<dyn Fn() -> SuppliedPower>,
    delta: Duration,
}
impl SimulationTestBed {
    pub fn new() -> Self {
        let bed = Self {
            reader_writer: TestReaderWriter::new(),
            get_supplied_power_fn: Box::new(|| SuppliedPower::new()),
            delta: Duration::from_secs(1),
        };

        bed.indicated_airspeed(Velocity::new::<knot>(250.))
            .indicated_altitude(Length::new::<foot>(5000.))
            .ambient_temperature(ThermodynamicTemperature::new::<degree_celsius>(0.))
            .on_ground(false)
    }

    pub fn seeded_with<T: SimulationElement>(element: &mut T) -> Self {
        let mut test_bed = Self::new();

        let mut writer = SimulatorWriter::new(&mut test_bed.reader_writer);
        let mut visitor = SimulationToSimulatorVisitor::new(&mut writer);
        element.accept(&mut visitor);

        test_bed
    }

    pub fn run_aircraft<T: Aircraft>(&mut self, aircraft: &mut T) {
        let mut simulation = Simulation::new(aircraft, &mut self.reader_writer);
        simulation.tick(self.delta);
    }

    pub fn run_before_power_distribution<T: SimulationElement, U: Fn(&mut T, &UpdateContext)>(
        &mut self,
        element: &mut T,
        update_fn: U,
    ) {
        self.run_within_test_aircraft(element, update_fn, true);
    }

    pub fn run_before_power_distribution_without_update<T: SimulationElement>(
        &mut self,
        element: &mut T,
    ) {
        self.run_before_power_distribution(element, |_, _| {});
    }

    pub fn run<T: SimulationElement, U: Fn(&mut T, &UpdateContext)>(
        &mut self,
        element: &mut T,
        update_fn: U,
    ) {
        self.run_within_test_aircraft(element, update_fn, false);
    }

    pub fn run_without_update<T: SimulationElement>(&mut self, element: &mut T) {
        self.run(element, |_, _| {});
    }

    fn run_within_test_aircraft<T: SimulationElement, U: Fn(&mut T, &UpdateContext)>(
        &mut self,
        element: &mut T,
        update_fn: U,
        before_power_distribution: bool,
    ) {
        let mut aircraft = TestAircraft::new(
            element,
            update_fn,
            (self.get_supplied_power_fn)(),
            before_power_distribution,
        );

        self.run_aircraft(&mut aircraft);
    }

    pub fn and(self) -> Self {
        self
    }

    pub fn delta(mut self, delta: Duration) -> Self {
        self.delta = delta;
        self
    }

    pub fn indicated_airspeed(mut self, indicated_airspeed: Velocity) -> Self {
        self.reader_writer.write_f64(
            UpdateContext::INDICATED_AIRSPEED_KEY,
            indicated_airspeed.get::<knot>(),
        );
        self
    }

    pub fn indicated_altitude(mut self, indicated_altitude: Length) -> Self {
        self.reader_writer.write_f64(
            UpdateContext::INDICATED_ALTITUDE_KEY,
            indicated_altitude.get::<foot>(),
        );
        self
    }

    pub fn ambient_temperature(mut self, ambient_temperature: ThermodynamicTemperature) -> Self {
        self.reader_writer.write_f64(
            UpdateContext::AMBIENT_TEMPERATURE_KEY,
            ambient_temperature.get::<degree_celsius>(),
        );
        self
    }

    pub fn on_ground(mut self, on_ground: bool) -> Self {
        self.reader_writer
            .write_bool(UpdateContext::IS_ON_GROUND_KEY, on_ground);
        self
    }

    pub fn supplied_power_fn<T: Fn() -> SuppliedPower + 'static>(
        mut self,
        supplied_power_fn: T,
    ) -> Self {
        self.get_supplied_power_fn = Box::new(supplied_power_fn);
        self
    }

    pub fn write_bool(&mut self, name: &str, value: bool) {
        self.reader_writer.write_bool(name, value);
    }

    pub fn write_f64(&mut self, name: &str, value: f64) {
        self.reader_writer.write_f64(name, value);
    }

    pub fn read_bool(&mut self, name: &str) -> bool {
        self.reader_writer.read_bool(name)
    }

    pub fn read_f64(&mut self, name: &str) -> f64 {
        self.reader_writer.read_f64(name)
    }

    pub fn contains_f64(&self, name: &str, value: f64) -> bool {
        self.reader_writer.contains_f64(name, value)
    }

    pub fn contains_bool(&self, name: &str, value: bool) -> bool {
        self.reader_writer.contains_bool(name, value)
    }

    pub fn contains_key(&self, name: &str) -> bool {
        self.reader_writer.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        electrical::consumption::{PowerConsumption, PowerConsumptionReport, SuppliedPower},
        simulation::{SimulatorReader, SimulatorWriter},
    };

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum CallOrder {
        Before,
        After,
    }

    #[derive(Default)]
    struct ElementUnderTest {
        update_called: bool,
        read_called: bool,
        receive_power_called: bool,
        consume_power_called: bool,
        consume_power_in_converters_called: bool,
        process_power_consumption_report_called: bool,
        update_called_before_or_after_receive_power: Option<CallOrder>,
    }
    impl ElementUnderTest {
        fn update(&mut self, _: &UpdateContext) {
            self.update_called = true;
            self.update_called_before_or_after_receive_power = if self.receive_power_called {
                Some(CallOrder::After)
            } else {
                Some(CallOrder::Before)
            };
        }

        fn all_functions_called(&self) -> bool {
            self.update_called
                && self.read_called
                && self.receive_power_called
                && self.consume_power_called
                && self.consume_power_in_converters_called
                && self.process_power_consumption_report_called
        }

        fn update_called_before_or_after_receive_power(&self) -> Option<CallOrder> {
            self.update_called_before_or_after_receive_power
        }
    }
    impl SimulationElement for ElementUnderTest {
        fn read(&mut self, _: &mut SimulatorReader) {
            self.read_called = true;
        }

        fn write(&self, _: &mut SimulatorWriter) {
            // Can't check this as the fn doesn't require mutable self.
        }

        fn receive_power(&mut self, _: &SuppliedPower) {
            self.receive_power_called = true;
        }

        fn consume_power(&mut self, _: &mut PowerConsumption) {
            self.consume_power_called = true;
        }

        fn consume_power_in_converters(&mut self, _consumption: &mut PowerConsumption) {
            self.consume_power_in_converters_called = true;
        }

        fn process_power_consumption_report<T: PowerConsumptionReport>(
            &mut self,
            _: &T,
            _: &UpdateContext,
        ) {
            self.process_power_consumption_report_called = true;
        }
    }

    #[test]
    fn test_aircraft_can_run_in_simulation() {
        let mut element = ElementUnderTest::default();
        let mut bed = SimulationTestBed::new();
        bed.run_before_power_distribution(&mut element, |el, context| {
            el.update(context);
        });

        assert!(element.all_functions_called());
    }

    #[test]
    fn defaults_to_receiving_power_before_update() {
        let mut element = ElementUnderTest::default();
        let mut bed = SimulationTestBed::new();
        bed.run(&mut element, |el, context| {
            el.update(context);
        });

        assert_eq!(
            element.update_called_before_or_after_receive_power(),
            Some(CallOrder::After)
        );
    }

    #[test]
    fn when_update_before_receive_power_requested_executes_update_before_receive_power() {
        let mut element = ElementUnderTest::default();
        let mut bed = SimulationTestBed::new();
        bed.run_before_power_distribution(&mut element, |el, context| {
            el.update(context);
        });

        assert_eq!(
            element.update_called_before_or_after_receive_power(),
            Some(CallOrder::Before)
        );
    }
}
