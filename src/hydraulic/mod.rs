// should we use liter/s or m^3/s for volume_rate?
use uom::si::{
    pressure::psi, volume::gallon, volume_rate::gallon_per_second,
};

use crate::{
    overhead::{NormalAltnPushButton, OnOffPushButton},
    shared::{Engine, UpdateContext},
    visitor::Visitable,
}

////////////////////////////////////////////////////////////////////////////////
// ENUMERATIONS
////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActuatorType {
    Aileron,
    BrakesNormal,
    BrakesAlternate,
    BrakesParking,
    CargoDoor,
    Elevator,
    EmergencyGenerator,
    EngReverser,
    Flaps,
    LandingGearNose,
    LandingGearMain,
    LandingGearDoorNose,
    LandingGearDoorMain,
    NoseWheelSteering,
    Rudder,
    Slat,
    Spoiler,
    Stabilizer,
    YawDamper,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BleedSrc {
    None,
    Engine1,
    XBleedLine
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoopColor {
    Blue,
    Green,
    Yellow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pump {
    None,
    ElectricPump,
    EngineDrivenPump,
    PtuPump,
    RatPump,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PtuState {
    Off,
    GreenToYellow,
    YellowToGreen,
}

////////////////////////////////////////////////////////////////////////////////
// TRAITS
////////////////////////////////////////////////////////////////////////////////

// Trait common to all hydraulic pumps
pub trait PressureSource {
    fn get_flow(&self) -> volume_rate {
        self.flow
    }

    fn get_displacement(&self) -> volume {
        self.displacement
    }
}

////////////////////////////////////////////////////////////////////////////////
// LOOP DEFINITION - INCLUDES RESERVOIR AND ACCUMULATOR
////////////////////////////////////////////////////////////////////////////////

pub struct HydLoop {
    color:          LoopColor,
    line_pressure:  pressure,
    res_volume:     volume,
}

impl HydLoop {
    pub const ACCUMULATOR_PRE_CHARGE: pressure = 1885;
    pub const ACCUMULATOR_MAX_VOLUME: volume = 0.241966;

    pub fn new(color: LoopColor, res_volume: volume) -> HydLoop {
        HydLoop {
            color,
            line_pressure:  0,
            res_volume,
        }
    }

    pub fn pressurized_by(&mut self, pumps: Vec<Pump>) {

    }

    pub fn get_pressure(&self) -> pressure {
        self.line_pressure
    }

    pub fn get_res_volume(&self) -> volume {
        self.res_volume
    }

    pub fn update(&mut self) {

    }
}

////////////////////////////////////////////////////////////////////////////////
// PUMP DEFINITION
////////////////////////////////////////////////////////////////////////////////

pub struct ElectricPump {
    active: bool,
}
impl ElectricPump {
    pub fn update(&mut self) {

    }
}
impl PressureSource for ElectricPump {

}


pub struct EngineDrivenPump {
    active: bool,
}
impl EngineDrivenPump {
    pub fn update(&mut self) {
        
    }
}
impl PressureSource for EngineDrivenPump {

}

pub struct PtuPump {
    active: bool,
    state:  PtuState,
}
impl PtuPump {
    pub fn update(&mut self) {
        
    }
}
impl PresusreSource for PtuPump {

}

pub struct RatPump {
    active: bool,
}
impl RatPump {
    pub fn update(&mut self) {
        
    }
}
impl PressureSource for RatPump {

}

////////////////////////////////////////////////////////////////////////////////
// ACTUATOR DEFINITION
////////////////////////////////////////////////////////////////////////////////

pub struct Actuator {
    type: ActuatorType,
}

impl Actuator {

}

////////////////////////////////////////////////////////////////////////////////
// BLEED AIR SRC DEFINITION
////////////////////////////////////////////////////////////////////////////////

pub struct BleedAirSource {
    type: BleedSrc,
}

impl BleedAirSource {

}

////////////////////////////////////////////////////////////////////////////////
// TESTS
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {

}