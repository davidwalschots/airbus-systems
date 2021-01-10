use std::time::Duration;
use uom::si::{
    area::square_meter, f64::*, force::newton, length::foot, length::meter,
    mass_density::kilogram_per_cubic_meter, pressure::atmosphere, pressure::pascal, pressure::psi,
    ratio::percent, thermodynamic_temperature::degree_celsius, time::second, velocity::knot,
    volume::cubic_inch, volume::gallon, volume::liter, volume_rate::cubic_meter_per_second,
    volume_rate::gallon_per_second,
};
use crate::{
    hydraulic::{LoopColor, HydLoop, Pump, EngineDrivenPump, ElectricPump, RatPump, Actuator},
    overhead::{AutoOffPushButton, NormalAltnPushButton, OnOffPushButton},
    shared::{DelayedTrueLogicGate, Engine, UpdateContext},
    visitor::Visitable,
};

pub struct A320Hydraulic {
    blue_loop: HydLoop,
    green_loop: HydLoop,
    yellow_loop: HydLoop,
    engine_driven_pump_1: EngineDrivenPump,
    engine_driven_pump_2: EngineDrivenPump,
    blue_electric_pump: ElectricPump,
    yellow_electric_pump: ElectricPump,
    // Until hydraulic is implemented, we'll fake it with this boolean.
    // blue_pressurised: bool,
}

impl A320Hydraulic {
    pub fn new() -> A320Hydraulic {
        A320Hydraulic {
            // blue_pressurised: true,
            blue_loop: HydLoop::new(
                LoopColor::Blue,
                false,
                Length::new::<meter>(10.),
                Volume::new::<gallon>(1.5),
                Volume::new::<gallon>(1.6),
                Volume::new::<gallon>(1.5)
            ),
            green_loop: HydLoop::new(
                LoopColor::Green,
                true,
                Length::new::<meter>(15.),
                Volume::new::<gallon>(3.6),
                Volume::new::<gallon>(3.7),
                Volume::new::<gallon>(3.6)
            ),
            yellow_loop: HydLoop::new(
                LoopColor::Blue,
                true,
                Length::new::<meter>(14.),
                Volume::new::<gallon>(3.1),
                Volume::new::<gallon>(3.2),
                Volume::new::<gallon>(3.1)
            ),
            engine_driven_pump_1: EngineDrivenPump::new(),
            engine_driven_pump_2: EngineDrivenPump::new(),
            blue_electric_pump: ElectricPump::new(),
            yellow_electric_pump: ElectricPump::new(),
        }
    }

    pub fn is_blue_pressurised(&self) -> bool {
        // self.blue_pressurised
        true
    }

    pub fn update(&mut self, _: &UpdateContext) {}
}

impl Visitable for A320Hydraulic {
    fn accept(&mut self, _: &mut Box<dyn super::MutableVisitor>) {
        // TODO
    }
}

pub struct A320HydraulicOverheadPanel {
}

impl A320HydraulicOverheadPanel {
    pub fn new() -> A320HydraulicOverheadPanel {
        A320HydraulicOverheadPanel {

        }
    }

    pub fn update(&mut self, context: &UpdateContext) {
    }
}
