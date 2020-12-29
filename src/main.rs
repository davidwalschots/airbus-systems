use a320::{A320Electrical, A320ElectricalOverheadPanel, A320Hydraulic, A320};
use electrical::ExternalPowerSource;
use shared::{Engine, UpdateContext};
use std::time::Duration;
use uom::si::{f32::*, length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

mod a320;
mod electrical;
mod hydraulic;
mod overhead;
mod shared;
mod visitor;

fn main() {
    let mut a320 = A320::new();

    a320.update(&UpdateContext::new(
        Duration::new(1, 0),
        Velocity::new::<knot>(250.),
        Length::new::<foot>(5000.),
        ThermodynamicTemperature::new::<degree_celsius>(0.),
    ));
}
