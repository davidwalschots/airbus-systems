use crate::simulator::{
    SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorVariable,
    SimulatorWriteState,
};

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
            is_on_id: format!("OVERHEAD_{}_PB_IS_ON", name),
            has_fault_id: format!("OVERHEAD_{}_PB_HAS_FAULT", name),
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

    pub fn turn_on(&mut self) {
        self.is_on = true;
    }

    pub fn turn_off(&mut self) {
        self.is_on = false;
    }

    pub fn has_fault(&self) -> bool {
        self.has_fault
    }

    pub fn is_on(&self) -> bool {
        self.is_on == true
    }

    pub fn is_off(&self) -> bool {
        self.is_on == false
    }
}
impl SimulatorElementVisitable for OnOffFaultPushButton {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for OnOffFaultPushButton {
    fn write(&self, state: &mut SimulatorWriteState) {
        state.add(SimulatorVariable::from_bool(&self.is_on_id, self.is_on()));
        state.add(SimulatorVariable::from_bool(
            &self.has_fault_id,
            self.has_fault(),
        ));
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
            is_on_id: format!("OVERHEAD_{}_PB_IS_ON", name),
            is_available_id: format!("OVERHEAD_{}_PB_IS_AVAILABLE", name),
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
        self.is_on == true
    }

    pub fn is_off(&self) -> bool {
        self.is_on == false
    }
}
impl SimulatorElementVisitable for OnOffAvailablePushButton {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for OnOffAvailablePushButton {
    fn write(&self, state: &mut SimulatorWriteState) {
        state.add(SimulatorVariable::from_bool(&self.is_on_id, self.is_on()));
        state.add(SimulatorVariable::from_bool(
            &self.is_available_id,
            self.is_available(),
        ));
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
            is_normal_id: format!("OVERHEAD_{}_PB_IS_NORMAL", name),
            has_fault_id: format!("OVERHEAD_{}_PB_HAS_FAULT", name),
            is_normal,
            has_fault: false,
        }
    }

    pub fn push_altn(&mut self) {
        self.is_normal = false;
    }

    pub fn is_normal(&self) -> bool {
        self.is_normal == true
    }

    pub fn is_altn(&self) -> bool {
        self.is_normal == false
    }

    pub fn set_normal(&mut self, value: bool) {
        self.is_normal = value;
    }

    pub fn has_fault(&self) -> bool {
        self.has_fault
    }
}
impl SimulatorElementVisitable for NormalAltnFaultPushButton {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for NormalAltnFaultPushButton {
    fn write(&self, state: &mut SimulatorWriteState) {
        state.add(SimulatorVariable::from_bool(
            &self.is_normal_id,
            self.is_normal(),
        ));
        state.add(SimulatorVariable::from_bool(
            &self.has_fault_id,
            self.has_fault(),
        ));
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
            is_auto_id: format!("OVERHEAD_{}_PB_IS_AUTO", name),
            has_fault_id: format!("OVERHEAD_{}_PB_HAS_FAULT", name),
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
        self.is_auto == true
    }

    pub fn is_off(&self) -> bool {
        self.is_auto == false
    }

    pub fn set_auto(&mut self, value: bool) {
        self.is_auto = value;
    }

    pub fn has_fault(&self) -> bool {
        self.has_fault
    }
}
impl SimulatorElementVisitable for AutoOffFaultPushButton {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for AutoOffFaultPushButton {
    fn write(&self, state: &mut SimulatorWriteState) {
        state.add(SimulatorVariable::from_bool(
            &self.is_auto_id,
            self.is_auto(),
        ));
        state.add(SimulatorVariable::from_bool(
            &self.has_fault_id,
            self.has_fault(),
        ));
    }
}

pub struct FirePushButton {
    released_id: String,
    released: bool,
}
impl FirePushButton {
    pub fn new(name: &str) -> Self {
        Self {
            released_id: format!("OVERHEAD_{}_FIRE_PB_IS_RELEASED", name),
            released: false,
        }
    }

    pub fn set(&mut self, released: bool) {
        self.released = self.released || released;
    }

    pub fn is_released(&self) -> bool {
        self.released
    }
}
impl SimulatorElementVisitable for FirePushButton {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for FirePushButton {
    fn write(&self, state: &mut SimulatorWriteState) {
        state.add(SimulatorVariable::from_bool(
            &self.released_id,
            self.is_released(),
        ));
    }
}

#[cfg(test)]
mod on_off_fault_push_button_tests {
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
        let button = OnOffFaultPushButton::new_on("ELEC_GEN_1");
        let mut state = SimulatorWriteState::new();

        button.write(&mut state);

        assert!(state.len_is(2));
        assert!(state.contains_bool("OVERHEAD_ELEC_GEN_1_PB_IS_ON", true));
        assert!(state.contains_bool("OVERHEAD_ELEC_GEN_1_PB_HAS_FAULT", false));
    }
}

#[cfg(test)]
mod on_off_available_push_button_tests {
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
        let button = OnOffAvailablePushButton::new_on("ELEC_EXT_PWR");
        let mut state = SimulatorWriteState::new();

        button.write(&mut state);

        assert!(state.len_is(2));
        assert!(state.contains_bool("OVERHEAD_ELEC_EXT_PWR_PB_IS_ON", true));
        assert!(state.contains_bool("OVERHEAD_ELEC_EXT_PWR_PB_IS_AVAILABLE", false));
    }
}

#[cfg(test)]
mod normal_altn_fault_push_button_tests {
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
        let button = NormalAltnFaultPushButton::new_normal("ELEC_AC_ESS_FEED");
        let mut state = SimulatorWriteState::new();

        button.write(&mut state);

        assert!(state.len_is(2));
        assert!(state.contains_bool("OVERHEAD_ELEC_AC_ESS_FEED_PB_IS_NORMAL", true));
        assert!(state.contains_bool("OVERHEAD_ELEC_AC_ESS_FEED_PB_HAS_FAULT", false));
    }
}

#[cfg(test)]
mod auto_off_fault_push_button_tests {
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
        let button = AutoOffFaultPushButton::new_auto("ELEC_BUS_TIE");
        let mut state = SimulatorWriteState::new();

        button.write(&mut state);

        assert!(state.len_is(2));
        assert!(state.contains_bool("OVERHEAD_ELEC_BUS_TIE_PB_IS_AUTO", true));
        assert!(state.contains_bool("OVERHEAD_ELEC_BUS_TIE_PB_HAS_FAULT", false));
    }
}

#[cfg(test)]
mod fire_push_button_tests {
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
        let button = FirePushButton::new("APU");
        let mut state = SimulatorWriteState::new();

        button.write(&mut state);

        assert!(state.len_is(1));
        assert!(state.contains_bool("OVERHEAD_APU_FIRE_PB_IS_RELEASED", false));
    }
}
