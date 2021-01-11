#[cfg(any(target_arch = "wasm32", doc))]
use msfs::{
    legacy::{execute_calculator_code, AircraftVariable, NamedVariable},
    MSFSEvent,
};
use shared::UpdateContext;
use uom::si::{f64::*, length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

mod a320;
mod apu;
mod electrical;
mod overhead;
mod pneumatic;
mod shared;
#[cfg(any(target_arch = "wasm32", doc))]
mod simulator;
mod state;
pub use a320::A320;

#[cfg(any(target_arch = "wasm32", doc))]
use simulator::{ModelToSimVisitor, SimToModelVisitor, SimulatorReadWriter};
use state::SimulatorVisitable;

#[cfg(any(target_arch = "wasm32", doc))]
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
