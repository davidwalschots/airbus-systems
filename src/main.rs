use electrical::{AuxiliaryPowerUnit, ExternalPowerSource};
use a320::{A320ElectricalCircuit, A320ElectricalOverheadPanel, A320HydraulicCircuit};
use shared::{Engine, UpdateContext};
use std::time::Duration;

mod shared;
mod a320;
mod electrical;
mod overhead;

fn main() {
    let mut circuit = A320ElectricalCircuit::new();
    circuit.update(&UpdateContext::new(Duration::new(1, 0)), &Engine::new(), &Engine::new(), &AuxiliaryPowerUnit::new(), &ExternalPowerSource::new(),
        &A320HydraulicCircuit::new(), &A320ElectricalOverheadPanel::new());
}
