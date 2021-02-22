use super::AirIntakeFlapController;
use crate::{shared::random_number, simulation::UpdateContext};
use std::time::Duration;
use uom::si::{f64::*, ratio::percent};

#[derive(PartialEq)]
enum AirIntakeFlapState {
    Closed,
    Open,
}

pub struct AirIntakeFlap {
    open_amount: Ratio,
    delay: Duration,
    last_state: AirIntakeFlapState,
}
impl AirIntakeFlap {
    const MINIMUM_TRAVEL_TIME_SECS: u8 = 6;
    const MAXIMUM_TRAVEL_TIME_SECS: u8 = 12;

    pub fn new() -> AirIntakeFlap {
        let random_above_minimum_mod =
            AirIntakeFlap::MAXIMUM_TRAVEL_TIME_SECS - AirIntakeFlap::MINIMUM_TRAVEL_TIME_SECS + 1;
        let delay = Duration::from_secs(
            (AirIntakeFlap::MINIMUM_TRAVEL_TIME_SECS + (random_number() % random_above_minimum_mod))
                as u64,
        );

        AirIntakeFlap {
            open_amount: Ratio::new::<percent>(0.),
            delay,
            last_state: AirIntakeFlapState::Closed,
        }
    }

    pub fn update<T: AirIntakeFlapController>(&mut self, context: &UpdateContext, controller: &T) {
        if controller.should_open_air_intake_flap()
            && self.open_amount < Ratio::new::<percent>(100.)
        {
            self.open_amount += Ratio::new::<percent>(
                self.get_flap_change_for_delta(context)
                    .min(100. - self.open_amount.get::<percent>()),
            );

            if (self.open_amount.get::<percent>() - 100.).abs() < f64::EPSILON {
                self.last_state = AirIntakeFlapState::Open;
            }
        } else if !controller.should_open_air_intake_flap()
            && self.open_amount > Ratio::new::<percent>(0.)
        {
            self.open_amount -= Ratio::new::<percent>(
                self.get_flap_change_for_delta(context)
                    .min(self.open_amount.get::<percent>()),
            );

            if (self.open_amount.get::<percent>() - 0.).abs() < f64::EPSILON {
                self.last_state = AirIntakeFlapState::Closed;
            }
        }
    }

    fn get_flap_change_for_delta(&self, context: &UpdateContext) -> f64 {
        100. * (context.delta.as_secs_f64() / self.delay.as_secs_f64())
    }

    pub fn is_fully_open(&self) -> bool {
        self.open_amount == Ratio::new::<percent>(100.)
    }

    pub fn open_amount(&self) -> Ratio {
        self.open_amount
    }

    /// Determines if the the flap is open, as per the definition that is used
    /// for displaying the "FLAP OPEN" message on the APU ECAM.
    /// Returns true when:
    /// 1. The flap is fully open
    /// 2. The flap was fully open and is closing, but not fully closed.
    /// 3. The flap was fully open, started closing, but started opening again before fully closing.
    /// Returns false otherwise.
    pub fn is_apu_ecam_open(&self) -> bool {
        self.last_state == AirIntakeFlapState::Open
    }

    #[cfg(test)]
    pub fn set_delay(&mut self, delay: Duration) {
        self.delay = delay;
    }
}

#[cfg(test)]
mod air_intake_flap_tests {
    use super::*;
    use crate::simulation::test::SimulationTestBed;
    use crate::simulation::{Aircraft, SimulationElement};

    struct TestAircraft {
        flap: AirIntakeFlap,
        controller: TestFlapController,
    }
    impl TestAircraft {
        fn new(flap: AirIntakeFlap, controller: TestFlapController) -> Self {
            Self { flap, controller }
        }

        fn command_flap_open(&mut self) {
            self.controller.open();
        }

        fn command_flap_close(&mut self) {
            self.controller.close();
        }

        fn flap_open_amount(&self) -> Ratio {
            self.flap.open_amount()
        }

        fn flap_is_fully_open(&self) -> bool {
            self.flap.is_fully_open()
        }
    }
    impl Aircraft for TestAircraft {
        fn update_before_power_distribution(&mut self, context: &UpdateContext) {
            self.flap.update(context, &self.controller);
        }
    }
    impl SimulationElement for TestAircraft {}

    struct TestFlapController {
        should_open: bool,
    }
    impl TestFlapController {
        fn new() -> Self {
            TestFlapController { should_open: false }
        }

        fn open(&mut self) {
            self.should_open = true;
        }

        fn close(&mut self) {
            self.should_open = false;
        }
    }
    impl AirIntakeFlapController for TestFlapController {
        fn should_open_air_intake_flap(&self) -> bool {
            self.should_open
        }
    }

    #[test]
    fn starts_opening_when_target_is_open() {
        let mut aircraft = TestAircraft::new(AirIntakeFlap::new(), TestFlapController::new());
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_secs(5));

        aircraft.command_flap_open();
        test_bed.run_aircraft(&mut aircraft);

        assert!(aircraft.flap_open_amount().get::<percent>() > 0.);
    }

    #[test]
    fn does_not_instantly_open() {
        let mut aircraft = TestAircraft::new(AirIntakeFlap::new(), TestFlapController::new());
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_secs(
            (AirIntakeFlap::MINIMUM_TRAVEL_TIME_SECS - 1) as u64,
        ));

        aircraft.command_flap_open();
        test_bed.run_aircraft(&mut aircraft);

        assert!(aircraft.flap_open_amount().get::<percent>() < 100.);
    }

    #[test]
    fn closes_when_target_is_closed() {
        let mut aircraft = TestAircraft::new(AirIntakeFlap::new(), TestFlapController::new());
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_secs(5));

        aircraft.command_flap_open();
        test_bed.run_aircraft(&mut aircraft);

        let flap_open_amount = aircraft.flap_open_amount();

        aircraft.command_flap_close();
        test_bed
            .delta(Duration::from_secs(2))
            .run_aircraft(&mut aircraft);

        assert!(aircraft.flap_open_amount() < flap_open_amount);
    }

    #[test]
    fn does_not_instantly_close() {
        let mut aircraft = TestAircraft::new(AirIntakeFlap::new(), TestFlapController::new());
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_secs(
            AirIntakeFlap::MAXIMUM_TRAVEL_TIME_SECS as u64,
        ));

        aircraft.command_flap_open();
        test_bed.run_aircraft(&mut aircraft);

        aircraft.command_flap_close();
        test_bed
            .delta(Duration::from_secs(
                (AirIntakeFlap::MINIMUM_TRAVEL_TIME_SECS - 1) as u64,
            ))
            .run_aircraft(&mut aircraft);

        assert!(aircraft.flap_open_amount().get::<percent>() > 0.);
    }

    #[test]
    fn never_closes_beyond_0_percent() {
        let mut aircraft = TestAircraft::new(AirIntakeFlap::new(), TestFlapController::new());
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_secs(1_000));

        aircraft.command_flap_close();
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(aircraft.flap_open_amount(), Ratio::new::<percent>(0.));
    }

    #[test]
    fn never_opens_beyond_100_percent() {
        let mut aircraft = TestAircraft::new(AirIntakeFlap::new(), TestFlapController::new());
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_secs(1_000));

        aircraft.command_flap_open();
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(aircraft.flap_open_amount(), Ratio::new::<percent>(100.));
    }

    #[test]
    fn is_fully_open_returns_false_when_closed() {
        let aircraft = TestAircraft::new(AirIntakeFlap::new(), TestFlapController::new());

        assert_eq!(aircraft.flap_is_fully_open(), false)
    }

    #[test]
    fn is_fully_open_returns_true_when_open() {
        let mut aircraft = TestAircraft::new(AirIntakeFlap::new(), TestFlapController::new());
        let mut test_bed = SimulationTestBed::new().delta(Duration::from_secs(1_000));

        aircraft.command_flap_open();
        test_bed.run_aircraft(&mut aircraft);

        assert_eq!(aircraft.flap_is_fully_open(), true)
    }
}
