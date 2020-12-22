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

    pub fn push_on(&mut self) {
        self.state = OnOffPushButtonState::On;
    }

    pub fn push_off(&mut self) {
        self.state = OnOffPushButtonState::Off;
    }

    pub fn is_on(&self) -> bool {
        if let OnOffPushButtonState::On = self.state {
            true
        } else {
            false
        }
    }

    pub fn is_off(&self) -> bool {
        if let OnOffPushButtonState::Off = self.state {
            true
        } else {
            false
        }
    }
}

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

    pub fn push_normal(&mut self) {
        self.state = NormalAltnPushButtonState::Normal;
    }

    pub fn push_altn(&mut self) {
        self.state = NormalAltnPushButtonState::Altn;
    }

    pub fn is_normal(&self) -> bool {
        if let NormalAltnPushButtonState::Normal = self.state {
            true
        } else {
            false
        }
    }

    pub fn is_altn(&self) -> bool {
        if let NormalAltnPushButtonState::Altn = self.state {
            true
        } else {
            false
        }
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
