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

#[cfg(test)]
mod tests {
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