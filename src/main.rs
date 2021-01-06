use a320::{A320Electrical, A320ElectricalOverheadPanel, A320Hydraulic, A320};
use apu::{AuxiliaryPowerUnit, AuxiliaryPowerUnitOverheadPanel};
use electrical::ExternalPowerSource;
use pneumatic::PneumaticOverheadPanel;
use shared::{Engine, UpdateContext};
use std::{io, time::Duration};
use uom::si::{f64::*, length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

mod a320;
mod apu;
mod electrical;
mod overhead;
mod pneumatic;
mod shared;
mod visitor;

fn main() -> io::Result<()> {
    let mut a320 = A320::new();

    a320.update(&UpdateContext::new(
        Duration::new(1, 0),
        Velocity::new::<knot>(250.),
        Length::new::<foot>(5000.),
        ThermodynamicTemperature::new::<degree_celsius>(0.),
    ));

    let mut apu = AuxiliaryPowerUnit::new();
    let mut apu_overhead = AuxiliaryPowerUnitOverheadPanel::new();
    let mut pneumatic_overhead = PneumaticOverheadPanel::new();

    loop {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;

        let slice = &buffer[0..1];
        match slice {
            "M" => {
                if apu_overhead.master.is_on() {
                    apu_overhead.master.turn_off();
                } else {
                    apu_overhead.master.turn_on();
                };
            }
            "S" => {
                if apu_overhead.start.is_on() {
                    apu_overhead.start.turn_off();
                } else {
                    apu_overhead.start.turn_on();
                }
            }
            "B" => {
                if pneumatic_overhead.apu_bleed_is_on() {
                    pneumatic_overhead.turn_apu_bleed_off();
                } else {
                    pneumatic_overhead.turn_apu_bleed_on();
                }
            }
            _ => {}
        }

        apu.update(
            &UpdateContext {
                delta: Duration::from_secs(1),
                airspeed: Velocity::new::<knot>(0.),
                above_ground_level: Length::new::<foot>(0.),
                ambient_temperature: ThermodynamicTemperature::new::<degree_celsius>(10.),
            },
            &apu_overhead,
            &pneumatic_overhead,
        );
        apu_overhead.update_after_apu(&apu);

        println!("APU overhead: {:?}", apu_overhead);
        println!("Pneumatic overhead: {:?}", pneumatic_overhead);
        println!("APU: {:?}", apu);
    }
}
