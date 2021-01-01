use std::cmp::Ordering;
use std::f32::consts;
use std::time::Duration;

use uom::si::{
    area::square_meter, f32::*, force::newton, length::foot, length::meter,
    mass_density::kilogram_per_cubic_meter, pressure::atmosphere, pressure::pascal, pressure::psi,
    ratio::percent, thermodynamic_temperature::degree_celsius, time::second, velocity::knot,
    volume::cubic_inch, volume::gallon, volume::liter, volume_rate::cubic_meter_per_second,
    volume_rate::gallon_per_second,
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
/// Reservoirs
/// ------------------------------------------
/// Normal Qty:
/// -----------
/// Blue: 6.5L (1.7 US Gal)
/// Yellow: 12.5L (3.3 US Gal)
/// Green: 14.5L (3.8 US Gal)
///
/// Loops
/// ------------------------------------------
/// Max loop volume - green: 1.09985 gallons (double check)
///
///
/// EDP (Eaton PV3-240-10C/D/F (F is neo)):
/// ------------------------------------------
/// 37.5 GPM max (100% N2)
/// 3750 RPM
/// 3000 PSI
/// Displacement: 2.40 in3/rev
///
///
/// Electric Pump (Eaton MPEV3-032-EA2 (neo) MPEV-032-15 (ceo)):
/// ------------------------------------------
/// Uses 115/200 VAC, 400HZ electric motor
/// 8.45 GPM max
/// 7600 RPM at full displacement, 8000 RPM at no displacement
/// 3000 PSI
/// Displacement: 0.263 in3/rev
///
///
/// PTU (Eaton Vickers MPHV3-115-1C):
/// ------------------------------------------
/// 2987 PSI
///
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
///
/// Actuator Force Simvars
/// -------------------------------------------
/// ACCELERATION BODY X (relative to aircraft, "east/west", Feet per second squared)
/// ACCELERATION BODY Y (relative to aircraft, vertical, Feet per second squared)
/// ACCELERATION BODY Z (relative to aircraft, "north/south", Feet per second squared)
/// ROTATION VELOCITY BODY X (feet per second)
/// ROTATION VELOCITY BODY Y (feet per second)
/// ROTATION VELOCITY BODY Z (feet per second)
/// VELOCITY BODY X (feet per second)
/// VELOCITY BODY Y (feet per second)
/// VELOCITY BODY Z (feet per second)
/// WING FLEX PCT (:1 for left, :2 for right; settable) (percent over 100)
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
}

////////////////////////////////////////////////////////////////////////////////
// LOOP DEFINITION - INCLUDES RESERVOIR AND ACCUMULATOR
////////////////////////////////////////////////////////////////////////////////

pub struct HydLoop {
    accumulator_gas_pressure: Pressure,
    accumulator_gas_volume: Volume,
    accumulator_fluid_volume: Volume,
    color: LoopColor,
    connected_to_ptu: bool,
    loop_length: Length,
    loop_pressure: Pressure,
    loop_volume: Volume,
    max_loop_volume: Volume,
    prv_open: bool,
    ptu_active: bool,
    reservoir_volume: Volume,
}

impl HydLoop {
    const ACCUMULATOR_GAS_NRT: f32 = 128.26; // nRT: n=5.2423, R=0.08206, T=298.15K
    const ACCUMULATOR_GAS_PRE_CHARGE: f32 = 1885.0; // Nitrogen PSI
    const ACCUMULATOR_MAX_VOLUME: f32 = 0.264; // in gallons
    const HYDRAULIC_FLUID_DENSITY: f32 = 1000.55; // Exxon Hyjet IV, kg/m^3
    const HYDRAULIC_FLUID_KINEMATIC_VISCOSITY: f32 = 0.045; // approximate value for ~20C,  m^2/s
    const HYDRAULIC_FLUID_DYNAMIC_VISCOSITY: f32 = 45.02; // kg / (m * s)
    const PIPE_INNER_DIAMETER: f32 = 0.01; // meters
    const PIPE_CROSS_SECTION_AREA: f32 = 0.0000785; // m^2

    pub fn new(
        color: LoopColor,
        connected_to_ptu: bool,
        loop_length: Length,
        loop_volume: Volume,
        max_loop_volume: Volume,
        reservoir_volume: Volume,
    ) -> HydLoop {
        HydLoop {
            accumulator_gas_pressure: Pressure::new::<psi>(HydLoop::ACCUMULATOR_GAS_PRE_CHARGE),
            accumulator_gas_volume: Volume::new::<gallon>(HydLoop::ACCUMULATOR_MAX_VOLUME),
            accumulator_fluid_volume: Volume::new::<gallon>(0.),
            color,
            connected_to_ptu,
            loop_length,
            loop_pressure: Pressure::new::<psi>(0.),
            loop_volume,
            max_loop_volume,
            prv_open: false,
            ptu_active: false,
            reservoir_volume,
        }
    }

    pub fn get_pressure(&self) -> Pressure {
        self.loop_pressure
    }

    pub fn get_reservoir_volume(&self) -> Volume {
        self.reservoir_volume
    }

    pub fn get_usable_reservoir_fluid(&self, amount: Volume) -> Volume {
        let mut drawn = amount;
        if amount > self.reservoir_volume {
            drawn = self.reservoir_volume;
        }
        drawn
    }

    pub fn get_ptu_flow(&self, is_motor: bool, pressure: Pressure) -> VolumeRate {
        if self.color == LoopColor::Yellow {
            let vr = 34.0f32.min(pressure.get::<psi>() * 0.01133) / 60.0;
            if is_motor {
                VolumeRate::new::<gallon_per_second>(vr)
            } else {
                VolumeRate::new::<gallon_per_second>(vr * 0.7059)
            }
        } else if self.color == LoopColor::Green {
            let vr = 16.0f32.min(pressure.get::<psi>() * 0.005333) / 60.0;
            if is_motor {
                VolumeRate::new::<gallon_per_second>(vr)
            } else {
                VolumeRate::new::<gallon_per_second>(vr * 0.8125)
            }
        } else {
            VolumeRate::new::<gallon_per_second>(0.)
        }
    }

    pub fn flow_to_pressure(flow: VolumeRate) -> Pressure {
        Pressure::new::<pascal>(
            0.5 * HydLoop::HYDRAULIC_FLUID_DENSITY
                * ((1.0 / HydLoop::PIPE_CROSS_SECTION_AREA) * flow.get::<cubic_meter_per_second>())
                    .powf(2.0),
        )
    }

    pub fn pressure_to_flow(pressure: Pressure) -> VolumeRate {
        VolumeRate::new::<cubic_meter_per_second>(
            HydLoop::PIPE_CROSS_SECTION_AREA
                * (pressure.get::<pascal>() / (0.5 * HydLoop::HYDRAULIC_FLUID_DENSITY)).powf(0.5),
        )
    }

    pub fn update(
        &mut self,
        context: &UpdateContext,
        electric_pumps: Vec<&ElectricPump>,
        engine_driven_pumps: Vec<&EngineDrivenPump>,
        ram_air_pumps: Vec<&RatPump>,
        ptu_connected_loop: Vec<&HydLoop>,
    ) {
        let mut pressure = self.loop_pressure;
        let mut delta_vol = Volume::new::<gallon>(0.);
        let mut pump_flow_rate_sum = VolumeRate::new::<gallon_per_second>(0.);

        for p in engine_driven_pumps {
            delta_vol += p.get_delta_vol();
            pump_flow_rate_sum +=
                p.get_delta_vol() / Time::new::<second>(context.delta.as_secs_f32());
        }
        for p in electric_pumps {
            delta_vol += p.get_delta_vol();
            pump_flow_rate_sum +=
                p.get_delta_vol() / Time::new::<second>(context.delta.as_secs_f32());
        }
        for p in ram_air_pumps {
            delta_vol += p.get_delta_vol();
            pump_flow_rate_sum +=
                p.get_delta_vol() / Time::new::<second>(context.delta.as_secs_f32());
        }

        // println!(
        //     "==> Delta vol from pumps (g): {}",
        //     delta_vol.get::<gallon>()
        // );
        // println!(
        //     "==> Flow rate from pumps (g/s): {}",
        //     pump_flow_rate_sum.get::<gallon_per_second>()
        // );

        // Draw delta_vol from reservoir
        delta_vol = self.reservoir_volume.min(delta_vol);
        self.reservoir_volume -= delta_vol;

        // println!("==> Pressure before initial calculation: {}", pressure.get::<psi>());
        // Pressure supplied by engine/electric/ram-air pumps
        pressure += HydLoop::flow_to_pressure(pump_flow_rate_sum);
        // println!("==> Pressure after initial calculation: {}", pressure.get::<psi>());

        // TODO: PTU Pump/Motor
        // TODO: Check if PTU isn't off or failed first, and other valid conditions
        // TODO: Should it check against `pressure` or `self.loop_pressure`?
        if self.connected_to_ptu && ptu_connected_loop.len() > 0 {
            // PTU is powering our loop
            if self.ptu_active
                || ptu_connected_loop[0].loop_pressure.get::<psi>()
                    >= self.loop_pressure.get::<psi>() + 500.0
            {
                if !self.ptu_active {
                    self.ptu_active = true;
                }

                let ptu_delta_vol = self.get_usable_reservoir_fluid(
                    self.get_ptu_flow(false, pressure)
                        * Time::new::<second>(context.delta.as_secs_f32()),
                );
                let ptu_delta_flow =
                    ptu_delta_vol / Time::new::<second>(context.delta.as_secs_f32());
                let ptu_delta_p = HydLoop::flow_to_pressure(ptu_delta_flow);

                delta_vol += ptu_delta_vol;
                pressure += ptu_delta_p;
            }
            if ptu_connected_loop[0].loop_pressure.get::<psi>() <= self.loop_pressure.get::<psi>()
                && self.ptu_active
            {
                self.ptu_active = false;
            }

            // PTU is powering the other loop
            if self.ptu_active
                || ptu_connected_loop[0].loop_pressure.get::<psi>()
                    <= self.loop_pressure.get::<psi>() - 500.0
            {
                if !self.ptu_active {
                    self.ptu_active = true;
                }

                let ptu_delta_vol = delta_vol.min(
                    self.get_ptu_flow(true, pressure)
                        * Time::new::<second>(context.delta.as_secs_f32()),
                );
                let ptu_delta_flow =
                    ptu_delta_vol / Time::new::<second>(context.delta.as_secs_f32());
                let ptu_delta_p = HydLoop::flow_to_pressure(ptu_delta_flow);

                delta_vol -= ptu_delta_vol;
                pressure -= ptu_delta_p;
            }
            if ptu_connected_loop[0].loop_pressure.get::<psi>() >= self.loop_pressure.get::<psi>()
                && self.ptu_active
            {
                self.ptu_active = false;
            }
        }

        // Pressure relief valve
        if self.prv_open && pressure.get::<psi>() <= 3190.0 {
            self.prv_open = false;
        }
        if self.prv_open || pressure.get::<psi>() >= 3436.0 {
            if !self.prv_open {
                self.prv_open = true;
            }
            let delta_p_to_close = Pressure::new::<psi>(pressure.get::<psi>() - 3190.0);
            let prv_delta_vol = delta_vol.min(
                HydLoop::pressure_to_flow(delta_p_to_close)
                    * Time::new::<second>(context.delta.as_secs_f32()),
            );
            let prv_delta_p = HydLoop::flow_to_pressure(
                prv_delta_vol / Time::new::<second>(context.delta.as_secs_f32()),
            );

            self.reservoir_volume += prv_delta_vol;
            pressure -= prv_delta_p;
            delta_vol -= prv_delta_vol;
        }

        // If PSI is low, accumulator kicks in and provides flow if available
        // If PSI is high, accumulator kicks in and receives excess flow if able
        // TODO: Limit input flow per tick of accumulator
        // TODO: Limit output flow per tick of accumulator
        if pressure.get::<psi>() < self.accumulator_gas_pressure.get::<psi>()
            && self.accumulator_fluid_volume.get::<gallon>() > 0.
        {
            // Temp: Dividing acc_delta_p by 2 for smaller over/undershoots
            let acc_delta_p = (self.accumulator_gas_pressure - pressure) / 2.0;
            let acc_delta_flow = HydLoop::pressure_to_flow(acc_delta_p);

            // The amount of fluid the accumulator can release
            let acc_delta_vol = Volume::new::<gallon>(
                acc_delta_flow.get::<gallon_per_second>() * context.delta.as_secs_f32(),
            )
            .min(self.accumulator_fluid_volume);

            println!("==>acc_delta_vol: {} liters", acc_delta_vol.get::<liter>());

            // Update accumulator figures
            println!("==>acc_fluid_vol before dispersion: {} liters", self.accumulator_fluid_volume.get::<liter>());
            self.accumulator_fluid_volume -= acc_delta_vol;
            println!("==>acc_fluid_vol after dispersion: {} liters", self.accumulator_fluid_volume.get::<liter>());
            self.accumulator_gas_volume += acc_delta_vol;
            self.accumulator_gas_pressure = Pressure::new::<psi>(
                ((HydLoop::ACCUMULATOR_GAS_PRE_CHARGE + 14.7) * HydLoop::ACCUMULATOR_MAX_VOLUME
                    / self.accumulator_gas_volume.get::<gallon>().max(0.01))
                    - 14.7,
            );

            println!("==>delta_vol before dispersion: {} liters", delta_vol.get::<liter>());
            // Calculate resulting pressure and volume to add back to circuit
            let acc_flow_rate = VolumeRate::new::<gallon_per_second>(
                acc_delta_vol.get::<gallon>() / context.delta.as_secs_f32(),
            );
            delta_vol += acc_delta_vol;
            println!("==>delta_vol after dispersion: {} liters", delta_vol.get::<liter>());
            pressure += HydLoop::flow_to_pressure(acc_flow_rate);
        } else if pressure.get::<psi>() > self.accumulator_gas_pressure.get::<psi>()
            && self.accumulator_fluid_volume.get::<gallon>() < HydLoop::ACCUMULATOR_MAX_VOLUME
        {
            // Temp: Dividing acc_delta_p by 2 for smaller over/undershoots
            let acc_delta_p = (pressure - self.accumulator_gas_pressure) / 2.0;
            let acc_delta_flow = HydLoop::pressure_to_flow(acc_delta_p);

            // The amount of fluid the accumulator can take in
            let acc_delta_vol = Volume::new::<gallon>(
                acc_delta_flow.get::<gallon_per_second>() * context.delta.as_secs_f32(),
            )
            .min(delta_vol)
            .min(
                Volume::new::<gallon>(HydLoop::ACCUMULATOR_MAX_VOLUME)
                    - self.accumulator_fluid_volume,
            );

            // Update accumulator figures
            self.accumulator_fluid_volume += acc_delta_vol;
            self.accumulator_gas_volume -= acc_delta_vol.min(self.accumulator_gas_volume);
            self.accumulator_gas_pressure = Pressure::new::<psi>(
                ((HydLoop::ACCUMULATOR_GAS_PRE_CHARGE + 14.7) * HydLoop::ACCUMULATOR_MAX_VOLUME
                    / self.accumulator_gas_volume.get::<gallon>().max(0.01))
                    - 14.7,
            );

            // Calculate resulting pressure and volume to subtract from circuit
            let acc_flow_rate = acc_delta_vol / Time::new::<second>(context.delta.as_secs_f32());
            delta_vol -= acc_delta_vol;
            pressure -= HydLoop::flow_to_pressure(acc_flow_rate);
        }

        // If `self.loop_volume` is less than `self.max_loop_volume`, draw from `delta_vol` to fill loop
        if self.loop_volume < self.max_loop_volume {
            let difference = self.max_loop_volume - self.loop_volume;
            let delta_loop_vol = delta_vol.min(difference);
            delta_vol -= delta_loop_vol;
            self.loop_volume += delta_loop_vol;
        }

        // If `pressure` is still low, then draw from `self.loop_volume` - (TODO) until nominal loop volume achieved
        if pressure.get::<psi>() <= 14.5 && self.loop_volume.get::<gallon>() > 0. {
            let max_delta_loop_vol = VolumeRate::new::<gallon_per_second>(0.5)
                * Time::new::<second>(context.delta.as_secs_f32());
            let delta_loop_vol = self.loop_volume.min(max_delta_loop_vol);
            self.loop_volume -= delta_loop_vol;
            delta_vol += delta_loop_vol;
        }

        // Pressure drop-off from hydraulic tubing length
        let current_flow_rate = HydLoop::pressure_to_flow(pressure);
        let pressure_loss =
            Pressure::new::<psi>(0.25 * current_flow_rate.get::<gallon_per_second>().powf(2.5));
        // println!(
        //     "==> Current flow rate: {}",
        //     current_flow_rate.get::<gallon_per_second>()
        // );
        // println!(
        //     "==> Current pressure loss: {} ({}%)",
        //     pressure_loss.get::<psi>(),
        //     pressure_loss.get::<psi>() / pressure.get::<psi>() * 100.0
        // );
        pressure -= pressure.min(pressure_loss);

        // TODO: implement actuator (landing gear & cargo door) volume usage (both input and output) logic

        // TODO: implement pressure decrement from actuator usage
        // For each actuator, subtract its pressure (force * area) from `pressure`

        // Final step: update pressure and reservoir volume
        self.reservoir_volume += delta_vol;
        self.loop_pressure = pressure;
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUMP DEFINITION
////////////////////////////////////////////////////////////////////////////////

pub struct Pump {
    max_displacement: Volume,
    reservoir_fluid_used: Volume,
    delta_vol: Volume,
}
impl Pump {
    fn new(max_displacement: Volume) -> Pump {
        Pump {
            max_displacement,
            reservoir_fluid_used: Volume::new::<gallon>(0.),
            delta_vol: Volume::new::<gallon>(0.),
        }
    }

    fn update(&mut self, context: &UpdateContext, line: &HydLoop, rpm: f32) {
        let displacement = Pump::calculate_displacement(line.get_pressure(), self.max_displacement);

        let flow = Pump::calculate_flow(rpm, displacement);
        let delta_vol = flow * Time::new::<second>(context.delta.as_secs_f32());

        let amount_drawn = line.get_usable_reservoir_fluid(delta_vol);
        self.reservoir_fluid_used = amount_drawn;
        self.delta_vol = delta_vol.min(amount_drawn);
    }

    fn calculate_displacement(pressure: Pressure, max_displacement: Volume) -> Volume {
        let numerator_term = -1. * max_displacement.get::<cubic_inch>();
        let exponent_term = -0.25 * (pressure.get::<psi>() - 2990.0);
        let denominator_term = (1. + consts::E.powf(exponent_term)).powf(0.04);

        Volume::new::<cubic_inch>(
            numerator_term / denominator_term + max_displacement.get::<cubic_inch>(),
        )
    }

    fn calculate_flow(rpm: f32, displacement: Volume) -> VolumeRate {
        VolumeRate::new::<gallon_per_second>(rpm * displacement.get::<cubic_inch>() / 231.0 / 60.0)
    }
}
impl PressureSource for Pump {
    fn get_delta_vol(&self) -> Volume {
        self.delta_vol
    }
}

pub struct ElectricPump {
    active: bool,
    rpm: f32,
    pump: Pump,
}
impl ElectricPump {
    const SPOOLUP_TIME: f32 = 4.0;
    const SPOOLDOWN_TIME: f32 = 8.0;
    const MAX_DISPLACEMENT: f32 = 0.263;

    pub fn new() -> ElectricPump {
        ElectricPump {
            active: false,
            rpm: 0.,
            pump: Pump::new(Volume::new::<cubic_inch>(ElectricPump::MAX_DISPLACEMENT)),
        }
    }

    pub fn start(&mut self) {
        self.active = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn update(&mut self, context: &UpdateContext, line: &HydLoop) {
        // Pump startup/shutdown process
        if self.active && self.rpm < 7600.0 {
            self.rpm += 7600.0f32
                .min((7600. / ElectricPump::SPOOLUP_TIME) * (context.delta.as_secs_f32() * 10.));
        } else if !self.active && self.rpm > 0.0 {
            self.rpm -= 7600.0f32
                .min((7600. / ElectricPump::SPOOLDOWN_TIME) * (context.delta.as_secs_f32() * 10.));
        }

        self.pump.update(context, line, self.rpm);
    }
}
impl PressureSource for ElectricPump {
    fn get_delta_vol(&self) -> Volume {
        self.pump.get_delta_vol()
    }
}

pub struct EngineDrivenPump {
    active: bool,
    pump: Pump,
}
impl EngineDrivenPump {
    const LEAP_1A26_MAX_N2_RPM: f32 = 16645.0;
    const MAX_DISPLACEMENT: f32 = 2.4;
    const MAX_RPM: f32 = 4000.;

    pub fn new() -> EngineDrivenPump {
        EngineDrivenPump {
            active: false,
            pump: Pump::new(Volume::new::<cubic_inch>(
                EngineDrivenPump::MAX_DISPLACEMENT,
            )),
        }
    }

    pub fn update(&mut self, context: &UpdateContext, line: &HydLoop, engine: &Engine) {
        let rpm = (1.0f32.min(4.0 * engine.n2.get::<percent>())) * EngineDrivenPump::MAX_RPM;

        self.pump.update(context, line, rpm);
    }
}
impl PressureSource for EngineDrivenPump {
    fn get_delta_vol(&self) -> Volume {
        self.pump.get_delta_vol()
    }
}

pub struct RatPump {
    active: bool,
    pump: Pump,
}
impl RatPump {
    const MAX_DISPLACEMENT: f32 = 1.15;
    const NORMAL_RPM: f32 = 6000.;

    pub fn new() -> RatPump {
        RatPump {
            active: false,
            pump: Pump::new(Volume::new::<cubic_inch>(RatPump::MAX_DISPLACEMENT)),
        }
    }

    pub fn update(&mut self, context: &UpdateContext, line: &HydLoop) {
        self.pump.update(context, line, RatPump::NORMAL_RPM);
    }
}
impl PressureSource for RatPump {
    fn get_delta_vol(&self) -> Volume {
        self.pump.get_delta_vol()
    }
}

////////////////////////////////////////////////////////////////////////////////
// ACTUATOR DEFINITION
////////////////////////////////////////////////////////////////////////////////

// pub struct Actuator {
//     a_type: ActuatorType,
//     active: bool,
//     affected_by_gravity: bool,
//     area: Area,
//     line: HydLoop,
//     neutral_is_zero: bool,
//     stall_load: Force,
//     volume_used_at_max_deflection: Volume,
// }

// // TODO
// impl Actuator {
//     pub fn new(a_type: ActuatorType, line: HydLoop) -> Actuator {
//         Actuator { a_type, line }
//     }
// }

////////////////////////////////////////////////////////////////////////////////
// BLEED AIR SRC DEFINITION
////////////////////////////////////////////////////////////////////////////////

pub struct BleedAir {
    b_type: BleedSrcType,
}

impl BleedAir {
    pub fn new(b_type: BleedSrcType) -> BleedAir {
        BleedAir { b_type }
    }
}

////////////////////////////////////////////////////////////////////////////////
// TESTS
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn green_loop_edp_simulation() {
        let mut edp1 = engine_driven_pump();
        let mut green_loop = hydraulic_loop(LoopColor::Green);
        edp1.active = true;

        let init_n2 = Ratio::new::<percent>(0.5);
        let mut engine1 = engine(init_n2);
        let ct = context(Duration::from_millis(50));
        for x in 0..80 {
            // if x == 50 {
            //     // engine1.n2 = Ratio::new::<percent>(0.0);
            //     green_loop.loop_pressure = Pressure::new::<psi>(1500.);
            // }
            println!("Iteration {}", x);
            println!("-------------------------------------------");
            edp1.update(&ct, &green_loop, &engine1);
            green_loop.update(&ct, Vec::new(), vec![&edp1], Vec::new(), Vec::new());
            if x % 1 == 0 {
                // println!("Iteration {}", x);
                // println!("-------------------------------------------");
                println!("---PSI: {}", green_loop.loop_pressure.get::<psi>());
                println!(
                    "--------Reservoir Volume (g): {}",
                    green_loop.reservoir_volume.get::<gallon>()
                );
                println!(
                    "--------Loop Volume (g): {}",
                    green_loop.loop_volume.get::<gallon>()
                );
                // println!(
                //     "--------Acc Fluid Volume (L): {}",
                //     green_loop.accumulator_fluid_volume.get::<liter>()
                // );
                // println!(
                //     "--------Acc Gas Volume (L): {}",
                //     green_loop.accumulator_gas_volume.get::<liter>()
                // );
                println!(
                    "--------Acc Gas Pressure (psi): {}",
                    green_loop.accumulator_gas_pressure.get::<psi>()
                );
                // println!(
                //     "--------Pressure Relief Valve Open: {}",
                //     green_loop.prv_open
                // );
            }
        }

        assert!(true)
    }

    #[test]
    fn yellow_loop_epump_simulation() {
        let mut epump = electric_pump();
        let mut yellow_loop = hydraulic_loop(LoopColor::Yellow);
        epump.active = true;

        let ct = context(Duration::from_millis(50));
        for x in 0..800 {
            if x == 400 {
                epump.active = false;
            }
            epump.update(&ct, &yellow_loop);
            yellow_loop.update(&ct, vec![&epump], Vec::new(), Vec::new(), Vec::new());
            if x % 20 == 0 {
                println!("Iteration {}", x);
                println!("-------------------------------------------");
                println!("---PSI: {}", yellow_loop.loop_pressure.get::<psi>());
                println!("---RPM: {}", epump.rpm);
                println!(
                    "--------Reservoir Volume (g): {}",
                    yellow_loop.reservoir_volume.get::<gallon>()
                );
                println!(
                    "--------Loop Volume (g): {}",
                    yellow_loop.loop_volume.get::<gallon>()
                );
                println!(
                    "--------Acc Volume (g): {}",
                    yellow_loop.accumulator_gas_volume.get::<gallon>()
                );
            }
        }

        assert!(true)
    }

    fn hydraulic_loop(loop_color: LoopColor) -> HydLoop {
        HydLoop::new(
            loop_color,
            false,
            Length::new::<meter>(10.),
            Volume::new::<gallon>(1.),
            Volume::new::<gallon>(1.09985),
            Volume::new::<gallon>(3.7),
        )
    }

    fn electric_pump() -> ElectricPump {
        ElectricPump::new()
    }

    fn engine_driven_pump() -> EngineDrivenPump {
        EngineDrivenPump::new()
    }

    fn engine(n2: Ratio) -> Engine {
        let mut engine = Engine::new();
        engine.n2 = n2;

        engine
    }

    fn context(delta_time: Duration) -> UpdateContext {
        UpdateContext::new(
            delta_time,
            Velocity::new::<knot>(250.),
            Length::new::<foot>(5000.),
            ThermodynamicTemperature::new::<degree_celsius>(25.0),
        )
    }

    #[cfg(test)]
    mod loop_tests {}

    #[cfg(test)]
    mod epump_tests {}

    #[cfg(test)]
    mod edp_tests {
        use super::*;
        use uom::si::ratio::percent;

        #[test]
        fn starts_inactive() {
            assert!(engine_driven_pump().active == false);
        }

        #[test]
        fn max_flow_under_2500_psi_after_100ms() {
            let n2 = Ratio::new::<percent>(0.6);
            let pressure = Pressure::new::<psi>(2000.);
            let time = Duration::from_millis(100);
            let displacement = Volume::new::<cubic_inch>(EngineDrivenPump::MAX_DISPLACEMENT);
            assert!(delta_vol_equality_check(n2, displacement, pressure, time))
        }

        #[test]
        fn zero_flow_above_3000_psi_after_25ms() {
            let n2 = Ratio::new::<percent>(0.6);
            let pressure = Pressure::new::<psi>(3100.);
            let time = Duration::from_millis(25);
            let displacement = Volume::new::<cubic_inch>(0.);
            assert!(delta_vol_equality_check(n2, displacement, pressure, time))
        }

        fn delta_vol_equality_check(
            n2: Ratio,
            displacement: Volume,
            pressure: Pressure,
            time: Duration,
        ) -> bool {
            let actual = get_edp_actual_delta_vol_when(n2, pressure, time);
            let predicted = get_edp_predicted_delta_vol_when(n2, displacement, time);
            println!("Actual: {}", actual.get::<gallon>());
            println!("Predicted: {}", predicted.get::<gallon>());
            actual == predicted
        }

        fn get_edp_actual_delta_vol_when(n2: Ratio, pressure: Pressure, time: Duration) -> Volume {
            let eng = engine(n2);
            let mut edp = engine_driven_pump();
            let mut line = hydraulic_loop(LoopColor::Green);
            line.loop_pressure = pressure;
            edp.update(&context(time), &line, &eng);
            edp.get_delta_vol()
        }

        fn get_edp_predicted_delta_vol_when(
            n2: Ratio,
            displacement: Volume,
            time: Duration,
        ) -> Volume {
            let edp_rpm = (1.0f32.min(4.0 * n2.get::<percent>())) * EngineDrivenPump::MAX_RPM;
            let expected_flow = Pump::calculate_flow(edp_rpm, displacement);
            expected_flow * Time::new::<second>(time.as_secs_f32())
        }
    }
}
