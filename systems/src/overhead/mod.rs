#[derive(PartialEq)]
pub enum OnOffPushButtonState {
    On,
    Off,
}

pub struct OnOffPushButton {
    state: OnOffPushButtonState,
    fault: bool,
    available: bool,
}
impl OnOffPushButton {
    pub fn new_on() -> OnOffPushButton {
        OnOffPushButton {
            state: OnOffPushButtonState::On,
            fault: false,
            available: false,
        }
    }

    pub fn new_off() -> OnOffPushButton {
        OnOffPushButton {
            state: OnOffPushButtonState::Off,
            fault: false,
            available: false,
        }
    }

    pub fn set_on(&mut self, value: bool) {
        self.state = if value {
            OnOffPushButtonState::On
        } else {
            OnOffPushButtonState::Off
        };
    }

    pub fn set_fault(&mut self, fault: bool) {
        self.fault = fault;
    }

    pub fn turn_on(&mut self) {
        self.state = OnOffPushButtonState::On;
    }

    pub fn turn_off(&mut self) {
        self.state = OnOffPushButtonState::Off;
    }

    pub fn set_available(&mut self, available: bool) {
        self.available = available;
    }

    pub fn shows_available(&self) -> bool {
        self.available
    }

    pub fn has_fault(&self) -> bool {
        self.fault
    }

    pub fn is_on(&self) -> bool {
        self.state == OnOffPushButtonState::On
    }

    pub fn is_off(&self) -> bool {
        self.state == OnOffPushButtonState::Off
    }
}

#[derive(PartialEq)]
pub enum NormalAltnPushButtonState {
    Normal,
    Altn,
}

pub struct NormalAltnPushButton {
    state: NormalAltnPushButtonState,
    fault: bool,
}
impl NormalAltnPushButton {
    pub fn new_normal() -> NormalAltnPushButton {
        NormalAltnPushButton {
            state: NormalAltnPushButtonState::Normal,
            fault: false,
        }
    }

    pub fn new_altn() -> NormalAltnPushButton {
        NormalAltnPushButton {
            state: NormalAltnPushButtonState::Altn,
            fault: false,
        }
    }

    pub fn push_altn(&mut self) {
        self.state = NormalAltnPushButtonState::Altn;
    }

    pub fn is_normal(&self) -> bool {
        self.state == NormalAltnPushButtonState::Normal
    }

    pub fn is_altn(&self) -> bool {
        self.state == NormalAltnPushButtonState::Altn
    }

    pub fn set_normal(&mut self, value: bool) {
        self.state = if value {
            NormalAltnPushButtonState::Normal
        } else {
            NormalAltnPushButtonState::Altn
        };
    }
}

#[derive(PartialEq)]
pub enum AutoOffPushButtonState {
    Auto,
    Off,
}

pub struct AutoOffPushButton {
    state: AutoOffPushButtonState,
    fault: bool,
}
impl AutoOffPushButton {
    pub fn new_auto() -> AutoOffPushButton {
        AutoOffPushButton {
            state: AutoOffPushButtonState::Auto,
            fault: false,
        }
    }

    pub fn new_off() -> AutoOffPushButton {
        AutoOffPushButton {
            state: AutoOffPushButtonState::Off,
            fault: false,
        }
    }

    pub fn push_off(&mut self) {
        self.state = AutoOffPushButtonState::Off;
    }

    pub fn push_auto(&mut self) {
        self.state = AutoOffPushButtonState::Auto;
    }

    pub fn is_auto(&self) -> bool {
        self.state == AutoOffPushButtonState::Auto
    }

    pub fn is_off(&self) -> bool {
        self.state == AutoOffPushButtonState::Off
    }

    pub fn set_auto(&mut self, value: bool) {
        self.state = if value {
            AutoOffPushButtonState::Auto
        } else {
            AutoOffPushButtonState::Off
        };
    }
}

pub struct FirePushButton {
    released: bool,
}
impl FirePushButton {
    pub fn new() -> Self {
        FirePushButton { released: false }
    }

    pub fn set(&mut self, released: bool) {
        self.released = self.released || released;
    }

    pub fn is_released(&self) -> bool {
        self.released
    }
}

#[cfg(test)]
mod on_off_push_button_tests {
    use super::OnOffPushButton;

    #[test]
    fn new_on_push_button_is_on() {
        assert!(OnOffPushButton::new_on().is_on());
    }

    #[test]
    fn new_off_push_button_is_off() {
        assert!(OnOffPushButton::new_off().is_off());
    }
}

#[cfg(test)]
mod normal_altn_push_button_tests {
    use super::NormalAltnPushButton;

    #[test]
    fn new_normal_push_button_is_normal() {
        assert!(NormalAltnPushButton::new_normal().is_normal());
    }

    #[test]
    fn new_altn_push_button_is_altn() {
        assert!(NormalAltnPushButton::new_altn().is_altn());
    }
}

#[cfg(test)]
mod auto_off_push_button_tests {
    use super::AutoOffPushButton;

    #[test]
    fn new_auto_push_button_is_auto() {
        assert!(AutoOffPushButton::new_auto().is_auto());
    }

    #[test]
    fn new_off_push_button_is_off() {
        assert!(AutoOffPushButton::new_off().is_off());
    }
}

#[cfg(test)]
mod fire_push_button_tests {
    use super::FirePushButton;

    #[test]
    fn new_fire_push_button_is_not_released() {
        let pb = FirePushButton::new();

        assert_eq!(pb.is_released(), false);
    }

    #[test]
    fn when_set_as_released_is_released() {
        let mut pb = FirePushButton::new();
        pb.set(true);

        assert_eq!(pb.is_released(), true);
    }

    #[test]
    fn once_released_stays_released() {
        let mut pb = FirePushButton::new();
        pb.set(true);
        pb.set(false);

        assert_eq!(pb.is_released(), true);
    }
}
