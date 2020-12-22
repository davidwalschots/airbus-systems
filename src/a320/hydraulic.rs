pub struct A320HydraulicCircuit {
    // Until hydraulic is implemented, we'll fake it with this boolean.
    blue_pressurised: bool,
}

impl A320HydraulicCircuit {
    pub fn new() -> A320HydraulicCircuit {
        A320HydraulicCircuit {
            blue_pressurised: true,
        }
    }

    pub fn is_blue_pressurised(&self) -> bool {
        self.blue_pressurised
    }
}
