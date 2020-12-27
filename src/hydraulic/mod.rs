use std::cmp;

use uom::si::{
    f32::*, pressure::psi, time::second, volume::gallon, volume_rate::gallon_per_second,
};

use crate::{
    overhead::{NormalAltnPushButton, OnOffPushButton},
    shared::{Engine, UpdateContext},
    visitor::Visitable,
};

// TODO:
// - Priority valve
// - Engine fire shutoff valve
// - Leak measurement valve
// - Roll accumulator
// - PTU Rework
// - RAT pump implementation
// - Connecting electric pumps to electric sources
// - Connecting RAT pump/blue loop to emergency generator
// - Actuators
// - Bleed air sources for reservoir/line anti-cavitation

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
/// ------------------------------------------
/// Flow (Q), gpm:  Q = (in3/rev * rpm) / 231
/// Velocity (V), ft/s: V = (0.3208 * flow rate, gpm) / internal area, sq in
/// Force (F), lbs: F = density * area * velocity^2
/// Pressure (P), PSI: P = force / area
///
///
/// Hydraulic Fluid: EXXON HyJet IV
/// ------------------------------------------
/// Kinematic viscosity at 40C: 10.55 mm^2 s^-1, +/- 20%
/// Density at 25C: 996 kg m^-3
///
/// Hydraulic Line (HP) inner diameter
/// ------------------------------------------
/// Currently unknown. Estimating to be 7.5mm for now?
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
    XBleedLine,
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
    fn get_delta_vol(&self) -> Volume;
    fn get_flow(&self) -> VolumeRate;
    fn get_displacement(&self) -> Volume;
    fn is_active(&self) -> bool;
}

////////////////////////////////////////////////////////////////////////////////
// LOOP DEFINITION - INCLUDES RESERVOIR AND ACCUMULATOR
////////////////////////////////////////////////////////////////////////////////

pub struct HydLoop {
    acc_pressure: Pressure,
    acc_volume: Volume,
    pumps: Vec<Box<dyn PressureSource>>,
    color: LoopColor,
    line_pressure: Pressure,
    loop_volume: Volume,
    max_loop_volume: Volume,
    res_volume: Volume,
}

impl HydLoop {
    pub const ACC_PRE_CHARGE: f32 = 1885.0;
    pub const ACC_MAX_VOLUME: f32 = 0.241966;
    pub const ACC_3K_PSI_THRESH: f32 = 0.8993;
    // Moved to struct property:
    // const MAX_LOOP_VOLUME: f32 = 1.09985;

    pub fn new(
        pumps: Vec<Box<dyn PressureSource>>,
        color: LoopColor,
        loop_volume: Volume,
        max_loop_volume: Volume,
        res_volume: Volume,
    ) -> HydLoop {
        HydLoop {
            acc_pressure: Pressure::new::<psi>(HydLoop::ACC_PRE_CHARGE),
            acc_volume: Volume::new::<gallon>(0.),
            pumps,
            color,
            line_pressure: Pressure::new::<psi>(0.),
            loop_volume,
            max_loop_volume,
            res_volume,
        }
    }

    pub fn get_pressure(&self) -> Pressure {
        self.line_pressure
    }

    pub fn get_res_volume(&self) -> Volume {
        self.res_volume
    }

    pub fn draw_res_fluid(&mut self, amount: Volume) -> Volume {
        let mut drawn = amount;
        if amount > self.res_volume {
            drawn = self.res_volume;
            self.res_volume = Volume::new::<gallon>(0.);
        } else {
            self.res_volume -= drawn;
        }
        drawn
    }

    pub fn update(&mut self) {
        // Get total volume output of hydraulic pumps this tick
        // TODO: Implement hydraulic "load" subtraction?
        let mut delta_vol = Volume::new::<gallon>(0.);
        let mut delta_p = Pressure::new::<psi>(0.);
        for pump in self.pumps.iter_mut() {
            delta_vol += pump.get_delta_vol();
        }

        // Calculations involving accumulator and loop volume
        if delta_vol > Volume::new::<gallon>(0.) {
            if self.loop_volume < self.max_loop_volume {
                let vol_diff = self.max_loop_volume - (self.loop_volume + delta_vol);
                if vol_diff > Volume::new::<gallon>(0.) {
                    self.loop_volume += delta_vol;
                    delta_vol = Volume::new::<gallon>(0.);
                } else {
                    self.loop_volume = self.max_loop_volume;
                    delta_vol = vol_diff.abs();
                }
            }

            if self.acc_pressure < Pressure::new::<psi>(3000.)
                && delta_vol > Volume::new::<gallon>(0.)
            {
                let vol_diff = Volume::new::<gallon>(HydLoop::ACC_3K_PSI_THRESH)
                    - (self.acc_volume + delta_vol);
                if vol_diff > Volume::new::<gallon>(0.) {
                    self.acc_volume += delta_vol;
                    self.acc_pressure = (Pressure::new::<psi>(HydLoop::ACC_PRE_CHARGE)
                        * Volume::new::<gallon>(HydLoop::ACC_MAX_VOLUME))
                        / (Volume::new::<gallon>(HydLoop::ACC_MAX_VOLUME) - self.acc_volume);
                } else {
                    self.acc_volume = Volume::new::<gallon>(HydLoop::ACC_3K_PSI_THRESH);
                    self.acc_pressure = Pressure::new::<psi>(3000.);
                    delta_p = Pressure::new::<psi>(
                        (vol_diff.abs().get::<gallon>() * 250000.)
                            / self.loop_volume.get::<gallon>(),
                    );
                    self.loop_volume += vol_diff.abs();
                }
            } else {
                delta_p = Pressure::new::<psi>(
                    (delta_vol.get::<gallon>() * 250000.) / self.loop_volume.get::<gallon>(),
                );
                self.loop_volume += delta_vol;
            }
        } else if delta_vol < Volume::new::<gallon>(0.) {
            if self.acc_volume > Volume::new::<gallon>(0.) {
                let vol_sum = delta_vol + self.acc_volume;
                if vol_sum > Volume::new::<gallon>(0.) {
                    delta_vol = Volume::new::<gallon>(0.);
                    delta_p -= Pressure::new::<psi>(2.); // TODO: replace this WIP placeholder load
                    self.acc_volume += delta_vol; // TODO: is this necessary? delta_vol was just zeroed out...
                    self.acc_pressure = (Pressure::new::<psi>(HydLoop::ACC_PRE_CHARGE)
                        * Volume::new::<gallon>(HydLoop::ACC_MAX_VOLUME))
                        / (Volume::new::<gallon>(HydLoop::ACC_MAX_VOLUME) - self.acc_volume);
                } else {
                    delta_vol = vol_sum;
                    self.acc_volume = Volume::new::<gallon>(0.);
                    self.acc_pressure = Pressure::new::<psi>(HydLoop::ACC_PRE_CHARGE);
                }
            }

            let vol_diff = self.loop_volume + delta_vol - self.max_loop_volume;
            if vol_diff > Volume::new::<gallon>(0.) {
                // TODO: investigate magic number
                delta_p = Pressure::new::<psi>(
                    (delta_vol.get::<gallon>() * 250000.) / self.loop_volume.get::<gallon>(),
                );
            } else {
                self.line_pressure = Pressure::new::<psi>(0.);
            }

            self.loop_volume - Volume::new::<gallon>(0.).max(self.loop_volume + delta_vol);
        }

        // Update loop pressure
        if delta_p != Pressure::new::<psi>(0.) {
            self.line_pressure = Pressure::new::<psi>(0.).max(self.line_pressure + delta_p);
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUMP DEFINITION
////////////////////////////////////////////////////////////////////////////////

pub struct ElectricPump {
    active: bool,
    delta_vol: Volume,
    displacement: Volume,
    flow: VolumeRate,
    rpm: f32,
}
impl ElectricPump {
    const CNV_IN3_TO_GAL: f32 = 231.0;
    const EPUMP_SPOOLUP_TIME: f32 = 2.0;
    const EPUMP_DISP_MULTIPLIER: f32 = -0.02104;
    const EPUMP_DISP_SCALAR: f32 = 6.3646;

    pub fn new() -> ElectricPump {
        ElectricPump {
            active: false,
            delta_vol: Volume::new::<gallon>(0.),
            displacement: Volume::new::<gallon>(0.263),
            flow: VolumeRate::new::<gallon_per_second>(0.),
            rpm: 0.,
        }
    }

    pub fn start(&mut self) {
        self.active = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn update(&mut self, context: &UpdateContext, line: &mut HydLoop) {
        // Pump startup/shutdown process
        if self.active {
            self.rpm += 7600.0f32.max(
                (7600. / ElectricPump::EPUMP_SPOOLUP_TIME) * (context.delta.as_secs_f32() * 10.),
            );
        } else {
            self.rpm -= 7600.0f32.max(
                (7600. / ElectricPump::EPUMP_SPOOLUP_TIME) * (context.delta.as_secs_f32() * 10.),
            );
        }

        // Calculate displacement
        if line.get_pressure() < Pressure::new::<psi>(2900.) {
            self.displacement = Volume::new::<gallon>(0.263);
        } else {
            let disp_calc = Volume::new::<gallon>(
                line.get_pressure().get::<psi>() * ElectricPump::EPUMP_DISP_MULTIPLIER
                    + ElectricPump::EPUMP_DISP_SCALAR,
            );

            self.displacement = Volume::new::<gallon>(0.).max(disp_calc);
        }

        // Calculate flow
        self.flow =
            self.rpm * self.displacement / ElectricPump::CNV_IN3_TO_GAL / Time::new::<second>(60.);
        self.delta_vol = self.flow * Time::new::<second>(context.delta.as_secs_f32());

        // Update reservoir
        let amount_drawn = line.draw_res_fluid(self.delta_vol);
        self.delta_vol = self.delta_vol.min(amount_drawn);
    }
}
impl PressureSource for ElectricPump {
    fn get_delta_vol(&self) -> Volume {
        self.delta_vol
    }

    fn get_flow(&self) -> VolumeRate {
        self.flow
    }

    fn get_displacement(&self) -> Volume {
        self.displacement
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

pub struct EngineDrivenPump {
    active: bool,
    delta_vol: Volume,
    displacement: Volume,
    flow: VolumeRate,
}
impl EngineDrivenPump {
    const CNV_IN3_TO_GAL: f32 = 231.0;
    const EDP_MAX_RPM: f32 = 4000.;
    const EDP_DISP_MULTIPLIER: f32 = -0.192;
    const EDP_DISP_SCALAR: f32 = 58.08;

    const ENG_PCT_MAX_RPM: f32 = 65.00; // TODO: DUMMY PLACEHOLDER - get real N1!

    pub fn new() -> EngineDrivenPump {
        EngineDrivenPump {
            active: false,
            delta_vol: Volume::new::<gallon>(0.),
            displacement: Volume::new::<gallon>(2.4),
            flow: VolumeRate::new::<gallon_per_second>(0.),
        }
    }

    pub fn update(&mut self, context: &UpdateContext, line: &mut HydLoop) {
        // Calculate displacement
        if line.get_pressure() < Pressure::new::<psi>(2900.) {
            self.displacement = Volume::new::<gallon>(2.4);
        } else {
            let disp_calc = Volume::new::<gallon>(
                line.get_pressure().get::<psi>() * EngineDrivenPump::EDP_DISP_MULTIPLIER
                    + EngineDrivenPump::EDP_DISP_SCALAR,
            );
            self.displacement = Volume::new::<gallon>(0.).max(disp_calc);
        }

        // Calculate flow
        self.flow =
            EngineDrivenPump::ENG_PCT_MAX_RPM * EngineDrivenPump::EDP_MAX_RPM * self.displacement
                / EngineDrivenPump::CNV_IN3_TO_GAL
                / Time::new::<second>(60.);
        self.delta_vol = self.flow * Time::new::<second>(context.delta.as_secs_f32());

        // Update reservoir
        let amount_drawn = line.draw_res_fluid(self.delta_vol);
        self.delta_vol = self.delta_vol.min(amount_drawn);
    }
}
impl PressureSource for EngineDrivenPump {
    fn get_delta_vol(&self) -> Volume {
        self.delta_vol
    }

    fn get_flow(&self) -> VolumeRate {
        self.flow
    }

    fn get_displacement(&self) -> Volume {
        self.displacement
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

// PTU "pump" affects 2 hydraulic lines, not just 1
// Need to find a way to specify displacements for multiple lines
pub struct PtuPump {
    active: bool,
    delta_vol: Volume,
    displacement: Volume,
    flow: VolumeRate,
    state: PtuState,
}
impl PtuPump {
    pub fn new() -> PtuPump {
        PtuPump {
            active: false,
            delta_vol: Volume::new::<gallon>(0.),
            displacement: Volume::new::<gallon>(0.),
            flow: VolumeRate::new::<gallon_per_second>(0.),
            state: PtuState::Off,
        }
    }

    pub fn update(&mut self) {}
}
impl PressureSource for PtuPump {
    fn get_delta_vol(&self) -> Volume {
        self.delta_vol
    }

    fn get_flow(&self) -> VolumeRate {
        self.flow
    }

    fn get_displacement(&self) -> Volume {
        self.displacement
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

pub struct RatPump {
    active: bool,
    delta_vol: Volume,
    displacement: Volume,
    flow: VolumeRate,
}
impl RatPump {
    pub fn new() -> RatPump {
        RatPump {
            active: false,
            delta_vol: Volume::new::<gallon>(0.),
            displacement: Volume::new::<gallon>(0.),
            flow: VolumeRate::new::<gallon_per_second>(0.),
        }
    }

    pub fn update(&mut self) {}
}
impl PressureSource for RatPump {
    fn get_delta_vol(&self) -> Volume {
        self.delta_vol
    }

    fn get_flow(&self) -> VolumeRate {
        self.flow
    }

    fn get_displacement(&self) -> Volume {
        self.displacement
    }

    fn is_active(&self) -> bool {
        self.active
    }
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
        Actuator { a_type, line }
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
        BleedAirSource { b_type }
    }
}

////////////////////////////////////////////////////////////////////////////////
// TESTS
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {}
