pub enum OnOffPushButtonState {
    On,
    Off
}

pub struct OnOffPushButton {
    state: OnOffPushButtonState,
    fault: bool,
    available: bool
}

impl OnOffPushButton {
    pub fn new_on() -> OnOffPushButton {
        OnOffPushButton {
            state: OnOffPushButtonState::On,
            fault: false,
            available: false
        }
    }

    pub fn new_off() -> OnOffPushButton {
        OnOffPushButton {
            state: OnOffPushButtonState::Off,
            fault: false,
            available: false
        }
    }

    pub fn push_on(&mut self) {
        self.state = OnOffPushButtonState::On;
    }

    pub fn push_off(&mut self) {
        self.state = OnOffPushButtonState::Off;
    }

    pub fn is_on(&self) -> bool {
        if let OnOffPushButtonState::On = self.state { true } else { false }
    }

    pub fn is_off(&self) -> bool {
        if let OnOffPushButtonState::Off = self.state { true } else { false }
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