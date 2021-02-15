macro_rules! powerable {
    ($t: ty) => {
        impl Powerable for $t {
            fn powered_by<T: PowerSource + ?Sized>(&mut self, source: &T) {
                self.input = source.output_potential();
            }

            fn or_powered_by<T: PowerSource + ?Sized>(&mut self, source: &T) {
                if self.input.is_unpowered() {
                    self.powered_by(source);
                }
            }
        }
    };
}
