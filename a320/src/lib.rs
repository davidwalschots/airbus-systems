#![cfg(any(target_arch = "wasm32", doc))]
use airbus_systems::{
    simulator::{
        from_bool, to_bool, Simulation, SimulatorReadState, SimulatorReadWriter,
        SimulatorWriteState,
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

pub struct A320SimulatorReadWriter {
    ambient_temperature: AircraftVariable,
    apu_bleed_air_valve_open: NamedVariable,
    apu_bleed_fault: NamedVariable,
    apu_bleed_on: NamedVariable,
    apu_egt: NamedVariable,
    apu_egt_caution: NamedVariable,
    apu_egt_warning: NamedVariable,
    apu_fire_button_released: NamedVariable,
    apu_air_intake_flap_is_ecam_open: NamedVariable,
    apu_flap_open_percentage: NamedVariable,
    apu_gen_amperage: NamedVariable,
    apu_gen_frequency: NamedVariable,
    apu_gen_frequency_within_normal_range: NamedVariable,
    apu_gen_on: AircraftVariable,
    apu_gen_voltage: NamedVariable,
    apu_gen_voltage_within_normal_range: NamedVariable,
    apu_inoperable: NamedVariable,
    apu_is_auto_shutdown: NamedVariable,
    apu_is_emergency_shutdown: NamedVariable,
    apu_low_fuel_pressure_fault: NamedVariable,
    apu_master_sw_on: NamedVariable,
    apu_master_sw_fault: NamedVariable,
    apu_n: NamedVariable,
    apu_start_contactor_energized: NamedVariable,
    apu_start_available: NamedVariable,
    apu_start_on: NamedVariable,
    elec_ac_ess_feed_activated: NamedVariable,
    elec_ac_ess_feed_fault: NamedVariable,
    elec_battery_1_fault: NamedVariable,
    elec_battery_2_fault: NamedVariable,
    elec_battery_1_activated: NamedVariable,
    elec_battery_2_activated: NamedVariable,
    elec_bus_tie_activated: NamedVariable,
    elec_commercial_activated: NamedVariable,
    elec_external_power_available: AircraftVariable,
    elec_external_power_on: AircraftVariable,
    elec_galy_and_cab_activated: NamedVariable,
    elec_galy_and_cab_fault: NamedVariable,
    elec_gen_1_activated: AircraftVariable,
    elec_gen_1_fault: NamedVariable,
    elec_gen_2_activated: AircraftVariable,
    elec_gen_2_fault: NamedVariable,
    elec_idg_1_fault: NamedVariable,
    elec_idg_1_activated: NamedVariable,
    elec_idg_2_fault: NamedVariable,
    elec_idg_2_activated: NamedVariable,
    engine_1_n2: AircraftVariable,
    engine_2_n2: AircraftVariable,
    indicated_airspeed: AircraftVariable,
    indicated_altitude: AircraftVariable,
    left_inner_tank_fuel_quantity: AircraftVariable,
    unlimited_fuel: AircraftVariable,
}
impl A320SimulatorReadWriter {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(A320SimulatorReadWriter {
            ambient_temperature: AircraftVariable::from("AMBIENT TEMPERATURE", "celsius", 0)?,
            apu_bleed_air_valve_open: NamedVariable::from("A32NX_APU_BLEED_AIR_VALVE_OPEN"),
            apu_bleed_fault: NamedVariable::from("A32NX_APU_BLEED_FAULT"),
            apu_bleed_on: NamedVariable::from("A32NX_APU_BLEED_ON"),
            apu_egt: NamedVariable::from("A32NX_APU_EGT"),
            apu_egt_caution: NamedVariable::from("A32NX_APU_EGT_CAUTION"),
            apu_egt_warning: NamedVariable::from("A32NX_APU_EGT_WARNING"),
            apu_fire_button_released: NamedVariable::from("A32NX_FIRE_BUTTON_APU"),
            apu_air_intake_flap_is_ecam_open: NamedVariable::from("A32NX_APU_FLAP_ECAM_OPEN"),
            apu_flap_open_percentage: NamedVariable::from("A32NX_APU_FLAP_OPEN_PERCENTAGE"),
            apu_gen_amperage: NamedVariable::from("A32NX_APU_GEN_AMPERAGE"),
            apu_gen_frequency: NamedVariable::from("A32NX_APU_GEN_FREQ"),
            apu_gen_frequency_within_normal_range: NamedVariable::from("A32NX_APU_GEN_FREQ_NORMAL"),
            apu_gen_on: AircraftVariable::from("APU GENERATOR SWITCH", "Bool", 0)?,
            apu_gen_voltage: NamedVariable::from("A32NX_APU_GEN_VOLTAGE"),
            apu_gen_voltage_within_normal_range: NamedVariable::from(
                "A32NX_APU_GEN_VOLTAGE_NORMAL",
            ),
            apu_inoperable: NamedVariable::from("A32NX_ECAM_INOP_SYS_APU"),
            apu_is_auto_shutdown: NamedVariable::from("A32NX_APU_IS_AUTO_SHUTDOWN"),
            apu_is_emergency_shutdown: NamedVariable::from("A32NX_APU_IS_EMERGENCY_SHUTDOWN"),
            apu_low_fuel_pressure_fault: NamedVariable::from("A32NX_APU_LOW_FUEL_PRESSURE_FAULT"),
            apu_master_sw_on: NamedVariable::from("A32NX_APU_MASTER_SW_ACTIVATED"),
            apu_master_sw_fault: NamedVariable::from("A32NX_APU_MASTER_FAULT"),
            apu_n: NamedVariable::from("A32NX_APU_N"),
            apu_start_contactor_energized: NamedVariable::from(
                "A32NX_APU_START_CONTACTOR_ENERGIZED",
            ),
            apu_start_available: NamedVariable::from("A32NX_APU_AVAILABLE"),
            apu_start_on: NamedVariable::from("A32NX_APU_START_ACTIVATED"),
            elec_ac_ess_feed_activated: NamedVariable::from("A32NX_ELEC_AC_ESS_FEED_ACTIVATED"),
            elec_ac_ess_feed_fault: NamedVariable::from("A32NX_ELEC_AC_ESS_FEED_FAULT"),
            elec_battery_1_activated: NamedVariable::from("A32NX_ELEC_BATTERY_10_ACTIVATED"),
            elec_battery_1_fault: NamedVariable::from("A32NX_ELEC_BATTERY_10_FAULT"),
            elec_battery_2_activated: NamedVariable::from("A32NX_ELEC_BATTERY_11_ACTIVATED"),
            elec_battery_2_fault: NamedVariable::from("A32NX_ELEC_BATTERY_11_FAULT"),
            elec_bus_tie_activated: NamedVariable::from("A32NX_ELEC_BUS_TIE_ACTIVATED"),
            elec_commercial_activated: NamedVariable::from("A32NX_ELEC_COMMERCIAL_ACTIVATED"),
            elec_external_power_available: AircraftVariable::from(
                "EXTERNAL POWER AVAILABLE",
                "Bool",
                1,
            )?,
            elec_external_power_on: AircraftVariable::from("EXTERNAL POWER ON", "Bool", 1)?,
            elec_galy_and_cab_activated: NamedVariable::from("A32NX_ELEC_GALY_CAB_ACTIVATED"),
            elec_galy_and_cab_fault: NamedVariable::from("A32NX_ELEC_GALY_CAB_FAULT"),
            elec_gen_1_activated: AircraftVariable::from(
                "GENERAL ENG MASTER ALTERNATOR",
                "Bool",
                1,
            )?,
            elec_gen_1_fault: NamedVariable::from("A32NX_ELEC_GEN_1_FAULT"),
            elec_gen_2_activated: AircraftVariable::from(
                "GENERAL ENG MASTER ALTERNATOR",
                "Bool",
                2,
            )?,
            elec_gen_2_fault: NamedVariable::from("A32NX_ELEC_GEN_2_FAULT"),
            elec_idg_1_fault: NamedVariable::from("A32NX_ELEC_IDG_1_FAULT"),
            elec_idg_1_activated: NamedVariable::from("A32NX_ELEC_IDG_1_ACTIVATED"),
            elec_idg_2_fault: NamedVariable::from("A32NX_ELEC_IDG_2_FAULT"),
            elec_idg_2_activated: NamedVariable::from("A32NX_ELEC_IDG_2_ACTIVATED"),
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
    fn read(&self) -> SimulatorReadState {
        SimulatorReadState {
            ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(
                self.ambient_temperature.get(),
            ),
            apu_bleed_on: to_bool(self.apu_bleed_on.get_value()),
            apu_fire_button_released: to_bool(self.apu_fire_button_released.get_value()),
            apu_gen_on: to_bool(self.apu_gen_on.get()),
            apu_master_sw_on: to_bool(self.apu_master_sw_on.get_value()),
            apu_start_on: to_bool(self.apu_start_on.get_value()),
            elec_ac_ess_feed_activated: to_bool(self.elec_ac_ess_feed_activated.get_value()),
            elec_battery_1_activated: to_bool(self.elec_battery_1_activated.get_value()),
            elec_battery_2_activated: to_bool(self.elec_battery_2_activated.get_value()),
            elec_bus_tie_activated: to_bool(self.elec_bus_tie_activated.get_value()),
            elec_galy_and_cab_activated: to_bool(self.elec_galy_and_cab_activated.get_value()),
            elec_gen_1_activated: to_bool(self.elec_gen_1_activated.get()),
            elec_gen_2_activated: to_bool(self.elec_gen_2_activated.get()),
            elec_idg_1_activated: to_bool(self.elec_idg_1_activated.get_value()),
            elec_idg_2_activated: to_bool(self.elec_idg_2_activated.get_value()),
            engine_n2: [
                Ratio::new::<percent>(self.engine_1_n2.get()),
                Ratio::new::<percent>(self.engine_2_n2.get()),
            ],
            elec_commercial_activated: to_bool(self.elec_commercial_activated.get_value()),
            elec_external_power_available: to_bool(self.elec_external_power_available.get()),
            elec_external_power_on: to_bool(self.elec_external_power_on.get()),
            indicated_airspeed: Velocity::new::<knot>(self.indicated_airspeed.get()),
            indicated_altitude: Length::new::<foot>(self.indicated_altitude.get()),
            left_inner_tank_fuel_quantity: Mass::new::<pound>(
                self.left_inner_tank_fuel_quantity.get(),
            ),
            unlimited_fuel: to_bool(self.unlimited_fuel.get()),
        }
    }

    fn write(&self, state: &SimulatorWriteState) {
        self.apu_bleed_air_valve_open
            .set_value(from_bool(state.apu_bleed_air_valve_open));
        self.apu_bleed_fault
            .set_value(from_bool(state.apu_bleed_fault));
        self.apu_egt
            .set_value(state.apu_egt.get::<degree_celsius>());
        self.apu_egt_caution
            .set_value(state.apu_caution_egt.get::<degree_celsius>());
        self.apu_egt_warning
            .set_value(state.apu_warning_egt.get::<degree_celsius>());
        self.apu_air_intake_flap_is_ecam_open
            .set_value(from_bool(state.apu_air_intake_flap_is_ecam_open));
        self.apu_flap_open_percentage
            .set_value(state.apu_air_intake_flap_opened_for.get::<percent>());
        self.apu_gen_amperage
            .set_value(state.apu_gen_current.get::<ampere>());
        self.apu_gen_frequency
            .set_value(state.apu_gen_frequency.get::<hertz>());
        self.apu_gen_frequency_within_normal_range
            .set_value(from_bool(state.apu_gen_frequency_within_normal_range));
        self.apu_gen_voltage
            .set_value(state.apu_gen_potential.get::<volt>());
        self.apu_gen_voltage_within_normal_range
            .set_value(from_bool(state.apu_gen_potential_within_normal_range));
        self.apu_inoperable
            .set_value(from_bool(state.apu_inoperable));
        self.apu_is_auto_shutdown
            .set_value(from_bool(state.apu_is_auto_shutdown));
        self.apu_is_emergency_shutdown
            .set_value(from_bool(state.apu_is_emergency_shutdown));
        self.apu_low_fuel_pressure_fault
            .set_value(from_bool(state.apu_low_fuel_pressure_fault));
        self.apu_master_sw_fault
            .set_value(from_bool(state.apu_master_sw_fault));
        self.apu_n.set_value(state.apu_n.get::<percent>());
        self.apu_start_contactor_energized
            .set_value(from_bool(state.apu_start_contactor_energized));
        self.apu_start_available
            .set_value(from_bool(state.apu_start_available));
        self.apu_start_on.set_value(from_bool(state.apu_start_on));
        self.elec_ac_ess_feed_fault
            .set_value(from_bool(state.elec_ac_ess_feed_fault));
        self.elec_battery_1_fault
            .set_value(from_bool(state.elec_battery_1_fault));
        self.elec_battery_2_fault
            .set_value(from_bool(state.elec_battery_2_fault));
        self.elec_galy_and_cab_fault
            .set_value(from_bool(state.elec_galy_and_cab_fault));
        self.elec_gen_1_fault
            .set_value(from_bool(state.elec_gen_1_fault));
        self.elec_gen_2_fault
            .set_value(from_bool(state.elec_gen_2_fault));
        self.elec_idg_1_fault
            .set_value(from_bool(state.elec_idg_1_fault));
        self.elec_idg_2_fault
            .set_value(from_bool(state.elec_idg_2_fault));
    }
}
