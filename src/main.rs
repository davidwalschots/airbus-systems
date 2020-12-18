use electrical::{AuxiliaryPowerUnit, ExternalPowerSource};
use uom::si::{f32::{Time}, time::second};
use a320::{A320ElectricalCircuit, A320ElectricalOverheadPanel, A320HydraulicCircuit};
use shared::{Engine, UpdateContext};

mod shared;
mod a320;
mod electrical;
mod overhead;

fn main() {
    let mut circuit = A320ElectricalCircuit::new();
    circuit.update(&UpdateContext::new(Time::new::<second>(1.)), &Engine::new(), &Engine::new(), &AuxiliaryPowerUnit::new(), &ExternalPowerSource::new(),
        &A320HydraulicCircuit::new(), &A320ElectricalOverheadPanel::new());
}
