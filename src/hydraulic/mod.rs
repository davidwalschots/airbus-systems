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
pub enum HydLoop {
    Blue,
    Green,
    Yellow,
}

// Represents a source of hydraulic pressure
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pump {
    None,
    ElectricPump,
    EngineDrivenPump,
    HandPump,
    PtuPump,
    RatPump,
}

// Represents an actuator powered by hydraulics
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Actuator {
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

////////////////////////////////////////////////////////////////////////////////
// TRAITS
////////////////////////////////////////////////////////////////////////////////

// Trait which is used by all three loops
pub trait HydraulicLoop {

}

// Trait which is used by all pumps
pub trait PressureSource {

}

// Trait which is used by all three accumulators
pub trait PressureAccumulator {

}

// Trait which is used by all actuators
pub trait PressureSink {

}

////////////////////////////////////////////////////////////////////////////////
// LOOP STRUCTS/IMPLS
////////////////////////////////////////////////////////////////////////////////

pub struct BlueLoop {

}
impl BlueLoop {

}

pub struct GreenLoop {

}
impl GreenLoop {

}

pub struct YellowLoop {

}
impl YellowLoop {

}

////////////////////////////////////////////////////////////////////////////////
// PUMP STRUCTS/IMPLS
////////////////////////////////////////////////////////////////////////////////

pub struct ElectricPump {

}
impl ElectricPump {

}

pub struct EngineDrivenPump {

}
impl EngineDrivenPump {

}

pub struct HandPump {

}
impl HandPump {

}

pub struct PtuPump {

}
impl PtuPump {

}

pub struct RatPump {

}
impl RatPump {

}


////////////////////////////////////////////////////////////////////////////////
// ACCUMULATOR STRUCT/IMPL
////////////////////////////////////////////////////////////////////////////////

pub struct HydAccumulator {
    
}
impl HydAccumulator {

}

////////////////////////////////////////////////////////////////////////////////
// ACTUATOR STRUCTS/IMPL
////////////////////////////////////////////////////////////////////////////////

// Lots of stuff to add here...

////////////////////////////////////////////////////////////////////////////////
// TESTS
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {

}