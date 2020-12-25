use std::cmp;

use uom::si::{
    f32::*, pressure::psi, volume::gallon, volume_rate::gallon_per_second,
};

use crate::{
    overhead::{NormalAltnPushButton, OnOffPushButton},
    shared::{Engine, UpdateContext},
    visitor::Visitable,
}

////////////////////////////////////////////////////////////////////////////////
// DATA & REFERENCES
////////////////////////////////////////////////////////////////////////////////
/// 
/// On A320, the reservoir level variation can, depending on the system, 
/// decrease in flight by about 3.5 l (G RSVR), 4 l (Y RSVR) and 0.5 l (B RSVR)
/// 
/// Each MLG door open (2 total) uses 0.25 liters each of green hyd fluid
/// Each cargo door open (3 total) uses 0.2 liters each of yellow hyd fluid
/// 
/// 
/// EDP (Eaton PV3-240-10C/D/F):
/// ------------------------------------------
/// 37.5 GPM (141.95 L/min)
/// 3750 RPM
/// variable displacement
/// 3000 PSI
/// Displacement: 2.40 in3/rev, 39.3 mL/rev
/// 
/// 
/// Electric Pump (Eaton MPEV-032-15):
/// ------------------------------------------
/// Uses 115/200 VAC, 400HZ electric motor
/// 8.5 GPM (32 L/min)
/// variable displacement
/// 3000 PSI
/// Displacement: 0.263 in3/rev, 4.3 mL/ev
/// 
/// 
/// PTU (Eaton Vickers MPHV3-115-1C):
/// ------------------------------------------
/// Yellow to Green
/// ---------------
/// 34 GPM (130 L/min) from Yellow system
/// 24 GPM (90 L/min) to Green system
/// Maintains constant pressure near 3000PSI in green
/// 
/// Green to Yellow
/// ---------------
/// 16 GPM (60 L/min) from Green system
/// 13 GPM (50 L/min) to Yellow system
/// Maintains constant pressure near 3000PSI in yellow
///  
/// 
/// RAT PUMP (Eaton PV3-115):
/// ------------------------------------------
/// Max displacement: 1.15 in3/rev, 18.85 mL/rev
/// Normal speed: 6,600 RPM
/// Max. Ov. Speed: 8,250 RPM
/// Theoretical Flow at normal speed: 32.86 gpm, 124.4 l/m
/// 
/// 
/// Equations:
/// Flow (Q), gpm:  Q = (in3/rev * rpm)/231
/// 

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
    fn get_flow(&self) -> VolumeRate {
        self.flow
    }

    fn get_displacement(&self) -> Volume {
        self.displacement
    }
}

////////////////////////////////////////////////////////////////////////////////
// LOOP DEFINITION - INCLUDES RESERVOIR AND ACCUMULATOR
////////////////////////////////////////////////////////////////////////////////

pub struct HydLoop {
    color:          LoopColor,
    line_pressure:  Pressure,
    res_volume:     Volume,
}

impl HydLoop {
    const ACCUMULATOR_PRE_CHARGE: f32 = 1885.0;
    const ACCUMULATOR_MAX_VOLUME: f32 = 0.241966;

    pub fn new(color: LoopColor, res_volume: Volume) -> HydLoop {
        HydLoop {
            color,
            line_pressure:  0,
            res_volume,
        }
    }

    pub fn pressurized_by<T: PressureSource + ?Sized>(&mut self, pumps: Vec<&T>) {

    }

    pub fn get_pressure(&self) -> Pressure {
        self.line_pressure
    }

    pub fn get_res_volume(&self) -> Volume {
        self.res_volume
    }

    pub fn draw_res_fluid(&mut self, amount: Volume) -> Volume {
        let drawn: Volume = amount;
        if amount > self.res_volume {
            drawn = self.res_volume;
            self.res_volume = 0;
        } else {
            self.res_volume -= drawn;
        }
        drawn;
    }

    pub fn update(&mut self) {

    }
}

////////////////////////////////////////////////////////////////////////////////
// PUMP DEFINITION
////////////////////////////////////////////////////////////////////////////////

pub struct ElectricPump {
    active:         bool,
    displacement:   Volume,
    flow:           VolumeRate,
}
impl ElectricPump {
    pub fn new() -> ElectricPump {
        ElectricPump {
            active:         false,
            displacement:   Volume::new::<gallon>(0.263),
            flow:           VolumeRate::new::<gallon_per_second>(0),
        } 
    }

    pub fn update(&mut self, line: &HydLoop) {

    }
}
impl PressureSource for ElectricPump {

}

pub struct EngineDrivenPump {
    active:         bool,
    displacement:   Volume,
    flow:           VolumeRate,
}
impl EngineDrivenPump {
    const CNV_IN3_TO_GAL: f32 = 231.0;
    const EDP_MAX_RPM: f32 = 4000.0;
    const ENG_PCT_MAX_RPM: f32 = 65.00; // TODO: DUMMY PLACEHOLDER - get real N1!
    const ENG_DISP_SCALAR: f32 = 58.08;
    const ENG_DISP_MULTIPLIER: f32 = -0.0192;

    pub fn new() -> EngineDrivenPump {
        EngineDrivenPump {
            active:         false,
            displacement:   Volume::new::<gallon>(2.4),
            flow:           VolumeRate::new::<gallon_per_second>(0),
        }
    }
    
    pub fn update(&mut self, context: &UpdateContext, line: &HydLoop) {
        // Calculate displacement
        if line.get_pressure() < 2900 {
            self.displacement = 2.4;
        } else {
            self.displacement = cmp::max((
                line.get_pressure() *
                ENG_DISP_MULTIPLIER +
                ENG_DISP_SCALAR
            ), 0);
        }

        // Calculate flow
        self.flow = (
            ENG_PCT_MAX_RPM * 
            EDP_MAX_RPM * 
            self.displacement /
            CNV_IN3_TO_GAL / 
            60
        ) * (context.delta.as_millis() * 0.001);

        // Update reservoir
        let amount_drawn = line.draw_res_fluid(self.flow);
        self.flow = cmp::min(self.flow, amount_drawn);
    }
}
impl PressureSource for EngineDrivenPump {

}

// PTU "pump" affects 2 hydraulic lines, not just 1
// Need to find a way to specify displacements for multiple lines
pub struct PtuPump {
    active:         bool,
    displacement:   Volume,
    flow:           VolumeRate,
    state:          PtuState,
}
impl PtuPump {
    pub fn new() -> PtuPump {
        PtuPump {
            active:         false,
            displacement:   Volume::new::<gallon>(0),
            flow:           VolumeRate::new::<gallon_per_second>(0),
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
    displacement:   Volume,
    flow:           VolumeRate,
}
impl RatPump {
    pub fn new() -> RatPump {
        RatPump {
            active:         false,
            displacement:   Volume::new::<gallon_per_second>(0),
            flow:           VolumeRate::new::<gallon_per_second>(0),       
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
    a_type: ActuatorType,
    line: HydLoop,
}

impl Actuator {
    pub fn new(a_type: ActuatorType, line: HydLoop) -> Actuator {
        Actuator {
            a_type,
            line,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// BLEED AIR SRC DEFINITION
////////////////////////////////////////////////////////////////////////////////

pub struct BleedAirSource {
    b_type: BleedSrcType,
}

impl BleedAirSource {
    pub fn new(b_type: BleedSrcType) -> BleedAirSource {
        BleedAirSource {
            b_type,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// TESTS
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {

}