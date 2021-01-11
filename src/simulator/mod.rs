use crate::state::{SimVisitor, SimulatorReadState, SimulatorReadWritable, SimulatorWriteState};
use msfs::legacy::{AircraftVariable, NamedVariable};
use uom::si::{ratio::percent, thermodynamic_temperature::degree_celsius};

pub struct SimulatorReadWriter {
    apu_master_sw: AircraftVariable,
    apu_start_sw: NamedVariable,
    apu_n: NamedVariable,
    apu_egt: NamedVariable,
    apu_egt_caution: NamedVariable,
    apu_egt_warning: NamedVariable,
}
impl SimulatorReadWriter {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(SimulatorReadWriter {
            apu_master_sw: AircraftVariable::from("FUELSYSTEM VALVE SWITCH", "Bool", 8)?,
            apu_start_sw: NamedVariable::from("A32NX_APU_START_ACTIVATED"),
            apu_n: NamedVariable::from("APU_N"),
            apu_egt: NamedVariable::from("APU_EGT"),
            apu_egt_caution: NamedVariable::from("APU_EGT_WARN"),
            apu_egt_warning: NamedVariable::from("APU_EGT_MAX"),
        })
    }

    pub fn read(&self) -> SimulatorReadState {
        SimulatorReadState {
            apu_master_sw_on: to_bool(self.apu_master_sw.get()),
            apu_start_sw_on: to_bool(self.apu_start_sw.get_value()),
            apu_bleed_sw_on: true, // TODO
        }
    }

    pub fn write(&self, state: &SimulatorWriteState) {
        self.apu_n.set_value(state.apu_n.get::<percent>());
        self.apu_egt
            .set_value(state.apu_egt.get::<degree_celsius>());
        self.apu_egt_caution
            .set_value(state.apu_caution_egt.get::<degree_celsius>());
        self.apu_egt_warning
            .set_value(state.apu_warning_egt.get::<degree_celsius>());
    }
}

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

fn to_bool(value: f64) -> bool {
    value == 1.
}
