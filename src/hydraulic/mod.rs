use std::f32::consts;
use std::time::Duration;

use uom::si::{
    f32::*, length::foot, pressure::psi, ratio::percent, time::second, velocity::knot,
    volume::cubic_inch, volume::gallon, volume_rate::gallon_per_second,
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
}

////////////////////////////////////////////////////////////////////////////////
// LOOP DEFINITION - INCLUDES RESERVOIR AND ACCUMULATOR
////////////////////////////////////////////////////////////////////////////////

pub struct HydLoop {
    accumulator_pressure: Pressure,
    accumulator_volume: Volume,
    color: LoopColor,
    loop_pressure: Pressure,
    loop_volume: Volume,
    max_loop_volume: Volume,
    reservoir_volume: Volume,
}

impl HydLoop {
    const ACCUMULATOR_PRE_CHARGE: f32 = 1885.0;
    const ACCUMULATOR_MAX_VOLUME: f32 = 0.241966;
    const ACCUMULATOR_3K_PSI_THRESHOLD: f32 = 0.8993;
    // Moved to struct property:
    // const MAX_LOOP_VOLUME: f32 = 1.09985;

    pub fn new(
        color: LoopColor,
        loop_volume: Volume,
        max_loop_volume: Volume,
        reservoir_volume: Volume,
    ) -> HydLoop {
        HydLoop {
            accumulator_pressure: Pressure::new::<psi>(HydLoop::ACCUMULATOR_PRE_CHARGE),
            accumulator_volume: Volume::new::<gallon>(0.),
            color,
            loop_pressure: Pressure::new::<psi>(0.),
            loop_volume,
            max_loop_volume,
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

    pub fn update(
        &mut self,
        electric_pumps: Vec<&ElectricPump>,
        engine_driven_pumps: Vec<&EngineDrivenPump>,
        ram_air_pumps: Vec<&RatPump>,
    ) {
        let mut delta_vol = Volume::new::<gallon>(0.);
        let mut delta_p = Pressure::new::<psi>(0.);

        // Get total volume output of hydraulic pumps this tick
        for p in electric_pumps {
            self.reservoir_volume -= p.pump.reservoir_fluid_used;
            delta_vol += p.get_delta_vol();
        }
        for p in engine_driven_pumps {
            self.reservoir_volume -= p.pump.reservoir_fluid_used;
            delta_vol += p.get_delta_vol();
        }
        for p in ram_air_pumps {
            self.reservoir_volume -= p.pump.reservoir_fluid_used;
            delta_vol += p.get_delta_vol();
        }

        // println!("---------Delta vol before sub: {}", delta_vol.get::<gallon>());

        // WIP: Placeholder load
        delta_vol -= Volume::new::<gallon>(0.004);
        self.reservoir_volume += Volume::new::<gallon>(0.004);

        // println!("---------Delta vol after sub: {}", delta_vol.get::<gallon>());

        // Calculations involving accumulator and loop volume
        if delta_vol.get::<gallon>() > 0.0 {
            if self.loop_volume < self.max_loop_volume {
                let vol_diff = self.max_loop_volume.get::<gallon>()
                    - (self.loop_volume.get::<gallon>() + delta_vol.get::<gallon>());
                if vol_diff > 0.0 {
                    self.loop_volume += delta_vol;
                    delta_vol = Volume::new::<gallon>(0.);
                } else {
                    self.loop_volume = self.max_loop_volume;
                    delta_vol = Volume::new::<gallon>(vol_diff.abs());
                }
            }

            if self.accumulator_pressure < Pressure::new::<psi>(3000.)
                && delta_vol > Volume::new::<gallon>(0.)
            {
                let vol_diff = HydLoop::ACCUMULATOR_3K_PSI_THRESHOLD
                    - (self.accumulator_volume.get::<gallon>() + delta_vol.get::<gallon>());
                if vol_diff > 0.0 {
                    self.accumulator_volume += delta_vol;
                    self.accumulator_pressure =
                        (Pressure::new::<psi>(HydLoop::ACCUMULATOR_PRE_CHARGE)
                            * Volume::new::<gallon>(HydLoop::ACCUMULATOR_MAX_VOLUME))
                            / (Volume::new::<gallon>(HydLoop::ACCUMULATOR_MAX_VOLUME)
                                - self.accumulator_volume);
                } else {
                    self.accumulator_volume =
                        Volume::new::<gallon>(HydLoop::ACCUMULATOR_3K_PSI_THRESHOLD);
                    self.accumulator_pressure = Pressure::new::<psi>(3000.);
                    delta_p = Pressure::new::<psi>(
                        (vol_diff.abs() * 5000.) / self.loop_volume.get::<gallon>(),
                    );
                    self.loop_volume += Volume::new::<gallon>(vol_diff.abs());
                }
            } else {
                delta_p = Pressure::new::<psi>(
                    (delta_vol.get::<gallon>() * 5000.) / self.loop_volume.get::<gallon>(),
                );
                self.loop_volume += delta_vol;
            }
        } else if delta_vol.get::<gallon>() < 0.0 {
            if self.accumulator_volume.get::<gallon>() > 0.0 {
                // println!("---DEBUG: delta_vol < 0, decreasing accumulator volume...");
                let vol_sum = delta_vol + self.accumulator_volume;
                if vol_sum > Volume::new::<gallon>(0.) {
                    self.accumulator_volume += delta_vol;
                    delta_vol = Volume::new::<gallon>(0.);
                    delta_p -= Pressure::new::<psi>(2.); // TODO: replace this WIP placeholder load
                    self.accumulator_pressure =
                        (Pressure::new::<psi>(HydLoop::ACCUMULATOR_PRE_CHARGE)
                            * Volume::new::<gallon>(HydLoop::ACCUMULATOR_MAX_VOLUME))
                            / (Volume::new::<gallon>(HydLoop::ACCUMULATOR_MAX_VOLUME)
                                - self.accumulator_volume);
                } else {
                    delta_vol = vol_sum;
                    self.accumulator_volume = Volume::new::<gallon>(0.);
                    self.accumulator_pressure =
                        Pressure::new::<psi>(HydLoop::ACCUMULATOR_PRE_CHARGE);
                }
            }

            let vol_diff = self.loop_volume.get::<gallon>() + delta_vol.get::<gallon>()
                - self.max_loop_volume.get::<gallon>();
            if vol_diff > 0.0 {
                // TODO: investigate magic number 5000.
                delta_p = Pressure::new::<psi>(
                    (delta_vol.get::<gallon>() * 5000.) / self.loop_volume.get::<gallon>(),
                );
            } else {
                self.loop_pressure = Pressure::new::<psi>(0.);
            }

            self.loop_volume = Volume::new::<gallon>(0.).max(self.loop_volume + delta_vol);
        }

        // Update loop pressure
        if delta_p != Pressure::new::<psi>(0.) {
            self.loop_pressure = Pressure::new::<psi>(0.).max(self.loop_pressure + delta_p);
        }
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

        // TODO: Remove debug statements
        // println!("--- EDP Displacement: {}", displacement.get::<cubic_inch>());
        // println!(
        //     "--- Volume displaced this tick: {}",
        //     delta_vol.get::<gallon>()
        // );

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
    const SPOOLUP_TIME: f32 = 2.0;
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
        let delta_rpm = 7600.0f32
            .max((7600. / ElectricPump::SPOOLUP_TIME) * (context.delta.as_secs_f32() * 10.));
        if self.active {
            self.rpm += delta_rpm;
        } else {
            self.rpm -= delta_rpm;
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
        let rpm = engine.n2.get::<percent>() * EngineDrivenPump::MAX_RPM;

        self.pump.update(context, line, rpm);
    }
}
impl PressureSource for EngineDrivenPump {
    fn get_delta_vol(&self) -> Volume {
        self.pump.get_delta_vol()
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
            displacement: Volume::new::<cubic_inch>(0.),
            flow: VolumeRate::new::<gallon_per_second>(0.),
            state: PtuState::Off,
        }
    }

    pub fn update(&mut self, context: &UpdateContext, line: &HydLoop) {}
}
impl PressureSource for PtuPump {
    fn get_delta_vol(&self) -> Volume {
        self.delta_vol
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
        let mut green_loop = hydraulic_loop();
        edp1.active = true;

        let init_n2 = Ratio::new::<percent>(1.0);
        let mut engine1 = engine(init_n2);
        let ct = context(Duration::from_millis(25));
        for x in 0..400 {
            if x == 200 {
                engine1.n2 = Ratio::new::<percent>(0.0);
            }
            edp1.update(&ct, &green_loop, &engine1);
            green_loop.update(Vec::new(), vec![&edp1], Vec::new());
            if x % 10 == 0 {
                println!("Iteration {}", x);
                println!("-------------------------------------------");
                println!("---PSI: {}", green_loop.loop_pressure.get::<psi>());
                println!(
                    "--------Reservoir Volume (g): {}",
                    green_loop.reservoir_volume.get::<gallon>()
                );
                println!(
                    "--------Loop Volume (g): {}",
                    green_loop.loop_volume.get::<gallon>()
                );
                println!(
                    "--------Acc Volume (g): {}",
                    green_loop.accumulator_volume.get::<gallon>()
                );
            }
        }

        assert!(true)
    }

    fn hydraulic_loop() -> HydLoop {
        HydLoop::new(
            LoopColor::Green,
            Volume::new::<gallon>(1.),
            Volume::new::<gallon>(1.09985),
            Volume::new::<gallon>(3.7),
        )
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
        fn max_flow_under_2500_psi_after_25ms() {
            let n2 = Ratio::new::<percent>(0.6);
            let pressure = Pressure::new::<psi>(2400.);
            let time = Duration::from_millis(25);
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
            let mut line = hydraulic_loop();
            line.loop_pressure = pressure;
            edp.update(&context(time), &line, &eng);
            edp.get_delta_vol()
        }

        fn get_edp_predicted_delta_vol_when(
            n2: Ratio,
            displacement: Volume,
            time: Duration,
        ) -> Volume {
            let edp_rpm = n2.get::<percent>() * EngineDrivenPump::MAX_RPM;
            let expected_flow = Pump::calculate_flow(edp_rpm, displacement);
            expected_flow * Time::new::<second>(time.as_secs_f32())
        }
    }
}
