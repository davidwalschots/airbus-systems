use electrical::{AuxiliaryPowerUnit, ExternalPowerSource};
use uom::si::{f32::{Time}, time::second};
use a320::A320ElectricalCircuit;
use shared::{Engine, UpdateContext};

mod shared;
mod a320;
mod electrical;

fn main() {
    let mut circuit = A320ElectricalCircuit::new();
    circuit.update(&UpdateContext::new(Time::new::<second>(1.)), &Engine::new(), &Engine::new(), &AuxiliaryPowerUnit::new(), &ExternalPowerSource::new());
}
