use a320::A320;
use shared::UpdateContext;
use std::time::Duration;
use uom::si::{f64::*, length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

mod a320;
mod apu;
mod electrical;
mod overhead;
mod pneumatic;
mod shared;
mod state;

fn main() {
    let mut a320 = A320::new();

    a320.update(&UpdateContext::new(
        Duration::new(1, 0),
        Velocity::new::<knot>(250.),
        Length::new::<foot>(5000.),
        ThermodynamicTemperature::new::<degree_celsius>(0.),
    ));
}
