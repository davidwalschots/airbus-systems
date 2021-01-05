use a320::A320;
#[cfg(any(target_arch = "wasm32", doc))]
use msfs::{
    legacy::{execute_calculator_code, NamedVariable},
    MSFSEvent,
};
use shared::UpdateContext;
use uom::si::{f64::*, length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

mod a320;
mod apu;
mod electrical;
mod overhead;
mod shared;
mod visitor;

#[cfg(any(target_arch = "wasm32", doc))]
#[msfs::gauge(name=airbus)]
async fn demo(mut gauge: msfs::Gauge) -> Result<(), Box<dyn std::error::Error>> {
    let mut a320 = A320::new();

    while let Some(event) = gauge.next_event().await {
        match event {
            MSFSEvent::PreDraw(d) => {
                a320.apu_overhead.master.push_on();
                a320.apu_overhead.start.push_on();

                a320.update(&UpdateContext {
                    delta: d.delta_time(),
                    airspeed: Velocity::new::<knot>(250.),
                    above_ground_level: Length::new::<foot>(5000.),
                    ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(10.),
                });

                let x = NamedVariable::from("APU_EGT");
                x.set_value(10_f64);
            }
            _ => {}
        }
    }

    Ok(())
}
