// should we use liter/s or m^3/s for volume_rate?
use uom::si::{
    pressure::psi, volume::liter, volume_rate::liter_per_second,
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
pub enum HydLoop {
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

////////////////////////////////////////////////////////////////////////////////
// TRAITS
////////////////////////////////////////////////////////////////////////////////

// Methods which are common to all pumps
pub trait PressureSource {
    fn get_flow(&self) -> volume_rate {
        self.flow
    }
}

////////////////////////////////////////////////////////////////////////////////
// LOOP STRUCTS/IMPLS
////////////////////////////////////////////////////////////////////////////////

pub struct Loop {

}

impl Loop {

}

////////////////////////////////////////////////////////////////////////////////
// RESERVOIR STRUCT/IMPL
////////////////////////////////////////////////////////////////////////////////

pub struct Reservoir {
    
}

impl Reservoir {

}

////////////////////////////////////////////////////////////////////////////////
// PUMP STRUCTS/IMPLS
////////////////////////////////////////////////////////////////////////////////

pub struct ElectricPump {

}
impl ElectricPump {

}
impl PressureSource for ElectricPump {

}


pub struct EngineDrivenPump {

}
impl EngineDrivenPump {

}
impl PressureSource for EngineDrivenPump {

}

pub struct PtuPump {

}
impl PtuPump {

}
impl PresusreSource for PtuPump {

}

pub struct RatPump {

}
impl RatPump {

}
impl PressureSource for RatPump {

}

////////////////////////////////////////////////////////////////////////////////
// ACCUMULATOR STRUCT/IMPL
////////////////////////////////////////////////////////////////////////////////

pub struct HydAccumulator {
    
}

impl HydAccumulator {
    fn get_pressure(&self) -> pressure {
        self.pressure
    }

    fn get_volume(&self) -> volume {
        self.volume
    }
}

////////////////////////////////////////////////////////////////////////////////
// ACTUATOR STRUCT/IMPL
////////////////////////////////////////////////////////////////////////////////

pub struct Actuator {
    type: ActuatorType,
}

impl Actuator {

}

////////////////////////////////////////////////////////////////////////////////
// TESTS
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {

}