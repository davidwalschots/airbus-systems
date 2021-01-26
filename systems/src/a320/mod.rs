use self::{fuel::A320Fuel, pneumatic::A320PneumaticOverheadPanel};
use crate::{
    apu::{
        ApuGenerator, AuxiliaryPowerUnit, AuxiliaryPowerUnitFireOverheadPanel,
        AuxiliaryPowerUnitOverheadPanel,
    },
    electrical::ExternalPowerSource,
    shared::Engine,
    simulator::{SimulatorVisitable, SimulatorVisitor, UpdateContext},
};

mod electrical;
pub use electrical::*;

mod hydraulic;
pub use hydraulic::*;

mod fuel;

mod pneumatic;

pub struct A320 {
    engine_1: Engine,
    engine_2: Engine,
    apu: AuxiliaryPowerUnit,
    apu_fire_overhead: AuxiliaryPowerUnitFireOverheadPanel,
    apu_generator: ApuGenerator,
    apu_overhead: AuxiliaryPowerUnitOverheadPanel,
    pneumatic_overhead: A320PneumaticOverheadPanel,
    ext_pwr: ExternalPowerSource,
    electrical: A320Electrical,
    electrical_overhead: A320ElectricalOverheadPanel,
    fuel: A320Fuel,
    hydraulic: A320Hydraulic,
}

impl A320 {
    pub fn new() -> A320 {
        A320 {
            engine_1: Engine::new(),
            engine_2: Engine::new(),
            apu: AuxiliaryPowerUnit::new(),
            apu_fire_overhead: AuxiliaryPowerUnitFireOverheadPanel::new(),
            apu_generator: ApuGenerator::new(),
            apu_overhead: AuxiliaryPowerUnitOverheadPanel::new(),
            pneumatic_overhead: A320PneumaticOverheadPanel::new(),
            ext_pwr: ExternalPowerSource::new(),
            electrical: A320Electrical::new(),
            electrical_overhead: A320ElectricalOverheadPanel::new(),
            fuel: A320Fuel::new(),
            hydraulic: A320Hydraulic::new(),
        }
    }

    pub fn update(&mut self, context: &UpdateContext) {
        self.engine_1.update(context);
        self.engine_2.update(context);

        self.electrical_overhead.update(context);
        self.fuel.update();

        self.apu.update(
            context,
            &self.apu_overhead,
            &self.apu_fire_overhead,
            self.pneumatic_overhead.apu_bleed_is_on(),
            // This will be replaced when integrating the whole electrical system.
            // For now we use the same logic as found in the JavaScript code; ignoring whether or not
            // the engine generators are supplying electricity.
            self.electrical_overhead.apu_generator_is_on()
                && !(self.electrical_overhead.external_power_is_on()
                    && self.electrical_overhead.external_power_is_available()),
            self.fuel.left_inner_tank_has_fuel_remaining(),
        );
        self.apu_generator.update(&self.apu);
        self.apu_overhead.update_after_apu(&self.apu);

        self.ext_pwr.update(context);
        self.electrical_overhead.update(context);
        // Note that soon multiple systems will depend on each other, thus we can expect multiple update functions per type,
        // e.g. the hydraulic system depends on electricity being available, and the electrical system depends on the blue hyd system for
        // EMER GEN. Thus we end up with functions like: electrical.update_before_hydraulic, electrical.update_after_hydraulic.
        self.hydraulic.update(context);
        self.electrical.update(
            context,
            &self.engine_1,
            &self.engine_2,
            &self.apu,
            &self.ext_pwr,
            &self.hydraulic,
            &self.electrical_overhead,
        );
    }
}
impl SimulatorVisitable for A320 {
    fn accept<T: SimulatorVisitor>(&mut self, visitor: &mut T) {
        self.apu.accept(visitor);
        self.apu_fire_overhead.accept(visitor);
        self.apu_generator.accept(visitor);
        self.apu_overhead.accept(visitor);
        self.electrical_overhead.accept(visitor);
        self.fuel.accept(visitor);
        self.pneumatic_overhead.accept(visitor);
    }
}
