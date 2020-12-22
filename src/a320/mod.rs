mod electrical;
pub use electrical::*;

mod hydraulic;
pub use hydraulic::*;

use crate::{
    electrical::{AuxiliaryPowerUnit, ExternalPowerSource},
    shared::Engine,
    visitor::{MutableVisitor, Visitable},
};

struct A320 {
    engine_1: Engine,
    engine_2: Engine,
    apu: AuxiliaryPowerUnit,
    ext_pwr: ExternalPowerSource,
    electrical: A320Electrical,
    electrical_overhead: A320ElectricalOverheadPanel,
    hydraulic: A320Hydraulic,
}

impl A320 {
    fn new() -> A320 {
        A320 {
            engine_1: Engine::new(),
            engine_2: Engine::new(),
            apu: AuxiliaryPowerUnit::new(),
            ext_pwr: ExternalPowerSource::new(),
            electrical: A320Electrical::new(),
            electrical_overhead: A320ElectricalOverheadPanel::new(),
            hydraulic: A320Hydraulic::new(),
        }
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
