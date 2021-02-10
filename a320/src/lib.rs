use std::{borrow::Borrow, collections::HashMap};

//#![cfg(any(target_arch = "wasm32", doc))]
use airbus_systems::{
    simulator::{
        from_bool, to_bool, Simulation, SimulatorApuReadState, SimulatorElectricalReadState,
        SimulatorFireReadState, SimulatorPneumaticReadState, SimulatorReadState,
        SimulatorReadWriter, SimulatorWriteState,
    },
    A320,
};
use msfs::{
    legacy::{AircraftVariable, NamedVariable},
    MSFSEvent,
};
use uom::si::{
    electric_current::ampere, electric_potential::volt, f64::*, frequency::hertz, length::foot,
    mass::pound, ratio::percent, thermodynamic_temperature::degree_celsius, velocity::knot,
};

#[msfs::gauge(name=systems)]
async fn systems(mut gauge: msfs::Gauge) -> Result<(), Box<dyn std::error::Error>> {
    let mut simulation = Simulation::new(A320::new(), A320SimulatorReadWriter::new()?);

    while let Some(event) = gauge.next_event().await {
        match event {
            MSFSEvent::PreDraw(d) => {
                simulation.tick(d.delta_time());
            }
            _ => {}
        }
    }

    Ok(())
}

struct A320SimulatorReadWriter {
    dynamic_named_variables: HashMap<String, NamedVariable>,

    ambient_temperature: AircraftVariable,
    apu_available: NamedVariable,
    apu_bleed_air_valve_open: NamedVariable,
    apu_bleed_pb_on: NamedVariable,
    apu_egt: NamedVariable,
    apu_egt_caution: NamedVariable,
    apu_egt_warning: NamedVariable,
    apu_fire_button_released: NamedVariable,
    apu_air_intake_flap_is_ecam_open: NamedVariable,
    apu_flap_open_percentage: NamedVariable,
    apu_generator_pb_on: AircraftVariable,
    apu_inoperable: NamedVariable,
    apu_is_auto_shutdown: NamedVariable,
    apu_is_emergency_shutdown: NamedVariable,
    apu_low_fuel_pressure_fault: NamedVariable,
    apu_master_sw_pb_on: NamedVariable,
    apu_n: NamedVariable,
    apu_start_contactor_energized: NamedVariable,
    apu_start_pb_on: NamedVariable,
    apu_start_pb_available: NamedVariable,
    elec_ac_ess_feed_pb_normal: NamedVariable,
    elec_battery_1_pb_auto: NamedVariable,
    elec_battery_2_pb_auto: NamedVariable,
    elec_bus_tie_pb_auto: NamedVariable,
    elec_commercial_pb_on: NamedVariable,
    elec_external_power_available: AircraftVariable,
    elec_external_power_pb_on: AircraftVariable,
    elec_galy_and_cab_pb_auto: NamedVariable,
    elec_generator_1_pb_on: AircraftVariable,
    elec_generator_2_pb_on: AircraftVariable,
    elec_idg_1_pb_released: NamedVariable,
    elec_idg_2_pb_released: NamedVariable,
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
            apu_available: NamedVariable::from("A32NX_APU_AVAILABLE"),
            apu_bleed_air_valve_open: NamedVariable::from("A32NX_APU_BLEED_AIR_VALVE_OPEN"),
            apu_bleed_pb_on: NamedVariable::from("A32NX_APU_BLEED_PB_ON"),
            apu_egt: NamedVariable::from("A32NX_APU_EGT"),
            apu_egt_caution: NamedVariable::from("A32NX_APU_EGT_CAUTION"),
            apu_egt_warning: NamedVariable::from("A32NX_APU_EGT_WARNING"),
            apu_fire_button_released: NamedVariable::from("A32NX_FIRE_BUTTON_APU"),
            apu_air_intake_flap_is_ecam_open: NamedVariable::from("A32NX_APU_FLAP_ECAM_OPEN"),
            apu_flap_open_percentage: NamedVariable::from("A32NX_APU_FLAP_OPEN_PERCENTAGE"),
            apu_generator_pb_on: AircraftVariable::from("APU GENERATOR SWITCH", "Bool", 0)?,
            apu_inoperable: NamedVariable::from("A32NX_ECAM_INOP_SYS_APU"),
            apu_is_auto_shutdown: NamedVariable::from("A32NX_APU_IS_AUTO_SHUTDOWN"),
            apu_is_emergency_shutdown: NamedVariable::from("A32NX_APU_IS_EMERGENCY_SHUTDOWN"),
            apu_low_fuel_pressure_fault: NamedVariable::from("A32NX_APU_LOW_FUEL_PRESSURE_FAULT"),
            apu_master_sw_pb_on: NamedVariable::from("A32NX_APU_MASTER_SW_PB_ON"),
            apu_n: NamedVariable::from("A32NX_APU_N"),
            apu_start_contactor_energized: NamedVariable::from(
                "A32NX_APU_START_CONTACTOR_ENERGIZED",
            ),
            apu_start_pb_on: NamedVariable::from("A32NX_APU_START_PB_ON"),
            apu_start_pb_available: NamedVariable::from("A32NX_APU_START_PB_AVAILABLE"),
            elec_ac_ess_feed_pb_normal: NamedVariable::from("A32NX_ELEC_AC_ESS_FEED_PB_NORMAL"),
            elec_battery_1_pb_auto: NamedVariable::from("A32NX_ELEC_BATTERY_10_PB_AUTO"),
            elec_battery_2_pb_auto: NamedVariable::from("A32NX_ELEC_BATTERY_11_PB_AUTO"),
            elec_bus_tie_pb_auto: NamedVariable::from("A32NX_ELEC_BUS_TIE_PB_AUTO"),
            elec_commercial_pb_on: NamedVariable::from("A32NX_ELEC_COMMERCIAL_PB_ON"),
            elec_external_power_available: AircraftVariable::from(
                "EXTERNAL POWER AVAILABLE",
                "Bool",
                1,
            )?,
            elec_external_power_pb_on: AircraftVariable::from("EXTERNAL POWER ON", "Bool", 1)?,
            elec_galy_and_cab_pb_auto: NamedVariable::from("A32NX_ELEC_GALY_CAB_PB_AUTO"),
            elec_generator_1_pb_on: AircraftVariable::from(
                "GENERAL ENG MASTER ALTERNATOR",
                "Bool",
                1,
            )?,
            elec_generator_2_pb_on: AircraftVariable::from(
                "GENERAL ENG MASTER ALTERNATOR",
                "Bool",
                2,
            )?,
            elec_idg_1_pb_released: NamedVariable::from("A32NX_ELEC_IDG_1_PB_RELEASED"),
            elec_idg_2_pb_released: NamedVariable::from("A32NX_ELEC_IDG_2_PB_RELEASED"),
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
impl SimulatorReadWriter for A320SimulatorReadWriter {
    fn read(&mut self) -> SimulatorReadState {
        SimulatorReadState {
            ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(
                self.ambient_temperature.get(),
            ),
            apu: SimulatorApuReadState {
                master_sw_pb_on: to_bool(self.apu_master_sw_pb_on.get_value()),
                start_pb_on: to_bool(self.apu_start_pb_on.get_value()),
            },
            electrical: SimulatorElectricalReadState {
                apu_generator_pb_on: to_bool(self.apu_generator_pb_on.get()),
                ac_ess_feed_pb_normal: to_bool(self.elec_ac_ess_feed_pb_normal.get_value()),
                battery_pb_auto: [
                    to_bool(self.elec_battery_1_pb_auto.get_value()),
                    to_bool(self.elec_battery_2_pb_auto.get_value()),
                ],
                bus_tie_pb_auto: to_bool(self.elec_bus_tie_pb_auto.get_value()),
                galy_and_cab_pb_auto: to_bool(self.elec_galy_and_cab_pb_auto.get_value()),
                engine_generator_pb_on: [
                    to_bool(self.elec_generator_1_pb_on.get()),
                    to_bool(self.elec_generator_2_pb_on.get()),
                ],
                idg_pb_released: [
                    to_bool(self.elec_idg_1_pb_released.get_value()),
                    to_bool(self.elec_idg_2_pb_released.get_value()),
                ],
                commercial_pb_on: to_bool(self.elec_commercial_pb_on.get_value()),
                external_power_available: to_bool(self.elec_external_power_available.get()),
                external_power_pb_on: to_bool(self.elec_external_power_pb_on.get()),
            },
            fire: SimulatorFireReadState {
                apu_fire_button_released: to_bool(self.apu_fire_button_released.get_value()),
            },
            pneumatic: SimulatorPneumaticReadState {
                apu_bleed_pb_on: to_bool(self.apu_bleed_pb_on.get_value()),
            },
            engine_n2: [
                Ratio::new::<percent>(self.engine_1_n2.get()),
                Ratio::new::<percent>(self.engine_2_n2.get()),
            ],
            indicated_airspeed: Velocity::new::<knot>(self.indicated_airspeed.get()),
            indicated_altitude: Length::new::<foot>(self.indicated_altitude.get()),
            left_inner_tank_fuel_quantity: Mass::new::<pound>(
                self.left_inner_tank_fuel_quantity.get(),
            ),
            unlimited_fuel: to_bool(self.unlimited_fuel.get()),
        }
    }

    fn write(&mut self, state: &SimulatorWriteState) {
        self.apu_bleed_air_valve_open
            .set_value(from_bool(state.apu.bleed_air_valve_open));
        self.apu_egt
            .set_value(state.apu.egt.get::<degree_celsius>());
        self.apu_egt_caution
            .set_value(state.apu.caution_egt.get::<degree_celsius>());
        self.apu_egt_warning
            .set_value(state.apu.warning_egt.get::<degree_celsius>());
        self.apu_air_intake_flap_is_ecam_open
            .set_value(from_bool(state.apu.air_intake_flap_is_ecam_open));
        self.apu_flap_open_percentage
            .set_value(state.apu.air_intake_flap_opened_for.get::<percent>());
        self.apu_inoperable
            .set_value(from_bool(state.apu.inoperable));
        self.apu_is_auto_shutdown
            .set_value(from_bool(state.apu.is_auto_shutdown));
        self.apu_is_emergency_shutdown
            .set_value(from_bool(state.apu.is_emergency_shutdown));
        self.apu_low_fuel_pressure_fault
            .set_value(from_bool(state.apu.low_fuel_pressure_fault));
        self.apu_n.set_value(state.apu.n.get::<percent>());
        self.apu_start_contactor_energized
            .set_value(from_bool(state.apu.start_contactor_energized));
        self.apu_available.set_value(from_bool(state.apu.available));

        for variable in &state.variables {
            let named_variable = lookup_named_variable(
                &mut self.dynamic_named_variables,
                "A32NX_",
                variable.get_name(),
            );

            named_variable.set_value(variable.get_value());
        }
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
