use msfs::{
    legacy::{AircraftVariable, NamedVariable},
    MSFSEvent,
};
use std::collections::HashMap;
use systems::{
    simulator::{Simulation, SimulatorReaderWriter},
    A320,
};

#[msfs::gauge(name=systems)]
async fn systems(mut gauge: msfs::Gauge) -> Result<(), Box<dyn std::error::Error>> {
    let mut simulation = Simulation::new(A320::new(), A320SimulatorReadWriter::new()?);

    while let Some(event) = gauge.next_event().await {
        if let MSFSEvent::PreDraw(d) = event {
            simulation.tick(d.delta_time());
        }
    }

    Ok(())
}

struct A320SimulatorReadWriter {
    dynamic_named_variables: HashMap<String, NamedVariable>,

    ambient_temperature: AircraftVariable,
    apu_generator_pb_on: AircraftVariable,
    external_power_available: AircraftVariable,
    external_power_pb_on: AircraftVariable,
    engine_generator_1_pb_on: AircraftVariable,
    engine_generator_2_pb_on: AircraftVariable,
    engine_1_n2: AircraftVariable,
    engine_2_n2: AircraftVariable,
    indicated_airspeed: AircraftVariable,
    indicated_altitude: AircraftVariable,
    left_inner_tank_fuel_quantity: AircraftVariable,
    unlimited_fuel: AircraftVariable,
}
impl A320SimulatorReadWriter {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(A320SimulatorReadWriter {
            dynamic_named_variables: HashMap::new(),
            ambient_temperature: AircraftVariable::from("AMBIENT TEMPERATURE", "celsius", 0)?,
            apu_generator_pb_on: AircraftVariable::from("APU GENERATOR SWITCH", "Bool", 0)?,
            external_power_available: AircraftVariable::from(
                "EXTERNAL POWER AVAILABLE",
                "Bool",
                1,
            )?,
            external_power_pb_on: AircraftVariable::from("EXTERNAL POWER ON", "Bool", 1)?,
            engine_generator_1_pb_on: AircraftVariable::from(
                "GENERAL ENG MASTER ALTERNATOR",
                "Bool",
                1,
            )?,
            engine_generator_2_pb_on: AircraftVariable::from(
                "GENERAL ENG MASTER ALTERNATOR",
                "Bool",
                2,
            )?,
            engine_1_n2: AircraftVariable::from("ENG N2 RPM", "Percent", 1)?,
            engine_2_n2: AircraftVariable::from("ENG N2 RPM", "Percent", 2)?,
            indicated_airspeed: AircraftVariable::from("AIRSPEED INDICATED", "Knots", 0)?,
            indicated_altitude: AircraftVariable::from("INDICATED ALTITUDE", "Feet", 0)?,
            left_inner_tank_fuel_quantity: AircraftVariable::from(
                "FUEL TANK LEFT MAIN QUANTITY",
                "Pounds",
                0,
            )?,
            unlimited_fuel: AircraftVariable::from("UNLIMITED FUEL", "Bool", 0)?,
        })
    }
}
impl SimulatorReaderWriter for A320SimulatorReadWriter {
    fn read(&mut self, name: &str) -> f64 {
        match name {
            "OVHD_ELEC_APU_GEN_PB_IS_ON" => self.apu_generator_pb_on.get(),
            "OVHD_ELEC_EXT_PWR_PB_IS_AVAILABLE" => self.external_power_available.get(),
            "OVHD_ELEC_EXT_PWR_PB_IS_ON" => self.external_power_pb_on.get(),
            "OVHD_ELEC_ENG_GEN_1_PB_IS_ON" => self.engine_generator_1_pb_on.get(),
            "OVHD_ELEC_ENG_GEN_2_PB_IS_ON" => self.engine_generator_2_pb_on.get(),
            "AMBIENT TEMPERATURE" => self.ambient_temperature.get(),
            "EXTERNAL POWER AVAILABLE:1" => self.external_power_available.get(),
            "ENG N2 RPM:1" => self.engine_1_n2.get(),
            "ENG N2 RPM:2" => self.engine_2_n2.get(),
            "FUEL TANK LEFT MAIN QUANTITY" => self.left_inner_tank_fuel_quantity.get(),
            "UNLIMITED FUEL" => self.unlimited_fuel.get(),
            "AIRSPEED INDICATED" => self.indicated_airspeed.get(),
            "INDICATED ALTITUDE" => self.indicated_altitude.get(),
            _ => {
                lookup_named_variable(&mut self.dynamic_named_variables, "A32NX_", name).get_value()
            }
        }
    }

    fn write(&mut self, name: &str, value: f64) {
        let named_variable =
            lookup_named_variable(&mut self.dynamic_named_variables, "A32NX_", name);

        named_variable.set_value(value);
    }
}

fn lookup_named_variable<'a>(
    collection: &'a mut HashMap<String, NamedVariable>,
    key_prefix: &str,
    key: &str,
) -> &'a mut NamedVariable {
    let key = format!("{}{}", key_prefix, key);

    collection
        .entry(key.clone())
        .or_insert_with(|| NamedVariable::from(&key))
}
