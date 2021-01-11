#![cfg(any(target_arch = "wasm32", doc))]
use airbus_systems::shared::UpdateContext;
use msfs::{
    legacy::{execute_calculator_code, AircraftVariable, NamedVariable},
    MSFSEvent,
};
use uom::si::{
    f64::*, length::foot, ratio::percent, thermodynamic_temperature::degree_celsius, velocity::knot,
};

use airbus_systems::a320::A320;
use airbus_systems::simulator::{to_bool, ModelToSimVisitor, SimToModelVisitor};
use airbus_systems::state::{SimulatorReadState, SimulatorVisitable, SimulatorWriteState};

#[msfs::gauge(name=airbus)]
async fn demo(mut gauge: msfs::Gauge) -> Result<(), Box<dyn std::error::Error>> {
    let mut a320 = A320::new();
    let ambient_temperature = AircraftVariable::from("AMBIENT TEMPERATURE", "celsius", 0)?;
    let sim_read_writer = SimulatorReadWriter::new()?;

    while let Some(event) = gauge.next_event().await {
        match event {
            MSFSEvent::PreDraw(d) => {
                println!("TICK");

                let mut visitor = SimToModelVisitor::new(sim_read_writer.read());

                a320.accept(&mut visitor);

                a320.update(&UpdateContext {
                    delta: d.delta_time(),
                    airspeed: Velocity::new::<knot>(250.),
                    above_ground_level: Length::new::<foot>(5000.),
                    ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(
                        ambient_temperature.get(),
                    ),
                });

                let mut visitor = ModelToSimVisitor::new();
                a320.accept(&mut visitor);

                sim_read_writer.write(&visitor.get_state());
            }
            _ => {}
        }
    }

    Ok(())
}

pub struct SimulatorReadWriter {
    apu_master_sw: AircraftVariable,
    apu_start_sw: NamedVariable,
    apu_n: NamedVariable,
    apu_egt: NamedVariable,
    apu_egt_caution: NamedVariable,
    apu_egt_warning: NamedVariable,
}
impl SimulatorReadWriter {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(SimulatorReadWriter {
            apu_master_sw: AircraftVariable::from("FUELSYSTEM VALVE SWITCH", "Bool", 8)?,
            apu_start_sw: NamedVariable::from("A32NX_APU_START_ACTIVATED"),
            apu_n: NamedVariable::from("APU_N"),
            apu_egt: NamedVariable::from("APU_EGT"),
            apu_egt_caution: NamedVariable::from("APU_EGT_WARN"),
            apu_egt_warning: NamedVariable::from("APU_EGT_MAX"),
        })
    }

    pub fn read(&self) -> SimulatorReadState {
        SimulatorReadState {
            apu_master_sw_on: to_bool(self.apu_master_sw.get()),
            apu_start_sw_on: to_bool(self.apu_start_sw.get_value()),
            apu_bleed_sw_on: true, // TODO
        }
    }

    pub fn write(&self, state: &SimulatorWriteState) {
        self.apu_n.set_value(state.apu_n.get::<percent>());
        self.apu_egt
            .set_value(state.apu_egt.get::<degree_celsius>());
        self.apu_egt_caution
            .set_value(state.apu_caution_egt.get::<degree_celsius>());
        self.apu_egt_warning
            .set_value(state.apu_warning_egt.get::<degree_celsius>());
    }
}
