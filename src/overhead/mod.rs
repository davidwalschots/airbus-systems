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

    #[cfg(test)]
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

    pub fn is_auto(&self) -> bool {
        self.state == AutoOffPushButtonState::Auto
    }

    pub fn is_off(&self) -> bool {
        self.state == AutoOffPushButtonState::Off
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
