use std::borrow::Borrow;

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
    ambient_temperature: AircraftVariable,
    apu_available: NamedVariable,
    apu_bleed_air_valve_open: NamedVariable,
    apu_bleed_pb_fault: NamedVariable,
    apu_bleed_pb_on: NamedVariable,
    apu_egt: NamedVariable,
    apu_egt_caution: NamedVariable,
    apu_egt_warning: NamedVariable,
    apu_fire_button_released: NamedVariable,
    apu_air_intake_flap_is_ecam_open: NamedVariable,
    apu_flap_open_percentage: NamedVariable,
    apu_gen_frequency: NamedVariable,
    apu_gen_frequency_within_normal_range: NamedVariable,
    apu_gen_load: NamedVariable,
    apu_gen_load_within_normal_range: NamedVariable,
    apu_generator_pb_on: AircraftVariable,
    apu_gen_potential: NamedVariable,
    apu_gen_potential_within_normal_range: NamedVariable,
    apu_inoperable: NamedVariable,
    apu_is_auto_shutdown: NamedVariable,
    apu_is_emergency_shutdown: NamedVariable,
    apu_low_fuel_pressure_fault: NamedVariable,
    apu_master_sw_pb_on: NamedVariable,
    apu_master_sw_pb_fault: NamedVariable,
    apu_n: NamedVariable,
    apu_start_contactor_energized: NamedVariable,
    apu_start_pb_on: NamedVariable,
    apu_start_pb_available: NamedVariable,
    elec_ac_bus_1_is_powered: NamedVariable,
    elec_ac_bus_2_is_powered: NamedVariable,
    elec_ac_bus_tie_contactor_1_closed: NamedVariable,
    elec_ac_bus_tie_contactor_2_closed: NamedVariable,
    elec_ac_ess_bus_is_powered: NamedVariable,
    elec_ac_ess_feed_contactor_1_closed: NamedVariable,
    elec_ac_ess_feed_contactor_2_closed: NamedVariable,
    elec_ac_ess_feed_pb_normal: NamedVariable,
    elec_ac_ess_feed_pb_fault: NamedVariable,
    elec_apu_generator_contactor_closed: NamedVariable,
    elec_battery_contactor_1_closed: NamedVariable,
    elec_battery_contactor_2_closed: NamedVariable,
    elec_battery_1_current: NamedVariable,
    elec_battery_1_current_within_normal_range: NamedVariable,
    elec_battery_1_pb_auto: NamedVariable,
    elec_battery_1_pb_fault: NamedVariable,
    elec_battery_1_potential: NamedVariable,
    elec_battery_1_potential_within_normal_range: NamedVariable,
    elec_battery_2_current: NamedVariable,
    elec_battery_2_current_within_normal_range: NamedVariable,
    elec_battery_2_pb_auto: NamedVariable,
    elec_battery_2_pb_fault: NamedVariable,
    elec_battery_2_potential: NamedVariable,
    elec_battery_2_potential_within_normal_range: NamedVariable,
    elec_bus_tie_pb_auto: NamedVariable,
    elec_commercial_pb_on: NamedVariable,
    elec_dc_bat_bus_is_powered: NamedVariable,
    elec_dc_bus_1_is_powered: NamedVariable,
    elec_dc_bus_2_is_powered: NamedVariable,
    elec_dc_bus_tie_contactor_1_closed: NamedVariable,
    elec_dc_bus_tie_contactor_2_closed: NamedVariable,
    elec_dc_ess_bus_is_powered: NamedVariable,
    elec_emergency_generator_frequency: NamedVariable,
    elec_emergency_generator_frequency_within_normal_range: NamedVariable,
    elec_emergency_generator_potential: NamedVariable,
    elec_emergency_generator_potential_within_normal_range: NamedVariable,
    elec_external_power_available: AircraftVariable,
    elec_external_power_contactor_closed: NamedVariable,
    elec_external_power_frequency: NamedVariable,
    elec_external_power_frequency_within_normal_range: NamedVariable,
    elec_external_power_pb_on: AircraftVariable,
    elec_external_power_potential: NamedVariable,
    elec_external_power_potential_within_normal_range: NamedVariable,
    elec_galy_and_cab_pb_auto: NamedVariable,
    elec_galy_and_cab_pb_fault: NamedVariable,
    elec_generator_1_pb_fault: NamedVariable,
    elec_generator_1_pb_on: AircraftVariable,
    elec_generator_1_frequency: NamedVariable,
    elec_generator_1_frequency_within_normal_range: NamedVariable,
    elec_generator_1_load: NamedVariable,
    elec_generator_1_load_within_normal_range: NamedVariable,
    elec_generator_1_potential: NamedVariable,
    elec_generator_1_potential_within_normal_range: NamedVariable,
    elec_generator_2_pb_fault: NamedVariable,
    elec_generator_2_pb_on: AircraftVariable,
    elec_generator_2_frequency: NamedVariable,
    elec_generator_2_frequency_within_normal_range: NamedVariable,
    elec_generator_2_load: NamedVariable,
    elec_generator_2_load_within_normal_range: NamedVariable,
    elec_generator_2_potential: NamedVariable,
    elec_generator_2_potential_within_normal_range: NamedVariable,
    elec_generator_line_contactor_1_closed: NamedVariable,
    elec_generator_line_contactor_2_closed: NamedVariable,
    elec_idg_1_pb_fault: NamedVariable,
    elec_idg_1_pb_released: NamedVariable,
    elec_idg_2_pb_fault: NamedVariable,
    elec_idg_2_pb_released: NamedVariable,
    elec_static_inverter_frequency: NamedVariable,
    elec_static_inverter_frequency_within_normal_range: NamedVariable,
    elec_static_inverter_potential: NamedVariable,
    elec_static_inverter_potential_within_normal_range: NamedVariable,
    elec_tr_1_current: NamedVariable,
    elec_tr_1_current_within_normal_range: NamedVariable,
    elec_tr_1_contactor_closed: NamedVariable,
    elec_tr_1_potential: NamedVariable,
    elec_tr_1_potential_within_normal_range: NamedVariable,
    elec_tr_2_current: NamedVariable,
    elec_tr_2_current_within_normal_range: NamedVariable,
    elec_tr_2_contactor_closed: NamedVariable,
    elec_tr_2_potential: NamedVariable,
    elec_tr_2_potential_within_normal_range: NamedVariable,
    elec_tr_ess_contactor_closed: NamedVariable,
    elec_tr_ess_current: NamedVariable,
    elec_tr_ess_current_within_normal_range: NamedVariable,
    elec_tr_ess_potential: NamedVariable,
    elec_tr_ess_potential_within_normal_range: NamedVariable,
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
            ambient_temperature: AircraftVariable::from("AMBIENT TEMPERATURE", "celsius", 0)?,
            apu_available: NamedVariable::from("A32NX_APU_AVAILABLE"),
            apu_bleed_air_valve_open: NamedVariable::from("A32NX_APU_BLEED_AIR_VALVE_OPEN"),
            apu_bleed_pb_fault: NamedVariable::from("A32NX_APU_BLEED_PB_FAULT"),
            apu_bleed_pb_on: NamedVariable::from("A32NX_APU_BLEED_PB_ON"),
            apu_egt: NamedVariable::from("A32NX_APU_EGT"),
            apu_egt_caution: NamedVariable::from("A32NX_APU_EGT_CAUTION"),
            apu_egt_warning: NamedVariable::from("A32NX_APU_EGT_WARNING"),
            apu_fire_button_released: NamedVariable::from("A32NX_FIRE_BUTTON_APU"),
            apu_air_intake_flap_is_ecam_open: NamedVariable::from("A32NX_APU_FLAP_ECAM_OPEN"),
            apu_flap_open_percentage: NamedVariable::from("A32NX_APU_FLAP_OPEN_PERCENTAGE"),
            apu_gen_frequency: NamedVariable::from("A32NX_APU_GEN_FREQ"),
            apu_gen_frequency_within_normal_range: NamedVariable::from("A32NX_APU_GEN_FREQ_NORMAL"),
            apu_gen_load: NamedVariable::from("A32NX_APU_GEN_LOAD"),
            apu_gen_load_within_normal_range: NamedVariable::from("A32NX_APU_GEN_LOAD_NORMAL"),
            apu_generator_pb_on: AircraftVariable::from("APU GENERATOR SWITCH", "Bool", 0)?,
            apu_gen_potential: NamedVariable::from("A32NX_APU_GEN_POTENTIAL"),
            apu_gen_potential_within_normal_range: NamedVariable::from(
                "A32NX_APU_GEN_POTENTIAL_NORMAL",
            ),
            apu_inoperable: NamedVariable::from("A32NX_ECAM_INOP_SYS_APU"),
            apu_is_auto_shutdown: NamedVariable::from("A32NX_APU_IS_AUTO_SHUTDOWN"),
            apu_is_emergency_shutdown: NamedVariable::from("A32NX_APU_IS_EMERGENCY_SHUTDOWN"),
            apu_low_fuel_pressure_fault: NamedVariable::from("A32NX_APU_LOW_FUEL_PRESSURE_FAULT"),
            apu_master_sw_pb_on: NamedVariable::from("A32NX_APU_MASTER_SW_PB_ON"),
            apu_master_sw_pb_fault: NamedVariable::from("A32NX_APU_MASTER_SW_PB_FAULT"),
            apu_n: NamedVariable::from("A32NX_APU_N"),
            apu_start_contactor_energized: NamedVariable::from(
                "A32NX_APU_START_CONTACTOR_ENERGIZED",
            ),
            apu_start_pb_on: NamedVariable::from("A32NX_APU_START_PB_ON"),
            apu_start_pb_available: NamedVariable::from("A32NX_APU_START_PB_AVAILABLE"),
            elec_ac_bus_1_is_powered: NamedVariable::from("A32NX_ELEC_AC_BUS_1_IS_POWERED"),
            elec_ac_bus_2_is_powered: NamedVariable::from("A32NX_ELEC_AC_BUS_2_IS_POWERED"),
            elec_ac_bus_tie_contactor_1_closed: NamedVariable::from(
                "A32NX_ELEC_AC_BUS_TIE_CONTACTOR_1_CLOSED",
            ),
            elec_ac_bus_tie_contactor_2_closed: NamedVariable::from(
                "A32NX_ELEC_AC_BUS_TIE_CONTACTOR_2_CLOSED",
            ),
            elec_ac_ess_bus_is_powered: NamedVariable::from("A32NX_ELEC_AC_ESS_BUS_IS_POWERED"),
            elec_ac_ess_feed_contactor_1_closed: NamedVariable::from(
                "A32NX_ELEC_AC_ESS_FEED_CONTACTOR_1_CLOSED",
            ),
            elec_ac_ess_feed_contactor_2_closed: NamedVariable::from(
                "A32NX_ELEC_AC_ESS_FEED_CONTACTOR_2_CLOSED",
            ),
            elec_ac_ess_feed_pb_normal: NamedVariable::from("A32NX_ELEC_AC_ESS_FEED_PB_NORMAL"),
            elec_ac_ess_feed_pb_fault: NamedVariable::from("A32NX_ELEC_AC_ESS_FEED_PB_FAULT"),
            elec_apu_generator_contactor_closed: NamedVariable::from(
                "A32NX_ELEC_APU_GENERATOR_CONTACTOR_CLOSED",
            ),
            elec_battery_contactor_1_closed: NamedVariable::from(
                "A32NX_ELEC_BATTERY_CONTACTOR_1_CLOSED",
            ),
            elec_battery_contactor_2_closed: NamedVariable::from(
                "A32NX_ELEC_BATTERY_CONTACTOR_2_CLOSED",
            ),
            elec_battery_1_current: NamedVariable::from("A32NX_ELEC_BATTERY_1_CURRENT"),
            elec_battery_1_current_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_BATTERY_1_CURRENT_NORMAL",
            ),
            elec_battery_1_pb_auto: NamedVariable::from("A32NX_ELEC_BATTERY_10_PB_AUTO"),
            elec_battery_1_pb_fault: NamedVariable::from("A32NX_ELEC_BATTERY_10_PB_FAULT"),
            elec_battery_1_potential: NamedVariable::from("A32NX_ELEC_BATTERY_1_POTENTIAL"),
            elec_battery_1_potential_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_BATTERY_1_POTENTIAL_NORMAL",
            ),
            elec_battery_2_current: NamedVariable::from("A32NX_ELEC_BATTERY_2_CURRENT"),
            elec_battery_2_current_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_BATTERY_2_CURRENT_NORMAL",
            ),
            elec_battery_2_pb_auto: NamedVariable::from("A32NX_ELEC_BATTERY_11_PB_AUTO"),
            elec_battery_2_pb_fault: NamedVariable::from("A32NX_ELEC_BATTERY_11_PB_FAULT"),
            elec_battery_2_potential: NamedVariable::from("A32NX_ELEC_BATTERY_2_POTENTIAL"),
            elec_battery_2_potential_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_BATTERY_2_POTENTIAL_NORMAL",
            ),
            elec_bus_tie_pb_auto: NamedVariable::from("A32NX_ELEC_BUS_TIE_PB_AUTO"),
            elec_commercial_pb_on: NamedVariable::from("A32NX_ELEC_COMMERCIAL_PB_ON"),
            elec_dc_bat_bus_is_powered: NamedVariable::from("A32NX_ELEC_DC_BAT_BUS_IS_POWERED"),
            elec_dc_bus_1_is_powered: NamedVariable::from("A32NX_ELEC_DC_BUS_1_IS_POWERED"),
            elec_dc_bus_2_is_powered: NamedVariable::from("A32NX_ELEC_DC_BUS_2_IS_POWERED"),
            elec_dc_bus_tie_contactor_1_closed: NamedVariable::from(
                "A32NX_DC_BUS_TIE_CONTACTOR_1_CLOSED",
            ),
            elec_dc_bus_tie_contactor_2_closed: NamedVariable::from(
                "A32NX_DC_BUS_TIE_CONTACTOR_2_CLOSED",
            ),
            elec_dc_ess_bus_is_powered: NamedVariable::from("A32NX_ELEC_DC_ESS_BUS_IS_POWERED"),
            elec_emergency_generator_frequency: NamedVariable::from(
                "A32NX_ELEC_EMERGENCY_GENERATOR_FREQ",
            ),
            elec_emergency_generator_frequency_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_EMERGENCY_GENERATOR_FREQ_NORMAL",
            ),
            elec_emergency_generator_potential: NamedVariable::from(
                "A32NX_ELEC_EMERGENCY_GENERATOR_POTENTIAL",
            ),
            elec_emergency_generator_potential_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_EMERGENCY_GENERATOR_POTENTIAL_NORMAL",
            ),
            elec_external_power_available: AircraftVariable::from(
                "EXTERNAL POWER AVAILABLE",
                "Bool",
                1,
            )?,
            elec_external_power_contactor_closed: NamedVariable::from(
                "A32NX_ELEC_EXTERNAL_POWER_CONTACTOR_CLOSED",
            ),
            elec_external_power_frequency: NamedVariable::from("A32NX_ELEC_EXTERNAL_POWER_FREQ"),
            elec_external_power_frequency_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_EXTERNAL_POWER_FREQ_NORMAL",
            ),
            elec_external_power_pb_on: AircraftVariable::from("EXTERNAL POWER ON", "Bool", 1)?,
            elec_external_power_potential: NamedVariable::from(
                "A32NX_ELEC_EXTERNAL_POWER_POTENTIAL",
            ),
            elec_external_power_potential_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_EXTERNAL_POWER_POTENTIAL_NORMAL",
            ),
            elec_galy_and_cab_pb_auto: NamedVariable::from("A32NX_ELEC_GALY_CAB_PB_AUTO"),
            elec_galy_and_cab_pb_fault: NamedVariable::from("A32NX_ELEC_GALY_CAB_PB_FAULT"),
            elec_generator_1_pb_fault: NamedVariable::from("A32NX_ELEC_GEN_1_PB_FAULT"),
            elec_generator_1_pb_on: AircraftVariable::from(
                "GENERAL ENG MASTER ALTERNATOR",
                "Bool",
                1,
            )?,
            elec_generator_1_frequency: NamedVariable::from("A32NX_ELEC_GEN_1_FREQ"),
            elec_generator_1_frequency_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_GEN_1_FREQ_NORMAL",
            ),
            elec_generator_1_load: NamedVariable::from("A32NX_ELEC_GEN_1_LOAD"),
            elec_generator_1_load_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_GEN_1_LOAD_NORMAL",
            ),
            elec_generator_1_potential: NamedVariable::from("A32NX_ELEC_GEN_1_POTENTIAL"),
            elec_generator_1_potential_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_GEN_1_POTENTIAL_NORMAL",
            ),
            elec_generator_2_pb_fault: NamedVariable::from("A32NX_ELEC_GEN_2_PB_FAULT"),
            elec_generator_2_pb_on: AircraftVariable::from(
                "GENERAL ENG MASTER ALTERNATOR",
                "Bool",
                2,
            )?,
            elec_generator_2_frequency: NamedVariable::from("A32NX_ELEC_GEN_2_FREQ"),
            elec_generator_2_frequency_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_GEN_2_FREQ_NORMAL",
            ),
            elec_generator_2_load: NamedVariable::from("A32NX_ELEC_GEN_2_LOAD"),
            elec_generator_2_load_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_GEN_2_LOAD_NORMAL",
            ),
            elec_generator_2_potential: NamedVariable::from("A32NX_ELEC_GEN_2_POTENTIAL"),
            elec_generator_2_potential_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_GEN_2_POTENTIAL_NORMAL",
            ),
            elec_generator_line_contactor_1_closed: NamedVariable::from(
                "A32NX_ELEC_GENERATOR_LINE_CONTACTOR_1_CLOSED",
            ),
            elec_generator_line_contactor_2_closed: NamedVariable::from(
                "A32NX_ELEC_GENERATOR_LINE_CONTACTOR_2_CLOSED",
            ),
            elec_idg_1_pb_fault: NamedVariable::from("A32NX_ELEC_IDG_1_PB_FAULT"),
            elec_idg_1_pb_released: NamedVariable::from("A32NX_ELEC_IDG_1_PB_RELEASED"),
            elec_idg_2_pb_fault: NamedVariable::from("A32NX_ELEC_IDG_2_PB_FAULT"),
            elec_idg_2_pb_released: NamedVariable::from("A32NX_ELEC_IDG_2_PB_RELEASED"),
            elec_static_inverter_frequency: NamedVariable::from("A32NX_ELEC_STATIC_INVERTER_FREQ"),
            elec_static_inverter_frequency_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_STATIC_INVERTER_FREQ_NORMAL",
            ),
            elec_static_inverter_potential: NamedVariable::from(
                "A32NX_ELEC_STATIC_INVERTER_POTENTIAL",
            ),
            elec_static_inverter_potential_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_STATIC_INVERTER_POTENTIAL_NORMAL",
            ),
            elec_tr_1_current: NamedVariable::from("A32NX_ELEC_TRANSFORMER_RECTIFIER_1_CURRENT"),
            elec_tr_1_current_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_TRANSFORMER_RECTIFIER_1_CURRENT_NORMAL",
            ),
            elec_tr_1_contactor_closed: NamedVariable::from("A32NX_ELEC_TR_1_CONTACTOR_CLOSED"),
            elec_tr_1_potential: NamedVariable::from(
                "A32NX_ELEC_TRANSFORMER_RECTIFIER_1_POTENTIAL",
            ),
            elec_tr_1_potential_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_TRANSFORMER_RECTIFIER_1_POTENTIAL_NORMAL",
            ),
            elec_tr_2_current: NamedVariable::from("A32NX_ELEC_TRANSFORMER_RECTIFIER_2_CURRENT"),
            elec_tr_2_current_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_TRANSFORMER_RECTIFIER_2_CURRENT_NORMAL",
            ),
            elec_tr_2_contactor_closed: NamedVariable::from("A32NX_ELEC_TR_2_CONTACTOR_CLOSED"),
            elec_tr_2_potential: NamedVariable::from(
                "A32NX_ELEC_TRANSFORMER_RECTIFIER_2_POTENTIAL",
            ),
            elec_tr_2_potential_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_TRANSFORMER_RECTIFIER_2_POTENTIAL_NORMAL",
            ),
            elec_tr_ess_current: NamedVariable::from(
                "A32NX_ELEC_ESS_TRANSFORMER_RECTIFIER_CURRENT",
            ),
            elec_tr_ess_current_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_ESS_TRANSFORMER_RECTIFIER_CURRENT_NORMAL",
            ),
            elec_tr_ess_contactor_closed: NamedVariable::from("A32NX_ELEC_TR_ESS_CONTACTOR_CLOSED"),
            elec_tr_ess_potential: NamedVariable::from(
                "A32NX_ELEC_ESS_TRANSFORMER_RECTIFIER_POTENTIAL",
            ),
            elec_tr_ess_potential_within_normal_range: NamedVariable::from(
                "A32NX_ELEC_ESS_TRANSFORMER_RECTIFIER_POTENTIAL_NORMAL",
            ),
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

    fn write(&self, state: &SimulatorWriteState) {
        self.apu_bleed_air_valve_open
            .set_value(from_bool(state.apu.bleed_air_valve_open));
        self.apu_bleed_pb_fault
            .set_value(from_bool(state.pneumatic.apu_bleed_pb_fault));
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
        self.apu_gen_frequency
            .set_value(state.apu.generator.frequency.get::<hertz>());
        self.apu_gen_frequency_within_normal_range
            .set_value(from_bool(state.apu.generator.frequency_within_normal_range));
        self.apu_gen_load
            .set_value(state.apu.generator.load.get::<percent>());
        self.apu_gen_load_within_normal_range
            .set_value(from_bool(state.apu.generator.load_within_normal_range));
        self.apu_gen_potential
            .set_value(state.apu.generator.potential.get::<volt>());
        self.apu_gen_potential_within_normal_range
            .set_value(from_bool(state.apu.generator.potential_within_normal_range));
        self.apu_inoperable
            .set_value(from_bool(state.apu.inoperable));
        self.apu_is_auto_shutdown
            .set_value(from_bool(state.apu.is_auto_shutdown));
        self.apu_is_emergency_shutdown
            .set_value(from_bool(state.apu.is_emergency_shutdown));
        self.apu_low_fuel_pressure_fault
            .set_value(from_bool(state.apu.low_fuel_pressure_fault));
        self.apu_master_sw_pb_fault
            .set_value(from_bool(state.apu.master_sw_pb_fault));
        self.apu_n.set_value(state.apu.n.get::<percent>());
        self.apu_start_contactor_energized
            .set_value(from_bool(state.apu.start_contactor_energized));
        self.apu_available.set_value(from_bool(state.apu.available));
        self.apu_start_pb_available
            .set_value(from_bool(state.apu.start_pb_available));
        self.apu_start_pb_on
            .set_value(from_bool(state.apu.start_pb_on));
        self.elec_ac_bus_1_is_powered
            .set_value(from_bool(state.electrical.ac_bus_is_powered[0]));
        self.elec_ac_bus_2_is_powered
            .set_value(from_bool(state.electrical.ac_bus_is_powered[1]));
        self.elec_ac_ess_bus_is_powered
            .set_value(from_bool(state.electrical.ac_ess_bus_is_powered));
        self.elec_ac_ess_feed_contactor_1_closed
            .set_value(from_bool(state.electrical.ac_ess_feed_contactor_closed[0]));
        self.elec_ac_ess_feed_contactor_2_closed
            .set_value(from_bool(state.electrical.ac_ess_feed_contactor_closed[1]));
        self.elec_ac_ess_feed_pb_fault
            .set_value(from_bool(state.electrical.ac_ess_feed_pb_fault));
        self.elec_apu_generator_contactor_closed
            .set_value(from_bool(state.electrical.apu_generator_contactor_closed));
        self.elec_battery_1_current
            .set_value(state.electrical.batteries[0].current.get::<ampere>());
        self.elec_battery_1_current_within_normal_range
            .set_value(from_bool(
                state.electrical.batteries[0].current_within_normal_range,
            ));
        self.elec_battery_1_pb_fault
            .set_value(from_bool(state.electrical.battery_pb_fault[0]));
        self.elec_battery_1_potential
            .set_value(state.electrical.batteries[0].potential.get::<volt>());
        self.elec_battery_1_potential_within_normal_range
            .set_value(from_bool(
                state.electrical.batteries[0].potential_within_normal_range,
            ));
        self.elec_battery_2_current
            .set_value(state.electrical.batteries[1].current.get::<ampere>());
        self.elec_battery_2_current_within_normal_range
            .set_value(from_bool(
                state.electrical.batteries[1].current_within_normal_range,
            ));
        self.elec_battery_2_pb_fault
            .set_value(from_bool(state.electrical.battery_pb_fault[1]));
        self.elec_battery_2_potential
            .set_value(state.electrical.batteries[1].potential.get::<volt>());
        self.elec_battery_2_potential_within_normal_range
            .set_value(from_bool(
                state.electrical.batteries[1].potential_within_normal_range,
            ));
        self.elec_battery_contactor_1_closed
            .set_value(from_bool(state.electrical.battery_contactor_closed[0]));
        self.elec_battery_contactor_2_closed
            .set_value(from_bool(state.electrical.battery_contactor_closed[1]));
        self.elec_ac_bus_tie_contactor_1_closed
            .set_value(from_bool(state.electrical.ac_bus_tie_contactor_closed[0]));
        self.elec_ac_bus_tie_contactor_2_closed
            .set_value(from_bool(state.electrical.ac_bus_tie_contactor_closed[1]));
        self.elec_dc_bat_bus_is_powered
            .set_value(from_bool(state.electrical.dc_bat_bus_is_powered));
        self.elec_dc_bus_1_is_powered
            .set_value(from_bool(state.electrical.dc_bus_is_powered[0]));
        self.elec_dc_bus_2_is_powered
            .set_value(from_bool(state.electrical.dc_bus_is_powered[1]));
        self.elec_dc_bus_tie_contactor_1_closed
            .set_value(from_bool(state.electrical.dc_bus_tie_contactor_closed[0]));
        self.elec_dc_bus_tie_contactor_2_closed
            .set_value(from_bool(state.electrical.dc_bus_tie_contactor_closed[1]));
        self.elec_dc_ess_bus_is_powered
            .set_value(from_bool(state.electrical.dc_ess_bus_is_powered));
        self.elec_emergency_generator_frequency.set_value(
            state
                .electrical
                .emergency_generator
                .frequency
                .get::<hertz>(),
        );
        self.elec_emergency_generator_frequency_within_normal_range
            .set_value(from_bool(
                state
                    .electrical
                    .emergency_generator
                    .frequency_within_normal_range,
            ));
        self.elec_emergency_generator_potential
            .set_value(state.electrical.emergency_generator.potential.get::<volt>());
        self.elec_emergency_generator_potential_within_normal_range
            .set_value(from_bool(
                state
                    .electrical
                    .emergency_generator
                    .potential_within_normal_range,
            ));
        self.elec_external_power_contactor_closed
            .set_value(from_bool(state.electrical.external_power_contactor_closed));
        self.elec_external_power_frequency
            .set_value(state.electrical.external_power.frequency.get::<hertz>());
        self.elec_external_power_frequency_within_normal_range
            .set_value(from_bool(
                state
                    .electrical
                    .external_power
                    .frequency_within_normal_range,
            ));
        self.elec_external_power_potential
            .set_value(state.electrical.external_power.potential.get::<volt>());
        self.elec_external_power_potential_within_normal_range
            .set_value(from_bool(
                state
                    .electrical
                    .external_power
                    .potential_within_normal_range,
            ));
        self.elec_galy_and_cab_pb_fault
            .set_value(from_bool(state.electrical.galy_and_cab_pb_fault));
        self.elec_generator_1_pb_fault
            .set_value(from_bool(state.electrical.generator_pb_fault[0]));
        self.elec_generator_1_frequency.set_value(
            state.electrical.engine_generator[0]
                .frequency
                .get::<hertz>(),
        );
        self.elec_generator_1_frequency_within_normal_range
            .set_value(from_bool(
                state.electrical.engine_generator[0].frequency_within_normal_range,
            ));
        self.elec_generator_1_load
            .set_value(state.electrical.engine_generator[0].load.get::<percent>());
        self.elec_generator_1_load_within_normal_range
            .set_value(from_bool(
                state.electrical.engine_generator[0].load_within_normal_range,
            ));
        self.elec_generator_1_potential
            .set_value(state.electrical.engine_generator[0].potential.get::<volt>());
        self.elec_generator_1_potential_within_normal_range
            .set_value(from_bool(
                state.electrical.engine_generator[0].potential_within_normal_range,
            ));
        self.elec_generator_line_contactor_1_closed
            .set_value(from_bool(
                state.electrical.engine_generator_line_contactor_closed[0],
            ));
        self.elec_generator_2_pb_fault
            .set_value(from_bool(state.electrical.generator_pb_fault[1]));
        self.elec_generator_2_frequency.set_value(
            state.electrical.engine_generator[1]
                .frequency
                .get::<hertz>(),
        );
        self.elec_generator_2_frequency_within_normal_range
            .set_value(from_bool(
                state.electrical.engine_generator[1].frequency_within_normal_range,
            ));
        self.elec_generator_2_load
            .set_value(state.electrical.engine_generator[1].load.get::<percent>());
        self.elec_generator_2_load_within_normal_range
            .set_value(from_bool(
                state.electrical.engine_generator[1].load_within_normal_range,
            ));
        self.elec_generator_2_potential
            .set_value(state.electrical.engine_generator[1].potential.get::<volt>());
        self.elec_generator_2_potential_within_normal_range
            .set_value(from_bool(
                state.electrical.engine_generator[1].potential_within_normal_range,
            ));
        self.elec_generator_line_contactor_2_closed
            .set_value(from_bool(
                state.electrical.engine_generator_line_contactor_closed[1],
            ));
        self.elec_idg_1_pb_fault
            .set_value(from_bool(state.electrical.idg_pb_fault[0]));
        self.elec_idg_2_pb_fault
            .set_value(from_bool(state.electrical.idg_pb_fault[1]));
        self.elec_static_inverter_frequency
            .set_value(state.electrical.static_inverter.frequency.get::<hertz>());
        self.elec_static_inverter_frequency_within_normal_range
            .set_value(from_bool(
                state
                    .electrical
                    .static_inverter
                    .frequency_within_normal_range,
            ));
        self.elec_static_inverter_potential
            .set_value(state.electrical.static_inverter.potential.get::<volt>());
        self.elec_static_inverter_potential_within_normal_range
            .set_value(from_bool(
                state
                    .electrical
                    .static_inverter
                    .potential_within_normal_range,
            ));

        self.elec_tr_1_current.set_value(
            state.electrical.transformer_rectifiers[0]
                .current
                .get::<ampere>(),
        );
        self.elec_tr_1_current_within_normal_range
            .set_value(from_bool(
                state.electrical.transformer_rectifiers[0].current_within_normal_range,
            ));
        self.elec_tr_1_contactor_closed.set_value(from_bool(
            state.electrical.transformer_rectifier_contactor_closed[0],
        ));
        self.elec_tr_1_potential.set_value(
            state.electrical.transformer_rectifiers[0]
                .potential
                .get::<volt>(),
        );
        self.elec_tr_1_potential_within_normal_range
            .set_value(from_bool(
                state.electrical.transformer_rectifiers[0].potential_within_normal_range,
            ));
        self.elec_tr_2_current.set_value(
            state.electrical.transformer_rectifiers[1]
                .current
                .get::<ampere>(),
        );
        self.elec_tr_2_current_within_normal_range
            .set_value(from_bool(
                state.electrical.transformer_rectifiers[1].current_within_normal_range,
            ));
        self.elec_tr_2_contactor_closed.set_value(from_bool(
            state.electrical.transformer_rectifier_contactor_closed[1],
        ));
        self.elec_tr_2_potential.set_value(
            state.electrical.transformer_rectifiers[1]
                .potential
                .get::<volt>(),
        );
        self.elec_tr_2_potential_within_normal_range
            .set_value(from_bool(
                state.electrical.transformer_rectifiers[1].potential_within_normal_range,
            ));
        self.elec_tr_ess_contactor_closed.set_value(from_bool(
            state.electrical.transformer_rectifier_contactor_closed[2],
        ));
        self.elec_tr_ess_current.set_value(
            state.electrical.transformer_rectifiers[2]
                .current
                .get::<ampere>(),
        );
        self.elec_tr_ess_current_within_normal_range
            .set_value(from_bool(
                state.electrical.transformer_rectifiers[2].current_within_normal_range,
            ));
        self.elec_tr_ess_potential.set_value(
            state.electrical.transformer_rectifiers[2]
                .potential
                .get::<volt>(),
        );
        self.elec_tr_ess_potential_within_normal_range
            .set_value(from_bool(
                state.electrical.transformer_rectifiers[2].potential_within_normal_range,
            ));
    }
}
