mod electrical;
pub use electrical::*;

mod hydraulic;
pub use hydraulic::*;

use crate::{
    apu::{AuxiliaryPowerUnit, AuxiliaryPowerUnitOverheadPanel},
    electrical::ExternalPowerSource,
    shared::{Engine, UpdateContext},
    visitor::{MutableVisitor, Visitable},
};

pub struct A320 {
    engine_1: Engine,
    engine_2: Engine,
    pub apu: AuxiliaryPowerUnit,
    pub apu_overhead: AuxiliaryPowerUnitOverheadPanel,
    ext_pwr: ExternalPowerSource,
    electrical: A320Electrical,
    electrical_overhead: A320ElectricalOverheadPanel,
    hydraulic: A320Hydraulic,
}

impl A320 {
    pub fn new() -> A320 {
        A320 {
            engine_1: Engine::new(),
            engine_2: Engine::new(),
            apu: AuxiliaryPowerUnit::new(),
            apu_overhead: AuxiliaryPowerUnitOverheadPanel::new(),
            ext_pwr: ExternalPowerSource::new(),
            electrical: A320Electrical::new(),
            electrical_overhead: A320ElectricalOverheadPanel::new(),
            hydraulic: A320Hydraulic::new(),
        }
    }

    pub fn update(&mut self, context: &UpdateContext) {
        self.engine_1.update(context);
        self.engine_2.update(context);
        self.apu.update(context, &self.apu_overhead);
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
        )
    }
}

impl Visitable for A320 {
    fn accept(&mut self, visitor: &mut Box<dyn MutableVisitor>) {
        self.engine_1.accept(visitor);
        self.engine_2.accept(visitor);
        self.apu.accept(visitor);
        self.ext_pwr.accept(visitor);
        self.electrical.accept(visitor);
        self.electrical_overhead.accept(visitor);
        self.hydraulic.accept(visitor);
    }
}
