use systems::electrical::{consumption::SuppliedPower, ElectricalSystem};

pub(super) struct TestElectrical {}
impl ElectricalSystem for TestElectrical {
    fn get_supplied_power(&self) -> SuppliedPower {
        SuppliedPower::new()
    }
}
