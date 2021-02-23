macro_rules! potential_target {
    ($t: ty) => {
        impl PotentialTarget for $t {
            fn powered_by<T: PotentialSource + ?Sized>(&mut self, source: &T) {
                self.input = source.output_potential();
            }

            fn or_powered_by<T: PotentialSource + ?Sized>(&mut self, source: &T) {
                if self.input.is_unpowered() {
                    self.powered_by(source);
                }
            }
        }
    };
}

/// Provide potential with a normal range of 390 Hz to 410 Hz.
macro_rules! provide_frequency {
    ($t: ty) => {
        impl ProvideFrequency for $t {
            fn frequency(&self) -> Frequency {
                self.frequency
            }

            fn frequency_normal(&self) -> bool {
                let hz = self.frequency.get::<hertz>();
                (390.0..=410.0).contains(&hz)
            }
        }
    };
}

/// Provide load with a normal range of 0% to 100%.
macro_rules! provide_load {
    ($t: ty) => {
        impl ProvideLoad for $t {
            fn load(&self) -> Ratio {
                self.load
            }

            fn load_normal(&self) -> bool {
                self.load <= Ratio::new::<percent>(100.)
            }
        }
    };
}

/// Provide potential with a normal range of 110 V to 120 V.
macro_rules! provide_potential {
    ($t: ty) => {
        impl ProvidePotential for $t {
            fn potential(&self) -> ElectricPotential {
                self.potential
            }

            fn potential_normal(&self) -> bool {
                let volts = self.potential.get::<volt>();
                (110.0..=120.0).contains(&volts)
            }
        }
    };
}
