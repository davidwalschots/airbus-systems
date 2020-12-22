use a320::{A320Electrical, A320ElectricalOverheadPanel, A320Hydraulic, A320};
use electrical::{AuxiliaryPowerUnit, ExternalPowerSource};
use shared::{Engine, UpdateContext};
use std::time::Duration;

mod a320;
mod electrical;
mod overhead;
mod shared;
mod visitor;

fn main() {
    let mut a320 = A320::new();

    a320.update(&UpdateContext::new(Duration::new(1, 0)));
}
