use rand::prelude::*;
use uom::si::{f64::*, ratio::percent};

use crate::{overhead::OnOffPushButton, shared::UpdateContext, visitor::Visitable};

pub struct AuxiliaryPowerUnitOverheadPanel {
    master: OnOffPushButton,
    start: OnOffPushButton,
}
impl AuxiliaryPowerUnitOverheadPanel {
    pub fn new() -> AuxiliaryPowerUnitOverheadPanel {
        AuxiliaryPowerUnitOverheadPanel {
            master: OnOffPushButton::new_off(),
            start: OnOffPushButton::new_off(),
        }
    }

    fn master_is_on(&self) -> bool {
        self.master.is_on()
    }
}

pub struct AuxiliaryPowerUnit {
    pub n: Ratio,
    air_intake_flap: AirIntakeFlap,
}

impl AuxiliaryPowerUnit {
    pub fn new() -> AuxiliaryPowerUnit {
        AuxiliaryPowerUnit {
            n: Ratio::new::<percent>(0.),
            air_intake_flap: AirIntakeFlap::new(),
        }
    }

    pub fn update(&mut self, context: &UpdateContext, overhead: &AuxiliaryPowerUnitOverheadPanel) {
        if overhead.master_is_on() {
            self.air_intake_flap.open();
        } else {
            self.air_intake_flap.close();
        }

        self.air_intake_flap.update(context);
    }
}

impl Visitable for AuxiliaryPowerUnit {
    fn accept(&mut self, visitor: &mut Box<dyn crate::visitor::MutableVisitor>) {
        visitor.visit_auxiliary_power_unit(self);
    }
}

#[derive(PartialEq)]
enum AirIntakeFlapTarget {
    Open,
    Closed,
}

struct AirIntakeFlap {
    state: Ratio,
    target: AirIntakeFlapTarget,
    delay: i32,
}
impl AirIntakeFlap {
    fn new() -> AirIntakeFlap {
        let mut rng = rand::thread_rng();
        let delay = 3 + rng.gen_range(0..13);

        AirIntakeFlap {
            state: Ratio::new::<percent>(0.),
            target: AirIntakeFlapTarget::Closed,
            delay,
        }
    }

    fn update(&mut self, context: &UpdateContext) {
        if self.target == AirIntakeFlapTarget::Open && self.state < Ratio::new::<percent>(100.) {
            self.state += Ratio::new::<percent>(
                self.get_flap_change_for_delta(context)
                    .min(100. - self.state.get::<percent>()),
            );
        } else if self.target == AirIntakeFlapTarget::Closed
            && self.state > Ratio::new::<percent>(0.)
        {
            self.state -= Ratio::new::<percent>(
                self.get_flap_change_for_delta(context)
                    .max(self.state.get::<percent>()),
            );
        }
    }

    fn get_flap_change_for_delta(&self, context: &UpdateContext) -> f64 {
        100. * (context.delta.as_secs_f64() / self.delay as f64)
    }

    fn open(&mut self) {
        self.target = AirIntakeFlapTarget::Open;
    }

    fn close(&mut self) {
        self.target = AirIntakeFlapTarget::Closed;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use uom::si::{length::foot, thermodynamic_temperature::degree_celsius, velocity::knot};

    use super::*;

    fn context(delta: Duration) -> UpdateContext {
        UpdateContext::new(
            delta,
            Velocity::new::<knot>(250.),
            Length::new::<foot>(5000.),
            ThermodynamicTemperature::new::<degree_celsius>(0.),
        )
    }

    #[cfg(test)]
    mod apu_tests {
        use super::*;

        #[test]
        fn when_apu_master_sw_turned_on_air_intake_flap_opens() {
            let mut apu = AuxiliaryPowerUnit::new();
            let mut overhead = AuxiliaryPowerUnitOverheadPanel::new();
            overhead.master.push_on();

            apu.update(&context(Duration::from_secs(5)), &overhead);

            assert!(apu.air_intake_flap.state.get::<percent>() > 0.);
        }

        #[test]
        #[ignore]
        fn when_start_sw_on_when_air_intake_flap_fully_open_starting_sequence_commences() {}

        #[test]
        #[ignore]
        fn start_sw_on_light_turns_off_when_n_above_99_5() {
            // Note should also test 2 seconds after reaching 95 the light turns off?
        }

        #[test]
        #[ignore]
        fn start_sw_avail_light_turns_on_when_n_above_99_5() {
            // Note should also test 2 seconds after reaching 95 the light turns off?
        }

        #[test]
        #[ignore]
        fn when_egt_is_greater_than_egt_max_automatic_shutdown_begins() {
            // Note should also test 2 seconds after reaching 95 the light turns off?
        }

        #[test]
        #[ignore]
        fn when_apu_master_sw_turned_off_avail_on_start_pb_goes_off() {}

        #[test]
        #[ignore]
        fn when_apu_master_sw_turned_off_if_apu_bleed_air_was_used_apu_keeps_running_for_60_second_cooldown(
        ) {
        }

        #[test]
        #[ignore]
        fn when_apu_shutting_down_at_7_percent_air_inlet_flap_closes() {}
    }

    #[cfg(test)]
    mod air_intake_flap_tests {
        use super::*;

        #[test]
        fn starts_opening_when_target_is_open() {
            let mut flap = AirIntakeFlap::new();
            flap.open();

            flap.update(&context(Duration::from_secs(5)));

            assert!(flap.state.get::<percent>() > 0.);
        }

        #[test]
        fn closes_when_target_is_closed() {
            let mut flap = AirIntakeFlap::new();
            flap.open();
            flap.update(&context(Duration::from_secs(5)));
            let open_percentage = flap.state.get::<percent>();

            flap.close();
            flap.update(&context(Duration::from_secs(2)));

            assert!(flap.state.get::<percent>() < open_percentage);
        }

        #[test]
        fn never_closes_beyond_0_percent() {
            let mut flap = AirIntakeFlap::new();
            flap.close();
            flap.update(&context(Duration::from_secs(1000)));

            assert_eq!(flap.state.get::<percent>(), 0.);
        }

        #[test]
        fn never_opens_beyond_100_percent() {
            let mut flap = AirIntakeFlap::new();
            flap.open();
            flap.update(&context(Duration::from_secs(1000)));

            assert_eq!(flap.state.get::<percent>(), 100.);
        }
    }
}
