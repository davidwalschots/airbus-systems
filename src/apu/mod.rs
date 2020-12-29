use uom::si::{f64::*, ratio::percent};

use crate::{shared::UpdateContext, visitor::Visitable};

pub struct AuxiliaryPowerUnit {
    pub n1: Ratio,
}

impl AuxiliaryPowerUnit {
    pub fn new() -> AuxiliaryPowerUnit {
        AuxiliaryPowerUnit {
            n1: Ratio::new::<percent>(0.),
        }
    }

    pub fn update(&mut self, context: &UpdateContext) {}
}

impl Visitable for AuxiliaryPowerUnit {
    fn accept(&mut self, visitor: &mut Box<dyn crate::visitor::MutableVisitor>) {
        visitor.visit_auxiliary_power_unit(self);
    }
}
