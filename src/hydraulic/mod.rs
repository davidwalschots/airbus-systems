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
pub enum BleedSrcType {
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

    pub fn pressurized_by<T: PressureSource + ?Sized>(&mut self, pumps: Vec<&T>) {

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
    active:         bool,
    displacement:   volume,
    flow:           volume_rate,
}
impl ElectricPump {
    pub fn new() -> ElectricPump {
        ElectricPump {
            active:         false,
            displacement:   0.263,
            flow:           0,
        } 
    }

    pub fn update(&mut self) {

    }
}
impl PressureSource for ElectricPump {

}


pub struct EngineDrivenPump {
    active:         bool,
    displacement:   volume,
    flow:           volume_rate,
}
impl EngineDrivenPump {
    pub fn new() -> EngineDrivenPump {
        EngineDrivenPump {
            active:         false,
            displacement:   2.4,
            flow:           0,
        }
    }
    
    pub fn update(&mut self) {
        
    }
}
impl PressureSource for EngineDrivenPump {

}

pub struct PtuPump {
    active:         bool,
    displacement:   volume,
    flow:           volume_rate,
    state:          PtuState,
}
impl PtuPump {
    pub fn new() -> PtuPump {
        PtuPump {
            active:         false,
            displacement:   0,
            flow:           0,
            state:          PtuState::Off,
        }
    }

    pub fn update(&mut self) {
        
    }
}
impl PressureSource for PtuPump {

}

pub struct RatPump {
    active:         bool,
    displacement:   volume,
    flow:           volume_rate,
}
impl RatPump {
    pub fn new() -> RatPump {
        RatPump {
            active:         false,
            displacement:   0,
            flow:           0,       
        }
    }

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
    line: HydLoop,
}

impl Actuator {
    pub fn new(type: ActuatorType, line: HydLoop) -> Actuator {
        Actuator {
            type,
            line,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// BLEED AIR SRC DEFINITION
////////////////////////////////////////////////////////////////////////////////

pub struct BleedAirSource {
    type: BleedSrcType,
}

impl BleedAirSource {
    pub fn new(type: BleedSrcType) -> BleedAirSource {
        BleedAirSource {
            type,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// TESTS
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {

}