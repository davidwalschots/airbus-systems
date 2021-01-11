use crate::state::{SimVisitor, SimulatorReadState, SimulatorReadWritable, SimulatorWriteState};

pub struct SimToModelVisitor {
    state: SimulatorReadState,
}
impl SimToModelVisitor {
    pub fn new(state: SimulatorReadState) -> Self {
        SimToModelVisitor { state }
    }
}
impl SimVisitor for SimToModelVisitor {
    fn visit<T: SimulatorReadWritable>(&mut self, visited: &mut T) {
        visited.read(&self.state);
    }
}

pub struct ModelToSimVisitor {
    state: SimulatorWriteState,
}
impl ModelToSimVisitor {
    pub fn new() -> Self {
        ModelToSimVisitor {
            state: Default::default(),
        }
    }

    pub fn get_state(self) -> SimulatorWriteState {
        self.state
    }
}
impl SimVisitor for ModelToSimVisitor {
    fn visit<T: SimulatorReadWritable>(&mut self, visited: &mut T) {
        visited.write(&mut self.state);
    }
}

pub fn to_bool(value: f64) -> bool {
    value == 1.
}
