use uom::si::f32::{Frequency, ElectricPotential, ElectricCurrent};

/// Represents a type of electric current.
#[derive(Debug)]
enum Current {
    Alternating(Frequency, ElectricPotential, ElectricCurrent),
    Direct(ElectricPotential, ElectricCurrent),
    None
}

impl Current {
    pub fn is_alternating(self) -> bool {
        if let Current::Alternating(..) = self { true } else { false }
    }

    pub fn is_direct(self) -> bool {
        if let Current::Direct(..) = self { true } else { false }
    }

    pub fn is_none(self) -> bool {
        if let Current::None = self { true } else { false }
    }
}

/// Represents the state of a contactor.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ContactorState {
    Open,
    Closed
}

/// Represents a contactor in a electrical power circuit.
#[derive(Debug)]
pub struct Contactor {
    state: ContactorState,
}

impl Contactor {
    fn new() -> Contactor {
        Contactor {
            state: ContactorState::Open
        }
    }

    fn toggle(&mut self, should_be_closed: bool) {
        self.state = match self.state {
            ContactorState::Open if should_be_closed => ContactorState::Closed,
            ContactorState::Closed if !should_be_closed => ContactorState::Open,
            _ => self.state
        };
    }
}

#[cfg(test)]
mod current_tests {
    use uom::si::{electric_current::ampere, electric_potential::volt, frequency::hertz};

    use super::*;

    #[test]
    fn alternating_current_is_alternating() {
        assert_eq!(alternating_current().is_alternating(), true);
    }

    #[test]
    fn alternating_current_is_not_direct() {
        assert_eq!(alternating_current().is_direct(), false);
    }

    #[test]
    fn alternating_current_is_not_none() {
        assert_eq!(alternating_current().is_none(), false);
    }

    #[test]
    fn direct_current_is_not_alternating() {
        assert_eq!(direct_current().is_alternating(), false);
    }

    #[test]
    fn direct_current_is_direct() {
        assert_eq!(direct_current().is_direct(), true);
    }

    #[test]
    fn direct_current_is_not_none() {
        assert_eq!(direct_current().is_none(), false);
    }
    
    #[test]
    fn none_current_is_not_alternating() {
        assert_eq!(none_current().is_alternating(), false);
    }

    #[test]
    fn none_current_is_not_direct() {
        assert_eq!(none_current().is_direct(), false);
    }

    #[test]
    fn none_current_is_none() {
        assert_eq!(none_current().is_none(), true);
    }

    fn alternating_current() -> Current {
        Current::Alternating(Frequency::new::<hertz>(0.), ElectricPotential::new::<volt>(0.), ElectricCurrent::new::<ampere>(0.))
    }

    fn direct_current() -> Current {
        Current::Direct(ElectricPotential::new::<volt>(0.), ElectricCurrent::new::<ampere>(0.))
    }

    fn none_current() -> Current {
        Current::None
    }
}

#[cfg(test)]
mod contactor_tests {
    use super::*;

    #[test]
    fn contactor_starts_open() {
        assert_eq!(contactor().state, ContactorState::Open);
    }

    #[test]
    fn open_contactor_when_toggled_open_stays_open() {
        let mut contactor = open_contactor();
        contactor.toggle(false);

        assert_eq!(contactor.state, ContactorState::Open);
    }

    #[test]
    fn open_contactor_when_toggled_closed_closes() {
        let mut contactor = open_contactor();
        contactor.toggle(true);

        assert_eq!(contactor.state, ContactorState::Closed);
    }

    #[test]
    fn closed_contactor_when_toggled_open_opens() {
        let mut contactor = closed_contactor();
        contactor.toggle(false);

        assert_eq!(contactor.state, ContactorState::Open);
    }

    #[test]
    fn closed_contactor_when_toggled_closed_stays_closed() {
        let mut contactor = closed_contactor();
        contactor.toggle(true);

        assert_eq!(contactor.state, ContactorState::Closed);
    }

    fn contactor() -> Contactor {
        Contactor::new()
    }

    fn open_contactor() -> Contactor {
        let mut contactor = contactor();
        contactor.state = ContactorState::Open;

        contactor
    }

    fn closed_contactor() -> Contactor {
        let mut contactor = contactor();
        contactor.state = ContactorState::Closed;

        contactor
    }
}