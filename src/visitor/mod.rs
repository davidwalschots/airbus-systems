use crate::{
    electrical::{
        ApuGenerator, AuxiliaryPowerUnit, Contactor, EngineGenerator, ExternalPowerSource,
    },
    shared::Engine,
};

pub trait MutableVisitor {
    fn visit_auxiliary_power_unit(&mut self, _apu: &mut AuxiliaryPowerUnit) {}
    fn visit_engine(&mut self, _engine: &mut Engine) {}
    fn visit_external_power_source(&mut self, _ext_pwr: &mut ExternalPowerSource) {}
}

pub trait Visitable {
    fn accept(&mut self, visitor: &mut Box<dyn MutableVisitor>);
}
