use a320::{A320Electrical, A320ElectricalOverheadPanel, A320Hydraulic};
use electrical::{AuxiliaryPowerUnit, ExternalPowerSource};
use shared::{Engine, UpdateContext};
use std::time::Duration;

mod a320;
mod electrical;
mod overhead;
mod shared;
mod visitor;

fn main() {
    let mut circuit = A320Electrical::new();
    circuit.update(
        &UpdateContext::new(Duration::new(1, 0)),
        &Engine::new(),
        &Engine::new(),
        &AuxiliaryPowerUnit::new(),
        &ExternalPowerSource::new(),
        &A320Hydraulic::new(),
        &A320ElectricalOverheadPanel::new(),
    );
}
