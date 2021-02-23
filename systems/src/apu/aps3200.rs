use super::{ApuGenerator, Turbine, TurbineController, TurbineState};
use crate::{
    electrical::{
        consumption::PowerConsumptionReport, ElectricalStateWriter, Potential, PotentialSource,
        ProvideFrequency, ProvideLoad, ProvidePotential,
    },
    shared::{calculate_towards_target_temperature, random_number},
    simulation::{SimulationElement, SimulatorWriter, UpdateContext},
};
use std::time::Duration;
use uom::si::{
    electric_potential::volt, f64::*, frequency::hertz, power::watt, ratio::percent,
    temperature_interval, thermodynamic_temperature::degree_celsius,
};

pub struct ShutdownAps3200Turbine {
    egt: ThermodynamicTemperature,
}
impl ShutdownAps3200Turbine {
    pub fn new() -> Self {
        ShutdownAps3200Turbine {
            egt: ThermodynamicTemperature::new::<degree_celsius>(0.),
        }
    }

    fn new_with_egt(egt: ThermodynamicTemperature) -> Self {
        ShutdownAps3200Turbine { egt }
    }
}
impl Turbine for ShutdownAps3200Turbine {
    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        _: bool,
        _: bool,
        controller: &dyn TurbineController,
    ) -> Box<dyn Turbine> {
        self.egt = calculate_towards_ambient_egt(self.egt, context);

        if controller.should_start() {
            Box::new(Starting::new(self.egt))
        } else {
            self
        }
    }

    fn n(&self) -> Ratio {
        Ratio::new::<percent>(0.)
    }

    fn egt(&self) -> ThermodynamicTemperature {
        self.egt
    }

    fn state(&self) -> TurbineState {
        TurbineState::Shutdown
    }
}

struct Starting {
    since: Duration,
    n: Ratio,
    egt: ThermodynamicTemperature,
    ignore_calculated_egt: bool,
}
impl Starting {
    fn new(egt: ThermodynamicTemperature) -> Starting {
        Starting {
            since: Duration::from_secs(0),
            n: Ratio::new::<percent>(0.),
            egt,
            ignore_calculated_egt: true,
        }
    }

    fn calculate_egt(&mut self, context: &UpdateContext) -> ThermodynamicTemperature {
        // Refer to APS3200.md for details on the values below and source data.
        const APU_N_TEMP_CONST: f64 = 0.8260770092912485;
        const APU_N_TEMP_X: f64 = -10.521171805148322;
        const APU_N_TEMP_X2: f64 = 9.991789425954353;
        const APU_N_TEMP_X3: f64 = -3.082752847935092;
        const APU_N_TEMP_X4: f64 = 0.4261454295059484;
        const APU_N_TEMP_X5: f64 = -0.03117154621503877;
        const APU_N_TEMP_X6: f64 = 0.0013843186755010547;
        const APU_N_TEMP_X7: f64 = -0.00004016856934546301;
        const APU_N_TEMP_X8: f64 = 0.00000078892955962222;
        const APU_N_TEMP_X9: f64 = -0.00000001058955825891;
        const APU_N_TEMP_X10: f64 = 0.00000000009582985112;
        const APU_N_TEMP_X11: f64 = -0.00000000000055952490;
        const APU_N_TEMP_X12: f64 = 0.00000000000000190415;
        const APU_N_TEMP_X13: f64 = -0.00000000000000000287;

        let n = self.n.get::<percent>();

        // Results below this value momentarily go above 0, while not intended.
        if n < 5.5 {
            calculate_towards_ambient_egt(self.egt, context)
        } else {
            let temperature = ThermodynamicTemperature::new::<degree_celsius>(
                APU_N_TEMP_CONST
                    + (APU_N_TEMP_X * n)
                    + (APU_N_TEMP_X2 * n.powi(2))
                    + (APU_N_TEMP_X3 * n.powi(3))
                    + (APU_N_TEMP_X4 * n.powi(4))
                    + (APU_N_TEMP_X5 * n.powi(5))
                    + (APU_N_TEMP_X6 * n.powi(6))
                    + (APU_N_TEMP_X7 * n.powi(7))
                    + (APU_N_TEMP_X8 * n.powi(8))
                    + (APU_N_TEMP_X9 * n.powi(9))
                    + (APU_N_TEMP_X10 * n.powi(10))
                    + (APU_N_TEMP_X11 * n.powi(11))
                    + (APU_N_TEMP_X12 * n.powi(12))
                    + (APU_N_TEMP_X13 * n.powi(13)),
            );

            // The above calculated EGT can be lower than the ambient temperature,
            // or the current APU EGT (when cooling down). To prevent sudden changes
            // in temperature, we ignore the calculated EGT until it exceeds the current
            // EGT.
            let towards_ambient_egt = calculate_towards_ambient_egt(self.egt, context);
            if temperature > towards_ambient_egt {
                self.ignore_calculated_egt = false;
            }

            if self.ignore_calculated_egt {
                towards_ambient_egt
            } else {
                temperature
            }
        }
    }

    fn calculate_n(&self) -> Ratio {
        const APU_N_CONST: f64 = -0.08013606018640967;
        const APU_N_X: f64 = 2.129832736394534;
        const APU_N_X2: f64 = 3.928273438786404;
        const APU_N_X3: f64 = -1.88613299921213;
        const APU_N_X4: f64 = 0.42749452749180916;
        const APU_N_X5: f64 = -0.05757707967690426;
        const APU_N_X6: f64 = 0.005022142795451004;
        const APU_N_X7: f64 = -0.00029612873626050866;
        const APU_N_X8: f64 = 0.00001204152497871946;
        const APU_N_X9: f64 = -0.00000033829604438116;
        const APU_N_X10: f64 = 0.00000000645140818528;
        const APU_N_X11: f64 = -0.00000000007974743535;
        const APU_N_X12: f64 = 0.00000000000057654695;
        const APU_N_X13: f64 = -0.00000000000000185126;

        // Protect against the formula returning decreasing results after this value.
        const TIME_LIMIT: f64 = 45.12;
        const START_IGNITION_AFTER_SECONDS: f64 = 1.5;
        let ignition_turned_on_secs =
            (self.since.as_secs_f64() - START_IGNITION_AFTER_SECONDS).min(TIME_LIMIT);

        if ignition_turned_on_secs > 0. {
            let n = (APU_N_CONST
                + (APU_N_X * ignition_turned_on_secs)
                + (APU_N_X2 * ignition_turned_on_secs.powi(2))
                + (APU_N_X3 * ignition_turned_on_secs.powi(3))
                + (APU_N_X4 * ignition_turned_on_secs.powi(4))
                + (APU_N_X5 * ignition_turned_on_secs.powi(5))
                + (APU_N_X6 * ignition_turned_on_secs.powi(6))
                + (APU_N_X7 * ignition_turned_on_secs.powi(7))
                + (APU_N_X8 * ignition_turned_on_secs.powi(8))
                + (APU_N_X9 * ignition_turned_on_secs.powi(9))
                + (APU_N_X10 * ignition_turned_on_secs.powi(10))
                + (APU_N_X11 * ignition_turned_on_secs.powi(11))
                + (APU_N_X12 * ignition_turned_on_secs.powi(12))
                + (APU_N_X13 * ignition_turned_on_secs.powi(13)))
            .min(100.)
            .max(0.);

            Ratio::new::<percent>(n)
        } else {
            Ratio::new::<percent>(0.)
        }
    }
}
impl Turbine for Starting {
    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        _: bool,
        _: bool,
        controller: &dyn TurbineController,
    ) -> Box<dyn Turbine> {
        self.since += context.delta;
        self.n = self.calculate_n();
        self.egt = self.calculate_egt(context);

        if controller.should_stop() {
            Box::new(Stopping::new(self.egt, self.n))
        } else if (self.n.get::<percent>() - 100.).abs() < f64::EPSILON {
            Box::new(Running::new(self.egt))
        } else {
            self
        }
    }

    fn n(&self) -> Ratio {
        self.n
    }

    fn egt(&self) -> ThermodynamicTemperature {
        self.egt
    }

    fn state(&self) -> TurbineState {
        TurbineState::Starting
    }
}

struct Running {
    egt: ThermodynamicTemperature,
    base_temperature: ThermodynamicTemperature,
    bleed_air_in_use_delta_temperature: TemperatureInterval,
    apu_gen_in_use_delta_temperature: TemperatureInterval,
}
impl Running {
    fn new(egt: ThermodynamicTemperature) -> Running {
        let base_temperature = 340. + ((random_number() % 11) as f64);
        let bleed_air_in_use_delta_temperature = 30. + ((random_number() % 11) as f64);
        let apu_gen_in_use_delta_temperature = 10. + ((random_number() % 6) as f64);
        Running {
            egt,
            base_temperature: ThermodynamicTemperature::new::<degree_celsius>(base_temperature),
            bleed_air_in_use_delta_temperature: TemperatureInterval::new::<
                temperature_interval::degree_celsius,
            >(bleed_air_in_use_delta_temperature),
            apu_gen_in_use_delta_temperature: TemperatureInterval::new::<
                temperature_interval::degree_celsius,
            >(apu_gen_in_use_delta_temperature),
        }
    }

    fn calculate_slow_cooldown_to_running_temperature(
        &self,
        context: &UpdateContext,
        apu_gen_is_used: bool,
        apu_bleed_is_used: bool,
    ) -> ThermodynamicTemperature {
        let mut target_temperature = self.base_temperature;
        if apu_bleed_is_used {
            target_temperature += self.bleed_air_in_use_delta_temperature;
        }
        if apu_gen_is_used {
            target_temperature += self.apu_gen_in_use_delta_temperature;
        }

        calculate_towards_target_temperature(self.egt, target_temperature, 0.4, context.delta)
    }
}
impl Turbine for Running {
    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        apu_bleed_is_used: bool,
        apu_gen_is_used: bool,
        controller: &dyn TurbineController,
    ) -> Box<dyn Turbine> {
        self.egt = self.calculate_slow_cooldown_to_running_temperature(
            context,
            apu_gen_is_used,
            apu_bleed_is_used,
        );

        if controller.should_stop() {
            Box::new(Stopping::new(self.egt, Ratio::new::<percent>(100.)))
        } else {
            self
        }
    }

    fn n(&self) -> Ratio {
        Ratio::new::<percent>(100.)
    }

    fn egt(&self) -> ThermodynamicTemperature {
        self.egt
    }

    fn state(&self) -> TurbineState {
        TurbineState::Running
    }
}

struct Stopping {
    since: Duration,
    n: Ratio,
    egt: ThermodynamicTemperature,
}
impl Stopping {
    fn new(egt: ThermodynamicTemperature, n: Ratio) -> Stopping {
        Stopping {
            since: Duration::from_secs(0),
            n,
            egt,
        }
    }

    fn calculate_n(&self, context: &UpdateContext) -> Ratio {
        const SPOOL_DOWN_COEFFICIENT: f64 = 2.;
        let mut n = self.n.get::<percent>();
        n = (n - (context.delta.as_secs_f64() * SPOOL_DOWN_COEFFICIENT)).max(0.);

        Ratio::new::<percent>(n)
    }
}
impl Turbine for Stopping {
    fn update(
        mut self: Box<Self>,
        context: &UpdateContext,
        _: bool,
        _: bool,
        _: &dyn TurbineController,
    ) -> Box<dyn Turbine> {
        self.since += context.delta;
        self.n = self.calculate_n(context);
        self.egt = calculate_towards_ambient_egt(self.egt, context);

        if self.n.get::<percent>() == 0. {
            Box::new(ShutdownAps3200Turbine::new_with_egt(self.egt))
        } else {
            self
        }
    }

    fn n(&self) -> Ratio {
        self.n
    }

    fn egt(&self) -> ThermodynamicTemperature {
        self.egt
    }

    fn state(&self) -> TurbineState {
        TurbineState::Stopping
    }
}

fn calculate_towards_ambient_egt(
    current_egt: ThermodynamicTemperature,
    context: &UpdateContext,
) -> ThermodynamicTemperature {
    const APU_AMBIENT_COEFFICIENT: f64 = 2.;
    calculate_towards_target_temperature(
        current_egt,
        context.ambient_temperature,
        APU_AMBIENT_COEFFICIENT,
        context.delta,
    )
}

/// APS3200 APU Generator
pub struct Aps3200ApuGenerator {
    number: usize,
    n: Ratio,
    writer: ElectricalStateWriter,
    output: Potential,
    frequency: Frequency,
    potential: ElectricPotential,
    load: Ratio,
}
impl Aps3200ApuGenerator {
    const APU_GEN_POWERED_N: f64 = 84.;

    pub fn new(number: usize) -> Aps3200ApuGenerator {
        Aps3200ApuGenerator {
            number,
            n: Ratio::new::<percent>(0.),
            writer: ElectricalStateWriter::new(&format!("APU_GEN_{}", number)),
            output: Potential::None,
            potential: ElectricPotential::new::<volt>(0.),
            frequency: Frequency::new::<hertz>(0.),
            load: Ratio::new::<percent>(0.),
        }
    }

    fn calculate_potential(&self, n: Ratio) -> ElectricPotential {
        let n = n.get::<percent>();

        if n < Aps3200ApuGenerator::APU_GEN_POWERED_N {
            panic!("Should not be invoked for APU N below {}", n);
        } else if n < 85. {
            ElectricPotential::new::<volt>(105.)
        } else {
            ElectricPotential::new::<volt>(115.)
        }
    }

    fn calculate_frequency(&self, n: Ratio) -> Frequency {
        let n = n.get::<percent>();

        // Refer to APS3200.md for details on the values below and source data.
        if n < Aps3200ApuGenerator::APU_GEN_POWERED_N {
            panic!("Should not be invoked for APU N below {}", n);
        } else if n < 100. {
            const APU_FREQ_CONST: f64 = 1076894372064.8204;
            const APU_FREQ_X: f64 = -118009165327.71873;
            const APU_FREQ_X2: f64 = 5296044666.7118;
            const APU_FREQ_X3: f64 = -108419965.09400678;
            const APU_FREQ_X4: f64 = -36793.31899267512;
            const APU_FREQ_X5: f64 = 62934.36386220135;
            const APU_FREQ_X6: f64 = -1870.5197158547767;
            const APU_FREQ_X7: f64 = 31.376473743149806;
            const APU_FREQ_X8: f64 = -0.3510150716459761;
            const APU_FREQ_X9: f64 = 0.002726493614147866;
            const APU_FREQ_X10: f64 = -0.00001463272647792659;
            const APU_FREQ_X11: f64 = 0.00000005203375009496;
            const APU_FREQ_X12: f64 = -0.00000000011071318044;
            const APU_FREQ_X13: f64 = 0.00000000000010697005;

            Frequency::new::<hertz>(
                APU_FREQ_CONST
                    + (APU_FREQ_X * n)
                    + (APU_FREQ_X2 * n.powi(2))
                    + (APU_FREQ_X3 * n.powi(3))
                    + (APU_FREQ_X4 * n.powi(4))
                    + (APU_FREQ_X5 * n.powi(5))
                    + (APU_FREQ_X6 * n.powi(6))
                    + (APU_FREQ_X7 * n.powi(7))
                    + (APU_FREQ_X8 * n.powi(8))
                    + (APU_FREQ_X9 * n.powi(9))
                    + (APU_FREQ_X10 * n.powi(10))
                    + (APU_FREQ_X11 * n.powi(11))
                    + (APU_FREQ_X12 * n.powi(12))
                    + (APU_FREQ_X13 * n.powi(13)),
            )
        } else {
            Frequency::new::<hertz>(400.)
        }
    }
}
impl ApuGenerator for Aps3200ApuGenerator {
    fn update(&mut self, n: Ratio, is_emergency_shutdown: bool) {
        self.n = n;

        self.output = if is_emergency_shutdown
            || n.get::<percent>() < Aps3200ApuGenerator::APU_GEN_POWERED_N
        {
            Potential::None
        } else {
            Potential::ApuGenerator(self.number)
        };
    }
}
provide_potential!(Aps3200ApuGenerator, (110.0..=120.0));
provide_frequency!(Aps3200ApuGenerator, (390.0..=410.0));
provide_load!(Aps3200ApuGenerator);
impl PotentialSource for Aps3200ApuGenerator {
    fn output_potential(&self) -> Potential {
        self.output
    }
}
impl SimulationElement for Aps3200ApuGenerator {
    fn write(&self, writer: &mut SimulatorWriter) {
        self.writer.write_alternating_with_load(self, writer);
    }

    fn process_power_consumption_report<T: PowerConsumptionReport>(
        &mut self,
        report: &T,
        _: &UpdateContext,
    ) {
        self.potential = if self.is_powered() {
            self.calculate_potential(self.n)
        } else {
            ElectricPotential::new::<volt>(0.)
        };

        self.frequency = if self.is_powered() {
            self.calculate_frequency(self.n)
        } else {
            Frequency::new::<hertz>(0.)
        };

        let power_consumption = report
            .total_consumption_of(&self.output_potential())
            .get::<watt>();
        let power_factor_correction = 0.8;
        let maximum_load = 90000.;
        self.load = Ratio::new::<percent>(
            (power_consumption * power_factor_correction / maximum_load) * 100.,
        );
    }
}

#[cfg(test)]
mod apu_generator_tests {
    use ntest::assert_about_eq;
    use uom::si::frequency::hertz;

    use crate::{
        apu::tests::{test_bed, test_bed_with},
        simulation::test::SimulationTestBed,
    };

    use super::*;

    #[test]
    fn starts_without_output() {
        assert!(apu_generator().is_unpowered());
    }

    #[test]
    fn when_apu_running_provides_output() {
        let mut generator = apu_generator();
        let mut test_bed = SimulationTestBed::new();
        update_below_threshold(&mut test_bed, &mut generator);
        update_above_threshold(&mut test_bed, &mut generator);

        assert!(generator.is_powered());
    }

    #[test]
    fn when_apu_shutdown_provides_no_output() {
        let mut generator = apu_generator();
        let mut test_bed = SimulationTestBed::new();
        update_above_threshold(&mut test_bed, &mut generator);
        update_below_threshold(&mut test_bed, &mut generator);

        assert!(generator.is_unpowered());
    }

    #[test]
    fn from_n_84_provides_voltage() {
        let mut test_bed = test_bed_with().starting_apu();

        loop {
            test_bed = test_bed.run(Duration::from_millis(50));

            let n = test_bed.n().get::<percent>();
            if n > 84. {
                assert!(test_bed.potential().get::<volt>() > 0.);
            }

            if (n - 100.).abs() < f64::EPSILON {
                break;
            }
        }
    }

    #[test]
    fn from_n_84_has_frequency() {
        let mut test_bed = test_bed_with().starting_apu();

        loop {
            test_bed = test_bed.run(Duration::from_millis(50));

            let n = test_bed.n().get::<percent>();
            if n > 84. {
                assert!(test_bed.frequency().get::<hertz>() > 0.);
            }

            if (n - 100.).abs() < f64::EPSILON {
                break;
            }
        }
    }

    #[test]
    fn in_normal_conditions_when_n_100_voltage_114_or_115() {
        let mut test_bed = test_bed_with().running_apu();

        for _ in 0..100 {
            test_bed = test_bed.run(Duration::from_millis(50));

            let voltage = test_bed.potential().get::<volt>();
            assert!((114.0..=115.0).contains(&voltage))
        }
    }

    #[test]
    fn in_normal_conditions_when_n_100_frequency_400() {
        let mut test_bed = test_bed_with().running_apu();

        for _ in 0..100 {
            test_bed = test_bed.run(Duration::from_millis(50));

            let frequency = test_bed.frequency().get::<hertz>();
            assert_about_eq!(frequency, 400.);
        }
    }

    #[test]
    fn when_shutdown_frequency_not_normal() {
        let mut test_bed = test_bed().run(Duration::from_secs(1_000));

        assert!(!test_bed.frequency_within_normal_range());
    }

    #[test]
    fn when_running_frequency_normal() {
        let mut test_bed = test_bed_with()
            .running_apu()
            .run(Duration::from_secs(1_000));

        assert!(test_bed.frequency_within_normal_range());
    }

    #[test]
    fn when_shutdown_potential_not_normal() {
        let mut test_bed = test_bed().run(Duration::from_secs(1_000));

        assert!(!test_bed.potential_within_normal_range());
    }

    #[test]
    fn when_running_potential_normal() {
        let mut test_bed = test_bed_with()
            .running_apu()
            .run(Duration::from_secs(1_000));

        assert!(test_bed.potential_within_normal_range());
    }

    #[test]
    fn when_shutdown_has_no_load() {
        let mut test_bed = test_bed().run(Duration::from_secs(1_000));

        assert_eq!(test_bed.load(), Ratio::new::<percent>(0.));
    }

    #[test]
    fn when_running_but_potential_unused_has_no_load() {
        let mut test_bed = test_bed_with()
            .running_apu()
            .power_demand(Power::new::<watt>(0.))
            .run(Duration::from_secs(1_000));

        assert_eq!(test_bed.load(), Ratio::new::<percent>(0.));
    }

    #[test]
    fn when_running_and_potential_used_has_load() {
        let mut test_bed = test_bed_with()
            .running_apu()
            .power_demand(Power::new::<watt>(50000.))
            .run(Duration::from_secs(1_000));

        assert!(test_bed.load() > Ratio::new::<percent>(0.));
    }

    #[test]
    fn when_load_below_maximum_it_is_normal() {
        let mut test_bed = test_bed_with()
            .running_apu()
            .power_demand(Power::new::<watt>(90000. / 0.8))
            .run(Duration::from_secs(1_000));

        assert!(test_bed.load_within_normal_range());
    }

    #[test]
    fn when_load_exceeds_maximum_not_normal() {
        let mut test_bed = test_bed_with()
            .running_apu()
            .power_demand(Power::new::<watt>((90000. / 0.8) + 1.))
            .run(Duration::from_secs(1_000));

        assert!(!test_bed.load_within_normal_range());
    }

    #[test]
    fn when_apu_emergency_shutdown_provides_no_output() {
        let test_bed = test_bed_with()
            .running_apu()
            .and()
            .released_apu_fire_pb()
            .run(Duration::from_secs(1));

        assert!(test_bed.generator_output_potential().is_unpowered());
    }

    #[test]
    fn writes_its_state() {
        let mut apu_gen = apu_generator();
        let mut test_bed = SimulationTestBed::new();
        test_bed.run_without_update(&mut apu_gen);

        assert!(test_bed.contains_key("ELEC_APU_GEN_1_POTENTIAL"));
        assert!(test_bed.contains_key("ELEC_APU_GEN_1_POTENTIAL_NORMAL"));
        assert!(test_bed.contains_key("ELEC_APU_GEN_1_FREQUENCY"));
        assert!(test_bed.contains_key("ELEC_APU_GEN_1_FREQUENCY_NORMAL"));
        assert!(test_bed.contains_key("ELEC_APU_GEN_1_LOAD"));
        assert!(test_bed.contains_key("ELEC_APU_GEN_1_LOAD_NORMAL"));
    }

    fn apu_generator() -> Aps3200ApuGenerator {
        Aps3200ApuGenerator::new(1)
    }

    fn update_above_threshold(
        test_bed: &mut SimulationTestBed,
        generator: &mut Aps3200ApuGenerator,
    ) {
        test_bed.run_before_power_distribution(generator, |gen, _| {
            gen.update(Ratio::new::<percent>(100.), false);
        });
    }

    fn update_below_threshold(
        test_bed: &mut SimulationTestBed,
        generator: &mut Aps3200ApuGenerator,
    ) {
        test_bed.run_before_power_distribution(generator, |gen, _| {
            gen.update(Ratio::new::<percent>(0.), false);
        });
    }
}
