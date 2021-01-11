use uom::si::f64::*;

pub trait SimulatorReadWritable {
    /// Reads simulator state data into the struct.
    fn read(&mut self, state: &SimulatorReadState) {}

    /// Writes struct data into the simulator's state.
    fn write(&self, state: &mut SimulatorWriteState) {}
}

pub trait SimulatorVisitable {
    fn accept<T: SimVisitor>(&mut self, visitor: &mut T);
}

pub trait SimVisitor {
    fn visit<T: SimulatorReadWritable>(&mut self, visited: &mut T);
}

#[derive(Default)]
pub struct SimulatorReadState {
    pub apu_master_sw_on: bool,
    pub apu_start_sw_on: bool,
    pub apu_bleed_sw_on: bool,
}

#[derive(Default)]
pub struct SimulatorWriteState {
    pub apu_n: Ratio,
    pub apu_egt: ThermodynamicTemperature,
    pub apu_caution_egt: ThermodynamicTemperature,
    pub apu_warning_egt: ThermodynamicTemperature,
}
