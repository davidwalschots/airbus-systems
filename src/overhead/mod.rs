enum OnOffPushButtonState {
    On,
    Off
}

pub struct OnOffPushButton {
    state: OnOffPushButtonState,
    fault: bool,
    available: bool
}

impl OnOffPushButton {
    pub fn new() -> OnOffPushButton {
        OnOffPushButton {
            state: OnOffPushButtonState::On,
            fault: false,
            available: false
        }
    }
}