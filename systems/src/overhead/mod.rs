use crate::simulation::{SimulationElement, SimulatorReader, SimulatorWriter};

pub struct OnOffFaultPushButton {
    is_on_id: String,
    has_fault_id: String,

    is_on: bool,
    has_fault: bool,
}
impl OnOffFaultPushButton {
    pub fn new_on(name: &str) -> Self {
        Self::new(name, true)
    }

    pub fn new_off(name: &str) -> Self {
        Self::new(name, false)
    }

    fn new(name: &str, is_on: bool) -> Self {
        Self {
            is_on_id: format!("OVHD_{}_PB_IS_ON", name),
            has_fault_id: format!("OVHD_{}_PB_HAS_FAULT", name),
            is_on,
            has_fault: false,
        }
    }

    pub fn set_on(&mut self, value: bool) {
        self.is_on = value;
    }

    pub fn set_fault(&mut self, has_fault: bool) {
        self.has_fault = has_fault;
    }

    pub fn push_on(&mut self) {
        self.is_on = true;
    }

    pub fn push_off(&mut self) {
        self.is_on = false;
    }

    pub fn has_fault(&self) -> bool {
        self.has_fault
    }

    pub fn is_on(&self) -> bool {
        self.is_on
    }

    pub fn is_off(&self) -> bool {
        !self.is_on
    }
}
impl SimulationElement for OnOffFaultPushButton {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_bool(&self.is_on_id, self.is_on());
        writer.write_bool(&self.has_fault_id, self.has_fault());
    }

    fn read(&mut self, reader: &mut SimulatorReader) {
        self.set_on(reader.read_bool(&self.is_on_id));
        self.set_fault(reader.read_bool(&self.has_fault_id));
    }
}

pub struct OnOffAvailablePushButton {
    is_on_id: String,
    is_available_id: String,

    is_on: bool,
    is_available: bool,
}
impl OnOffAvailablePushButton {
    pub fn new_on(name: &str) -> Self {
        Self::new(name, true)
    }

    pub fn new_off(name: &str) -> Self {
        Self::new(name, false)
    }

    fn new(name: &str, is_on: bool) -> Self {
        Self {
            is_on_id: format!("OVHD_{}_PB_IS_ON", name),
            is_available_id: format!("OVHD_{}_PB_IS_AVAILABLE", name),
            is_on,
            is_available: false,
        }
    }

    pub fn set_on(&mut self, value: bool) {
        self.is_on = value;
    }

    pub fn set_available(&mut self, is_available: bool) {
        self.is_available = is_available;
    }

    pub fn turn_on(&mut self) {
        self.is_on = true;
    }

    pub fn turn_off(&mut self) {
        self.is_on = false;
    }

    pub fn is_available(&self) -> bool {
        self.is_available
    }

    pub fn is_on(&self) -> bool {
        self.is_on
    }

    pub fn is_off(&self) -> bool {
        !self.is_on
    }
}
impl SimulationElement for OnOffAvailablePushButton {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_bool(&self.is_on_id, self.is_on());
        writer.write_bool(&self.is_available_id, self.is_available());
    }

    fn read(&mut self, reader: &mut SimulatorReader) {
        self.set_on(reader.read_bool(&self.is_on_id));
        self.set_available(reader.read_bool(&self.is_available_id));
    }
}

pub struct NormalAltnFaultPushButton {
    is_normal_id: String,
    has_fault_id: String,

    is_normal: bool,
    has_fault: bool,
}
impl NormalAltnFaultPushButton {
    pub fn new_normal(name: &str) -> Self {
        Self::new(name, true)
    }

    pub fn new_altn(name: &str) -> Self {
        Self::new(name, false)
    }

    fn new(name: &str, is_normal: bool) -> Self {
        Self {
            is_normal_id: format!("OVHD_{}_PB_IS_NORMAL", name),
            has_fault_id: format!("OVHD_{}_PB_HAS_FAULT", name),
            is_normal,
            has_fault: false,
        }
    }

    pub fn push_altn(&mut self) {
        self.is_normal = false;
    }

    pub fn is_normal(&self) -> bool {
        self.is_normal
    }

    pub fn is_altn(&self) -> bool {
        !self.is_normal
    }

    pub fn set_normal(&mut self, value: bool) {
        self.is_normal = value;
    }

    pub fn has_fault(&self) -> bool {
        self.has_fault
    }

    pub fn set_fault(&mut self, value: bool) {
        self.has_fault = value;
    }
}
impl SimulationElement for NormalAltnFaultPushButton {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_bool(&self.is_normal_id, self.is_normal());
        writer.write_bool(&self.has_fault_id, self.has_fault());
    }

    fn read(&mut self, reader: &mut SimulatorReader) {
        self.set_normal(reader.read_bool(&self.is_normal_id));
        self.set_fault(reader.read_bool(&self.has_fault_id));
    }
}

pub struct AutoOffFaultPushButton {
    is_auto_id: String,
    has_fault_id: String,

    is_auto: bool,
    has_fault: bool,
}
impl AutoOffFaultPushButton {
    pub fn new_auto(name: &str) -> Self {
        Self::new(name, true)
    }

    pub fn new_off(name: &str) -> Self {
        Self::new(name, false)
    }

    fn new(name: &str, is_auto: bool) -> Self {
        Self {
            is_auto_id: format!("OVHD_{}_PB_IS_AUTO", name),
            has_fault_id: format!("OVHD_{}_PB_HAS_FAULT", name),
            is_auto,
            has_fault: false,
        }
    }

    pub fn push_off(&mut self) {
        self.is_auto = false;
    }

    pub fn push_auto(&mut self) {
        self.is_auto = true;
    }

    pub fn is_auto(&self) -> bool {
        self.is_auto
    }

    pub fn is_off(&self) -> bool {
        !self.is_auto
    }

    pub fn set_auto(&mut self, value: bool) {
        self.is_auto = value;
    }

    pub fn has_fault(&self) -> bool {
        self.has_fault
    }

    fn set_fault(&mut self, value: bool) {
        self.has_fault = value;
    }
}
impl SimulationElement for AutoOffFaultPushButton {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_bool(&self.is_auto_id, self.is_auto());
        writer.write_bool(&self.has_fault_id, self.has_fault());
    }

    fn read(&mut self, reader: &mut SimulatorReader) {
        self.set_auto(reader.read_bool(&self.is_auto_id));
        self.set_fault(reader.read_bool(&self.has_fault_id));
    }
}

pub struct FaultReleasePushButton {
    is_released_id: String,
    has_fault_id: String,
    is_released: bool,
    has_fault: bool,
}
impl FaultReleasePushButton {
    #[cfg(test)]
    pub fn new_released(name: &str) -> Self {
        Self::new(name, true)
    }

    pub fn new_in(name: &str) -> Self {
        Self::new(name, false)
    }

    fn new(name: &str, is_released: bool) -> Self {
        Self {
            is_released_id: format!("OVHD_{}_PB_IS_RELEASED", name),
            has_fault_id: format!("OVHD_{}_PB_HAS_FAULT", name),
            is_released,
            has_fault: false,
        }
    }

    pub fn set_released(&mut self, released: bool) {
        self.is_released = self.is_released || released;
    }

    pub fn is_released(&self) -> bool {
        self.is_released
    }

    pub fn set_fault(&mut self, fault: bool) {
        self.has_fault = fault;
    }

    pub fn has_fault(&self) -> bool {
        self.has_fault
    }
}
impl SimulationElement for FaultReleasePushButton {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_bool(&self.is_released_id, self.is_released());
        writer.write_bool(&self.has_fault_id, self.has_fault());
    }

    fn read(&mut self, reader: &mut SimulatorReader) {
        self.set_released(reader.read_bool(&self.is_released_id));
        self.set_fault(reader.read_bool(&self.has_fault_id));
    }
}

pub struct FirePushButton {
    is_released_id: String,
    is_released: bool,
}
impl FirePushButton {
    pub fn new(name: &str) -> Self {
        Self {
            is_released_id: format!("FIRE_BUTTON_{}", name),
            is_released: false,
        }
    }

    pub fn set(&mut self, released: bool) {
        self.is_released = self.is_released || released;
    }

    pub fn is_released(&self) -> bool {
        self.is_released
    }
}
impl SimulationElement for FirePushButton {
    fn write(&self, writer: &mut SimulatorWriter) {
        writer.write_bool(&self.is_released_id, self.is_released());
    }

    fn read(&mut self, reader: &mut SimulatorReader) {
        self.set(reader.read_bool(&self.is_released_id));
    }
}

#[cfg(test)]
mod on_off_fault_push_button_tests {
    use crate::simulation::test::SimulationTestBed;

    use super::*;

    #[test]
    fn new_on_push_button_is_on() {
        assert!(OnOffFaultPushButton::new_on("BUTTON").is_on());
    }

    #[test]
    fn new_off_push_button_is_off() {
        assert!(OnOffFaultPushButton::new_off("BUTTON").is_off());
    }

    #[test]
    fn writes_its_state() {
        let mut button = OnOffFaultPushButton::new_on("ELEC_GEN_1");
        let mut test_bed = SimulationTestBed::new();

        test_bed.run_without_update(&mut button);

        assert!(test_bed.contains_key("OVHD_ELEC_GEN_1_PB_IS_ON"));
        assert!(test_bed.contains_key("OVHD_ELEC_GEN_1_PB_HAS_FAULT"));
    }
}

#[cfg(test)]
mod on_off_available_push_button_tests {
    use crate::simulation::test::SimulationTestBed;

    use super::*;

    #[test]
    fn new_on_push_button_is_on() {
        assert!(OnOffAvailablePushButton::new_on("BUTTON").is_on());
    }

    #[test]
    fn new_off_push_button_is_off() {
        assert!(OnOffAvailablePushButton::new_off("BUTTON").is_off());
    }

    #[test]
    fn writes_its_state() {
        let mut button = OnOffAvailablePushButton::new_on("ELEC_EXT_PWR");
        let mut test_bed = SimulationTestBed::new();

        test_bed.run_without_update(&mut button);

        assert!(test_bed.contains_key("OVHD_ELEC_EXT_PWR_PB_IS_ON"));
        assert!(test_bed.contains_key("OVHD_ELEC_EXT_PWR_PB_IS_AVAILABLE"));
    }
}

#[cfg(test)]
mod normal_altn_fault_push_button_tests {
    use crate::simulation::test::SimulationTestBed;

    use super::*;

    #[test]
    fn new_normal_push_button_is_normal() {
        assert!(NormalAltnFaultPushButton::new_normal("TEST").is_normal());
    }

    #[test]
    fn new_altn_push_button_is_altn() {
        assert!(NormalAltnFaultPushButton::new_altn("TEST").is_altn());
    }

    #[test]
    fn writes_its_state() {
        let mut button = NormalAltnFaultPushButton::new_normal("ELEC_AC_ESS_FEED");
        let mut test_bed = SimulationTestBed::new();

        test_bed.run_without_update(&mut button);

        assert!(test_bed.contains_key("OVHD_ELEC_AC_ESS_FEED_PB_IS_NORMAL"));
        assert!(test_bed.contains_key("OVHD_ELEC_AC_ESS_FEED_PB_HAS_FAULT"));
    }
}

#[cfg(test)]
mod auto_off_fault_push_button_tests {
    use crate::simulation::test::SimulationTestBed;

    use super::*;

    #[test]
    fn new_auto_push_button_is_auto() {
        assert!(AutoOffFaultPushButton::new_auto("TEST").is_auto());
    }

    #[test]
    fn new_off_push_button_is_off() {
        assert!(AutoOffFaultPushButton::new_off("TEST").is_off());
    }

    #[test]
    fn writes_its_state() {
        let mut button = AutoOffFaultPushButton::new_auto("ELEC_BUS_TIE");
        let mut test_bed = SimulationTestBed::new();

        test_bed.run_without_update(&mut button);

        assert!(test_bed.contains_key("OVHD_ELEC_BUS_TIE_PB_IS_AUTO"));
        assert!(test_bed.contains_key("OVHD_ELEC_BUS_TIE_PB_HAS_FAULT"));
    }
}

#[cfg(test)]
mod fault_release_push_button_tests {
    use crate::simulation::test::SimulationTestBed;

    use super::*;

    #[test]
    fn new_in_is_not_released() {
        let pb = FaultReleasePushButton::new_in("TEST");

        assert_eq!(pb.is_released(), false);
    }

    #[test]
    fn new_released_is_released() {
        let pb = FaultReleasePushButton::new_released("TEST");

        assert_eq!(pb.is_released(), true);
    }

    #[test]
    fn when_set_as_released_is_released() {
        let mut pb = FaultReleasePushButton::new_in("TEST");
        pb.set_released(true);

        assert_eq!(pb.is_released(), true);
    }

    #[test]
    fn once_released_stays_released() {
        let mut pb = FaultReleasePushButton::new_in("TEST");
        pb.set_released(true);
        pb.set_released(false);

        assert_eq!(pb.is_released(), true);
    }

    #[test]
    fn writes_its_state() {
        let mut button = FaultReleasePushButton::new_in("IDG_1");
        let mut test_bed = SimulationTestBed::new();

        test_bed.run_without_update(&mut button);

        assert!(test_bed.contains_key("OVHD_IDG_1_PB_IS_RELEASED"));
        assert!(test_bed.contains_key("OVHD_IDG_1_PB_HAS_FAULT"));
    }
}

#[cfg(test)]
mod fire_push_button_tests {
    use crate::simulation::test::SimulationTestBed;

    use super::*;

    #[test]
    fn new_fire_push_button_is_not_released() {
        let pb = FirePushButton::new("TEST");

        assert_eq!(pb.is_released(), false);
    }

    #[test]
    fn when_set_as_released_is_released() {
        let mut pb = FirePushButton::new("TEST");
        pb.set(true);

        assert_eq!(pb.is_released(), true);
    }

    #[test]
    fn once_released_stays_released() {
        let mut pb = FirePushButton::new("TEST");
        pb.set(true);
        pb.set(false);

        assert_eq!(pb.is_released(), true);
    }

    #[test]
    fn writes_its_state() {
        let mut button = FirePushButton::new("APU");
        let mut test_bed = SimulationTestBed::new();

        test_bed.run_without_update(&mut button);

        assert!(test_bed.contains_key("FIRE_BUTTON_APU"));
    }
}
