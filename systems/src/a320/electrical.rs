use super::A320Hydraulic;
use crate::{
    apu::{ApuGenerator, AuxiliaryPowerUnit},
    electrical::{
        Battery, Contactor, ElectricalBus, EmergencyGenerator, EngineGenerator,
        ExternalPowerSource, PowerConductor, Powerable, StaticInverter, TransformerRectifier,
    },
    overhead::{AutoOffPushButton, NormalAltnPushButton, OnOffPushButton},
    shared::{DelayedTrueLogicGate, Engine},
    simulator::UpdateContext,
};
use std::time::Duration;
use uom::si::{f64::*, velocity::knot};

pub struct A320Electrical {
    engine_1_gen: EngineGenerator,
    engine_1_gen_contactor: Contactor,
    engine_2_gen: EngineGenerator,
    engine_2_gen_contactor: Contactor,
    bus_tie_1_contactor: Contactor,
    bus_tie_2_contactor: Contactor,
    apu_gen: ApuGenerator,
    apu_gen_contactor: Contactor,
    ext_pwr_contactor: Contactor,
    ac_bus_1: ElectricalBus,
    ac_bus_2: ElectricalBus,
    ac_ess_bus: ElectricalBus,
    ac_ess_feed_contactor_1: Contactor,
    ac_ess_feed_contactor_2: Contactor,
    ac_ess_feed_contactor_delay_logic_gate: DelayedTrueLogicGate,
    ac_ess_shed_bus: ElectricalBus,
    ac_ess_shed_contactor: Contactor,
    tr_1: TransformerRectifier,
    tr_1_contactor: Contactor,
    tr_2: TransformerRectifier,
    tr_2_contactor: Contactor,
    tr_ess: TransformerRectifier,
    tr_ess_contactor: Contactor,
    ac_ess_to_tr_ess_contactor: Contactor,
    emergency_gen: EmergencyGenerator,
    emergency_gen_contactor: Contactor,
    dc_bus_1: ElectricalBus,
    dc_bus_2: ElectricalBus,
    dc_bus_1_tie_contactor: Contactor,
    dc_bus_2_tie_contactor: Contactor,
    dc_bat_bus: ElectricalBus,
    dc_ess_bus: ElectricalBus,
    dc_bat_bus_to_dc_ess_bus_contactor: Contactor,
    dc_ess_shed_bus: ElectricalBus,
    dc_ess_shed_contactor: Contactor,
    battery_1: Battery,
    battery_1_contactor: Contactor,
    battery_2: Battery,
    battery_2_contactor: Contactor,
    battery_2_to_dc_ess_bus_contactor: Contactor,
    battery_1_to_static_inv_contactor: Contactor,
    static_inv: StaticInverter,
    static_inv_to_ac_ess_bus_contactor: Contactor,
    ac_stat_inv_bus: ElectricalBus,
    hot_bus_1: ElectricalBus,
    hot_bus_2: ElectricalBus,
}

impl A320Electrical {
    const AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS: Duration = Duration::from_secs(3);

    pub fn new() -> A320Electrical {
        A320Electrical {
            engine_1_gen: EngineGenerator::new(1),
            engine_1_gen_contactor: Contactor::new(String::from("9XU1")),
            engine_2_gen: EngineGenerator::new(2),
            engine_2_gen_contactor: Contactor::new(String::from("9XU2")),
            bus_tie_1_contactor: Contactor::new(String::from("11XU1")),
            bus_tie_2_contactor: Contactor::new(String::from("11XU2")),
            apu_gen: ApuGenerator::new(),
            apu_gen_contactor: Contactor::new(String::from("3XS")),
            ext_pwr_contactor: Contactor::new(String::from("3XG")),
            ac_bus_1: ElectricalBus::new(),
            ac_bus_2: ElectricalBus::new(),
            ac_ess_bus: ElectricalBus::new(),
            ac_ess_feed_contactor_1: Contactor::new(String::from("3XC1")),
            ac_ess_feed_contactor_2: Contactor::new(String::from("3XC2")),
            ac_ess_feed_contactor_delay_logic_gate: DelayedTrueLogicGate::new(
                A320Electrical::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS,
            ),
            ac_ess_shed_bus: ElectricalBus::new(),
            ac_ess_shed_contactor: Contactor::new(String::from("8XH")),
            tr_1: TransformerRectifier::new(),
            tr_1_contactor: Contactor::new(String::from("5PU1")),
            tr_2: TransformerRectifier::new(),
            tr_2_contactor: Contactor::new(String::from("5PU2")),
            tr_ess: TransformerRectifier::new(),
            tr_ess_contactor: Contactor::new(String::from("3PE")),
            ac_ess_to_tr_ess_contactor: Contactor::new(String::from("15XE1")),
            emergency_gen: EmergencyGenerator::new(),
            emergency_gen_contactor: Contactor::new(String::from("2XE")),
            dc_bus_1: ElectricalBus::new(),
            dc_bus_1_tie_contactor: Contactor::new(String::from("1PC1")),
            dc_bus_2: ElectricalBus::new(),
            dc_bus_2_tie_contactor: Contactor::new(String::from("1PC2")),
            dc_bat_bus: ElectricalBus::new(),
            dc_ess_bus: ElectricalBus::new(),
            dc_bat_bus_to_dc_ess_bus_contactor: Contactor::new(String::from("4PC")),
            dc_ess_shed_bus: ElectricalBus::new(),
            dc_ess_shed_contactor: Contactor::new(String::from("8PH")),
            battery_1: Battery::full(1),
            battery_1_contactor: Contactor::new(String::from("6PB1")),
            battery_2: Battery::full(2),
            battery_2_contactor: Contactor::new(String::from("6PB2")),
            battery_2_to_dc_ess_bus_contactor: Contactor::new(String::from("2XB-2")),
            battery_1_to_static_inv_contactor: Contactor::new(String::from("2XB-1")),
            static_inv: StaticInverter::new(),
            static_inv_to_ac_ess_bus_contactor: Contactor::new(String::from("15XE2")),
            ac_stat_inv_bus: ElectricalBus::new(),
            hot_bus_1: ElectricalBus::new(),
            hot_bus_2: ElectricalBus::new(),
        }
    }

    pub fn update(
        &mut self,
        context: &UpdateContext,
        engine1: &Engine,
        engine2: &Engine,
        apu: &AuxiliaryPowerUnit,
        ext_pwr: &ExternalPowerSource,
        hydraulic: &A320Hydraulic,
        overhead: &A320ElectricalOverheadPanel,
    ) {
        self.engine_1_gen.update(context, engine1, &overhead.idg_1);
        self.engine_2_gen.update(context, engine2, &overhead.idg_2);
        self.apu_gen.update(apu);
        self.emergency_gen.update(
            // ON GROUND BAT ONLY SPEED <= 100 kts scenario. We'll probably need to move this logic into
            // the ram air turbine, emergency generator and hydraulic implementation.
            hydraulic.is_blue_pressurised()
                && context.indicated_airspeed > Velocity::new::<knot>(100.),
        );

        let gen_1_provides_power = overhead.generator_1_is_on() && self.engine_1_gen.is_powered();
        let gen_2_provides_power = overhead.generator_2_is_on() && self.engine_2_gen.is_powered();
        let no_engine_gen_provides_power = !gen_1_provides_power && !gen_2_provides_power;
        let only_one_engine_gen_is_powered = gen_1_provides_power ^ gen_2_provides_power;
        let both_engine_gens_provide_power =
            !(no_engine_gen_provides_power || only_one_engine_gen_is_powered);
        let ext_pwr_provides_power = overhead.external_power_is_on()
            && ext_pwr.is_powered()
            && !both_engine_gens_provide_power;
        let apu_gen_provides_power = overhead.apu_generator_is_on()
            && self.apu_gen.is_powered()
            && !ext_pwr_provides_power
            && !both_engine_gens_provide_power;

        self.engine_1_gen_contactor.close_when(gen_1_provides_power);
        self.engine_2_gen_contactor.close_when(gen_2_provides_power);
        self.apu_gen_contactor.close_when(apu_gen_provides_power);
        self.ext_pwr_contactor.close_when(ext_pwr_provides_power);

        let apu_or_ext_pwr_provides_power = ext_pwr_provides_power || apu_gen_provides_power;
        self.bus_tie_1_contactor.close_when(
            overhead.bus_tie_is_auto()
                && ((only_one_engine_gen_is_powered && !apu_or_ext_pwr_provides_power)
                    || (apu_or_ext_pwr_provides_power && !gen_1_provides_power)),
        );
        self.bus_tie_2_contactor.close_when(
            overhead.bus_tie_is_auto()
                && ((only_one_engine_gen_is_powered && !apu_or_ext_pwr_provides_power)
                    || (apu_or_ext_pwr_provides_power && !gen_2_provides_power)),
        );

        self.apu_gen_contactor.powered_by(vec![&self.apu_gen]);
        self.ext_pwr_contactor.powered_by(vec![ext_pwr]);

        self.engine_1_gen_contactor
            .powered_by(vec![&self.engine_1_gen]);
        self.bus_tie_1_contactor.powered_by(vec![
            &self.engine_1_gen_contactor,
            &self.apu_gen_contactor,
            &self.ext_pwr_contactor,
        ]);

        self.engine_2_gen_contactor
            .powered_by(vec![&self.engine_2_gen]);
        self.bus_tie_2_contactor.powered_by(vec![
            &self.engine_2_gen_contactor,
            &self.apu_gen_contactor,
            &self.ext_pwr_contactor,
        ]);

        self.bus_tie_1_contactor
            .or_powered_by(vec![&self.bus_tie_2_contactor]);
        self.bus_tie_2_contactor
            .or_powered_by(vec![&self.bus_tie_1_contactor]);

        self.ac_bus_1.powered_by(vec![
            &self.engine_1_gen_contactor,
            &self.bus_tie_1_contactor,
        ]);
        self.ac_bus_2.powered_by(vec![
            &self.engine_2_gen_contactor,
            &self.bus_tie_2_contactor,
        ]);

        self.tr_1.powered_by(vec![&self.ac_bus_1]);
        self.tr_2.powered_by(vec![&self.ac_bus_2]);

        self.ac_ess_feed_contactor_delay_logic_gate
            .update(context, self.ac_bus_1.is_unpowered());

        self.ac_ess_feed_contactor_1.close_when(
            self.ac_bus_1.is_powered()
                && (!self.ac_ess_feed_contactor_delay_logic_gate.output()
                    && overhead.ac_ess_feed_is_normal()),
        );
        self.ac_ess_feed_contactor_2.close_when(
            self.ac_bus_2.is_powered()
                && (self.ac_ess_feed_contactor_delay_logic_gate.output()
                    || overhead.ac_ess_feed_is_altn()),
        );

        self.ac_ess_feed_contactor_1
            .powered_by(vec![&self.ac_bus_1]);
        self.ac_ess_feed_contactor_2
            .powered_by(vec![&self.ac_bus_2]);

        self.ac_ess_bus.powered_by(vec![
            &self.ac_ess_feed_contactor_1,
            &self.ac_ess_feed_contactor_2,
        ]);

        self.emergency_gen_contactor
            .close_when(self.ac_bus_1.is_unpowered() && self.ac_bus_2.is_unpowered());
        self.emergency_gen_contactor
            .powered_by(vec![&self.emergency_gen]);

        let ac_ess_to_tr_ess_contactor_power_sources: Vec<&dyn PowerConductor> =
            vec![&self.ac_ess_bus, &self.emergency_gen_contactor];
        self.ac_ess_to_tr_ess_contactor
            .powered_by(ac_ess_to_tr_ess_contactor_power_sources);

        let tr_1_or_2_unavailable = A320Electrical::has_failed_or_is_unpowered(&self.tr_1)
            || A320Electrical::has_failed_or_is_unpowered(&self.tr_2);
        self.ac_ess_to_tr_ess_contactor.close_when(
            (tr_1_or_2_unavailable
                && (self.ac_ess_feed_contactor_1.is_powered()
                    || self.ac_ess_feed_contactor_2.is_powered()))
                || self.emergency_gen_contactor.is_powered(),
        );

        self.ac_ess_bus
            .or_powered_by(vec![&self.ac_ess_to_tr_ess_contactor]);

        self.ac_ess_shed_contactor
            .powered_by(vec![&self.ac_ess_bus]);

        self.tr_ess.powered_by(vec![
            &self.ac_ess_to_tr_ess_contactor,
            &self.emergency_gen_contactor,
        ]);

        self.tr_1_contactor.close_when(self.tr_1.is_powered());
        self.tr_1_contactor.powered_by(vec![&self.tr_1]);

        self.tr_2_contactor.close_when(self.tr_2.is_powered());
        self.tr_2_contactor.powered_by(vec![&self.tr_2]);

        self.tr_ess_contactor
            .close_when(tr_1_or_2_unavailable && self.tr_ess.is_powered());
        self.tr_ess_contactor.powered_by(vec![&self.tr_ess]);

        self.dc_bus_1.powered_by(vec![&self.tr_1_contactor]);
        self.dc_bus_2.powered_by(vec![&self.tr_2_contactor]);

        self.dc_bus_1_tie_contactor.powered_by(vec![&self.dc_bus_1]);
        self.dc_bus_2_tie_contactor.powered_by(vec![&self.dc_bus_2]);

        self.dc_bus_1_tie_contactor
            .close_when(self.dc_bus_1.is_powered() || self.dc_bus_2.is_powered());
        self.dc_bus_2_tie_contactor
            .close_when(self.dc_bus_1.is_unpowered() || self.dc_bus_2.is_unpowered());

        self.dc_bat_bus.powered_by(vec![
            &self.dc_bus_1_tie_contactor,
            &self.dc_bus_2_tie_contactor,
        ]);

        self.dc_bus_1_tie_contactor
            .or_powered_by(vec![&self.dc_bat_bus]);
        self.dc_bus_2_tie_contactor
            .or_powered_by(vec![&self.dc_bat_bus]);
        self.dc_bus_1
            .or_powered_by(vec![&self.dc_bus_1_tie_contactor]);
        self.dc_bus_2
            .or_powered_by(vec![&self.dc_bus_2_tie_contactor]);

        self.battery_1_contactor.powered_by(vec![&self.dc_bat_bus]);
        self.battery_2_contactor.powered_by(vec![&self.dc_bat_bus]);

        // TODO: The actual logic for battery contactors is more complex, however
        // not all systems is relates to are implemented yet. We'll have to get back to this later.
        let ac_bus_1_and_2_unpowered = self.ac_bus_1.is_unpowered() && self.ac_bus_2.is_unpowered();
        let airspeed_below_100_knots = context.indicated_airspeed < Velocity::new::<knot>(100.);
        let batteries_should_supply_bat_bus = ac_bus_1_and_2_unpowered && airspeed_below_100_knots;
        self.battery_1_contactor.close_when(
            overhead.bat_1_is_auto()
                && (!self.battery_1.is_full() || batteries_should_supply_bat_bus),
        );
        self.battery_2_contactor.close_when(
            overhead.bat_2_is_auto()
                && (!self.battery_2.is_full() || batteries_should_supply_bat_bus),
        );

        self.battery_1.powered_by(vec![&self.battery_1_contactor]);
        self.battery_2.powered_by(vec![&self.battery_2_contactor]);

        self.battery_1_contactor
            .or_powered_by(vec![&self.battery_1]);
        self.battery_2_contactor
            .or_powered_by(vec![&self.battery_2]);

        self.dc_bat_bus
            .or_powered_by_both_batteries(&self.battery_1_contactor, &self.battery_2_contactor);

        self.hot_bus_1.powered_by(vec![&self.battery_1]);
        self.hot_bus_2.powered_by(vec![&self.battery_2]);

        self.dc_bat_bus_to_dc_ess_bus_contactor
            .powered_by(vec![&self.dc_bat_bus]);
        self.dc_bat_bus_to_dc_ess_bus_contactor
            .close_when(!tr_1_or_2_unavailable);
        self.battery_2_to_dc_ess_bus_contactor
            .powered_by(vec![&self.battery_2]);
        A320Electrical::close_multiple_contactors_when(
            vec![
                &mut self.battery_2_to_dc_ess_bus_contactor,
                &mut self.battery_1_to_static_inv_contactor,
            ],
            self.tr_ess_contactor.is_open() && self.dc_bat_bus_to_dc_ess_bus_contactor.is_open(),
        );

        self.battery_1_to_static_inv_contactor
            .powered_by(vec![&self.battery_1]);

        self.static_inv
            .powered_by(vec![&self.battery_1_to_static_inv_contactor]);

        self.ac_stat_inv_bus.powered_by(vec![&self.static_inv]);
        self.static_inv_to_ac_ess_bus_contactor.close_when(
            self.static_inv.is_powered()
                && context.indicated_airspeed >= Velocity::new::<knot>(50.),
        );
        self.static_inv_to_ac_ess_bus_contactor
            .powered_by(vec![&self.static_inv]);

        self.ac_ess_bus
            .or_powered_by(vec![&self.static_inv_to_ac_ess_bus_contactor]);

        let emergency_gen_provides_power = self.emergency_gen.is_powered();
        let generator_provides_power = gen_1_provides_power
            || gen_2_provides_power
            || apu_gen_provides_power
            || ext_pwr_provides_power
            || emergency_gen_provides_power;
        self.ac_ess_shed_contactor
            .close_when(generator_provides_power);

        self.ac_ess_shed_bus
            .powered_by(vec![&self.ac_ess_shed_contactor]);

        self.dc_ess_bus.powered_by(vec![
            &self.dc_bat_bus_to_dc_ess_bus_contactor,
            &self.tr_ess_contactor,
            &self.battery_2_to_dc_ess_bus_contactor,
        ]);
        self.dc_ess_shed_contactor
            .powered_by(vec![&self.dc_ess_bus]);
        self.dc_ess_shed_contactor
            .close_when(self.battery_2_to_dc_ess_bus_contactor.is_open());
        self.dc_ess_shed_bus
            .powered_by(vec![&self.dc_ess_shed_contactor]);

        self.debug_assert_invariants();
    }

    fn close_multiple_contactors_when(contactors: Vec<&mut Contactor>, should_be_closed: bool) {
        for contactor in contactors {
            contactor.close_when(should_be_closed);
        }
    }

    fn has_failed_or_is_unpowered(tr: &TransformerRectifier) -> bool {
        tr.has_failed() || tr.is_unpowered()
    }

    fn debug_assert_invariants(&self) {
        debug_assert!(self.battery_never_powers_dc_ess_shed());
        debug_assert!(self.only_one_source_powers_dc_ess_bus());
        debug_assert!(self.static_inverter_or_emergency_gen_powers_ac_ess_bus());
        debug_assert!(
            self.batteries_power_both_static_inv_and_dc_ess_bus_at_the_same_time_or_not_at_all()
        );
    }

    fn battery_never_powers_dc_ess_shed(&self) -> bool {
        !(self.battery_2_to_dc_ess_bus_contactor.is_closed()
            && self.dc_ess_shed_contactor.is_closed())
    }

    fn only_one_source_powers_dc_ess_bus(&self) -> bool {
        self.battery_2_to_dc_ess_bus_contactor.is_closed()
            ^ self.dc_bat_bus_to_dc_ess_bus_contactor.is_closed()
            ^ self.tr_ess_contactor.is_closed()
    }

    fn static_inverter_or_emergency_gen_powers_ac_ess_bus(&self) -> bool {
        !(self.static_inv_to_ac_ess_bus_contactor.is_closed()
            && self.ac_ess_to_tr_ess_contactor.is_closed())
    }

    fn batteries_power_both_static_inv_and_dc_ess_bus_at_the_same_time_or_not_at_all(
        &self,
    ) -> bool {
        self.battery_1_to_static_inv_contactor.is_closed()
            == self.battery_2_to_dc_ess_bus_contactor.is_closed()
    }
}

pub struct A320ElectricalOverheadPanel {
    bat_1: AutoOffPushButton,
    bat_2: AutoOffPushButton,
    idg_1: OnOffPushButton,
    idg_2: OnOffPushButton,
    gen_1: OnOffPushButton,
    gen_2: OnOffPushButton,
    apu_gen: OnOffPushButton,
    bus_tie: AutoOffPushButton,
    ac_ess_feed: NormalAltnPushButton,
    galy_and_cab: AutoOffPushButton,
    ext_pwr: OnOffPushButton,
    commercial: OnOffPushButton,
}

impl A320ElectricalOverheadPanel {
    pub fn new() -> A320ElectricalOverheadPanel {
        A320ElectricalOverheadPanel {
            bat_1: AutoOffPushButton::new_auto(),
            bat_2: AutoOffPushButton::new_auto(),
            idg_1: OnOffPushButton::new_on(),
            idg_2: OnOffPushButton::new_on(),
            gen_1: OnOffPushButton::new_on(),
            gen_2: OnOffPushButton::new_on(),
            apu_gen: OnOffPushButton::new_on(),
            bus_tie: AutoOffPushButton::new_auto(),
            ac_ess_feed: NormalAltnPushButton::new_normal(),
            galy_and_cab: AutoOffPushButton::new_auto(),
            ext_pwr: OnOffPushButton::new_on(),
            commercial: OnOffPushButton::new_on(),
        }
    }

    pub fn update(&mut self, _: &UpdateContext) {}

    fn generator_1_is_on(&self) -> bool {
        self.gen_1.is_on()
    }

    fn generator_2_is_on(&self) -> bool {
        self.gen_2.is_on()
    }

    fn external_power_is_on(&self) -> bool {
        self.ext_pwr.is_on()
    }

    fn apu_generator_is_on(&self) -> bool {
        self.apu_gen.is_on()
    }

    fn bus_tie_is_auto(&self) -> bool {
        self.bus_tie.is_auto()
    }

    fn ac_ess_feed_is_normal(&self) -> bool {
        self.ac_ess_feed.is_normal()
    }

    fn ac_ess_feed_is_altn(&self) -> bool {
        self.ac_ess_feed.is_altn()
    }

    fn bat_1_is_auto(&self) -> bool {
        self.bat_1.is_auto()
    }

    fn bat_2_is_auto(&self) -> bool {
        self.bat_2.is_auto()
    }
}

#[cfg(test)]
mod a320_electrical_circuit_tests {
    use crate::{
        apu::tests::{running_apu, stopped_apu},
        electrical::{Current, IntegratedDriveGenerator, PowerSource},
    };

    use uom::si::{
        length::foot, ratio::percent, thermodynamic_temperature::degree_celsius, velocity::knot,
    };

    use super::*;

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_norm_conf() {
        let tester = tester_with().running_engines().run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.ac_stat_inv_bus_output().source(), PowerSource::None);
        assert_eq!(
            tester.tr_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.tr_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(tester.tr_ess_output().source(), PowerSource::None);
        assert_eq!(
            tester.dc_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.dc_bat_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_only_gen_1_available() {
        let tester = tester_with().running_engine_1().run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.ac_stat_inv_bus_output().source(), PowerSource::None);
        assert_eq!(
            tester.tr_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.tr_2_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.tr_ess_output().source(), PowerSource::None);
        assert_eq!(
            tester.dc_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_bus_2_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_bat_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_only_gen_2_available() {
        let tester = tester_with().running_engine_2().run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(tester.ac_stat_inv_bus_output().source(), PowerSource::None);
        assert_eq!(
            tester.tr_1_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.tr_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(tester.tr_ess_output().source(), PowerSource::None);
        assert_eq!(
            tester.dc_bus_1_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.dc_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.dc_bat_bus_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.dc_ess_bus_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_only_apu_gen_available() {
        let tester = tester_with().running_apu().run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::ApuGenerator);
        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::ApuGenerator);
        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::ApuGenerator
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output().source(),
            PowerSource::ApuGenerator
        );
        assert_eq!(tester.ac_stat_inv_bus_output().source(), PowerSource::None);
        assert_eq!(tester.tr_1_output().source(), PowerSource::ApuGenerator);
        assert_eq!(tester.tr_2_output().source(), PowerSource::ApuGenerator);
        assert_eq!(tester.tr_ess_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_1_output().source(), PowerSource::ApuGenerator);
        assert_eq!(tester.dc_bus_2_output().source(), PowerSource::ApuGenerator);
        assert_eq!(
            tester.dc_bat_bus_output().source(),
            PowerSource::ApuGenerator
        );
        assert_eq!(
            tester.dc_ess_bus_output().source(),
            PowerSource::ApuGenerator
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output().source(),
            PowerSource::ApuGenerator
        );
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// Derived from A320 manual electrical distribution table
    /// (doesn't list external power, but we'll assume it's the same as other generators).
    #[test]
    fn distribution_table_only_external_power_available() {
        let tester = tester_with().connected_external_power().run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::External);
        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::External);
        assert_eq!(tester.ac_ess_bus_output().source(), PowerSource::External);
        assert_eq!(
            tester.ac_ess_shed_bus_output().source(),
            PowerSource::External
        );
        assert_eq!(tester.ac_stat_inv_bus_output().source(), PowerSource::None);
        assert_eq!(tester.tr_1_output().source(), PowerSource::External);
        assert_eq!(tester.tr_2_output().source(), PowerSource::External);
        assert_eq!(tester.tr_ess_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_1_output().source(), PowerSource::External);
        assert_eq!(tester.dc_bus_2_output().source(), PowerSource::External);
        assert_eq!(tester.dc_bat_bus_output().source(), PowerSource::External);
        assert_eq!(tester.dc_ess_bus_output().source(), PowerSource::External);
        assert_eq!(
            tester.dc_ess_shed_bus_output().source(),
            PowerSource::External
        );
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_emergency_config_before_emergency_gen_available() {
        let tester = tester().run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::None);
        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::None);
        assert_eq!(tester.ac_ess_bus_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.ac_ess_shed_bus_output().source(), PowerSource::None);
        assert_eq!(
            tester.ac_stat_inv_bus_output().source(),
            PowerSource::Battery(1)
        );
        assert_eq!(tester.tr_1_output().source(), PowerSource::None);
        assert_eq!(tester.tr_2_output().source(), PowerSource::None);
        assert_eq!(tester.tr_ess_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_1_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_2_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bat_bus_output().source(), PowerSource::None);
        assert_eq!(tester.dc_ess_bus_output().source(), PowerSource::Battery(2));
        assert_eq!(tester.dc_ess_shed_bus_output().source(), PowerSource::None);
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_emergency_config_after_emergency_gen_available() {
        let tester = tester_with().running_emergency_generator().run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::None);
        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::None);
        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EmergencyGenerator
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output().source(),
            PowerSource::EmergencyGenerator
        );
        assert_eq!(tester.tr_1_output().source(), PowerSource::None);
        assert_eq!(tester.tr_2_output().source(), PowerSource::None);
        assert_eq!(
            tester.tr_ess_output().source(),
            PowerSource::EmergencyGenerator
        );
        assert_eq!(tester.ac_stat_inv_bus_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_1_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_2_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bat_bus_output().source(), PowerSource::None);
        assert_eq!(
            tester.dc_ess_bus_output().source(),
            PowerSource::EmergencyGenerator
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output().source(),
            PowerSource::EmergencyGenerator
        );
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_tr_1_fault() {
        let tester = tester_with().running_engines().and().failed_tr_1().run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.ac_stat_inv_bus_output().source(), PowerSource::None);
        assert_eq!(tester.tr_1_output().source(), PowerSource::None);
        assert_eq!(
            tester.tr_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.tr_ess_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_bus_1_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.dc_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.dc_bat_bus_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.dc_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_tr_2_fault() {
        let tester = tester_with().running_engines().and().failed_tr_2().run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.ac_stat_inv_bus_output().source(), PowerSource::None);
        assert_eq!(
            tester.tr_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.tr_2_output().source(), PowerSource::None);
        assert_eq!(
            tester.tr_ess_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_bus_2_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_bat_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_tr_1_and_2_fault() {
        let tester = tester_with()
            .running_engines()
            .failed_tr_1()
            .and()
            .failed_tr_2()
            .run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.ac_stat_inv_bus_output().source(), PowerSource::None);
        assert_eq!(tester.tr_1_output().source(), PowerSource::None);
        assert_eq!(tester.tr_2_output().source(), PowerSource::None);
        assert_eq!(
            tester.tr_ess_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.dc_bus_1_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_2_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bat_bus_output().source(), PowerSource::None);
        assert_eq!(
            tester.dc_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_on_ground_bat_and_emergency_gen_only_speed_above_100_knots() {
        let tester = tester_with()
            .running_emergency_generator()
            .airspeed(Velocity::new::<knot>(101.))
            .and()
            .on_the_ground()
            .run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::None);
        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::None);
        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EmergencyGenerator
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output().source(),
            PowerSource::EmergencyGenerator
        );
        assert_eq!(tester.ac_stat_inv_bus_output().source(), PowerSource::None);
        assert_eq!(tester.tr_1_output().source(), PowerSource::None);
        assert_eq!(tester.tr_2_output().source(), PowerSource::None);
        assert_eq!(
            tester.tr_ess_output().source(),
            PowerSource::EmergencyGenerator
        );
        assert_eq!(tester.dc_bus_1_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_2_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bat_bus_output().source(), PowerSource::None);
        assert_eq!(
            tester.dc_ess_bus_output().source(),
            PowerSource::EmergencyGenerator
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output().source(),
            PowerSource::EmergencyGenerator
        );
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_on_ground_bat_only_rat_stall_or_speed_between_50_to_100_knots() {
        let tester = tester_with()
            .running_emergency_generator()
            .airspeed(Velocity::new::<knot>(50.))
            .and()
            .on_the_ground()
            .run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::None);
        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::None);
        assert_eq!(tester.ac_ess_bus_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.ac_ess_shed_bus_output().source(), PowerSource::None);
        assert_eq!(
            tester.ac_stat_inv_bus_output().source(),
            PowerSource::Battery(1)
        );
        assert_eq!(tester.tr_1_output().source(), PowerSource::None);
        assert_eq!(tester.tr_2_output().source(), PowerSource::None);
        assert_eq!(tester.tr_ess_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_1_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_2_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bat_bus_output().source(), PowerSource::Batteries);
        assert_eq!(tester.dc_ess_bus_output().source(), PowerSource::Battery(2));
        assert_eq!(tester.dc_ess_shed_bus_output().source(), PowerSource::None);
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_on_ground_bat_only_speed_less_than_50_knots() {
        let tester = tester_with()
            .running_emergency_generator()
            .airspeed(Velocity::new::<knot>(49.9))
            .and()
            .on_the_ground()
            .run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::None);
        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::None);
        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::None,
            "AC ESS BUS shouldn't be powered below 50 knots when on batteries only."
        );
        assert_eq!(tester.ac_ess_shed_bus_output().source(), PowerSource::None);
        assert_eq!(
            tester.ac_stat_inv_bus_output().source(),
            PowerSource::Battery(1)
        );
        assert_eq!(tester.tr_1_output().source(), PowerSource::None);
        assert_eq!(tester.tr_2_output().source(), PowerSource::None);
        assert_eq!(tester.tr_ess_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_1_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bus_2_output().source(), PowerSource::None);
        assert_eq!(tester.dc_bat_bus_output().source(), PowerSource::Batteries);
        assert_eq!(tester.dc_ess_bus_output().source(), PowerSource::Battery(2));
        assert_eq!(tester.dc_ess_shed_bus_output().source(), PowerSource::None);
        assert_eq!(tester.hot_bus_1_output().source(), PowerSource::Battery(1));
        assert_eq!(tester.hot_bus_2_output().source(), PowerSource::Battery(2));
    }

    #[test]
    fn when_available_engine_1_gen_supplies_ac_bus_1() {
        let tester = tester_with().running_engine_1().run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
    }

    #[test]
    fn when_available_engine_2_gen_supplies_ac_bus_2() {
        let tester = tester_with().running_engine_2().run();

        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
    }

    #[test]
    fn when_only_engine_1_is_running_supplies_ac_bus_2() {
        let tester = tester_with().running_engine_1().run();

        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(1)
        );
    }

    #[test]
    fn when_only_engine_2_is_running_supplies_ac_bus_1() {
        let tester = tester_with().running_engine_2().run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(2)
        );
    }

    #[test]
    fn when_no_power_source_ac_bus_1_is_unpowered() {
        let tester = tester().run();

        assert!(tester.ac_bus_1_output().is_unpowered());
    }

    #[test]
    fn when_no_power_source_ac_bus_2_is_unpowered() {
        let tester = tester().run();

        assert!(tester.ac_bus_2_output().is_unpowered());
    }

    #[test]
    fn when_engine_1_and_apu_running_apu_powers_ac_bus_2() {
        let tester = tester_with().running_engine_1().and().running_apu().run();

        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::ApuGenerator);
    }

    #[test]
    fn when_engine_2_and_apu_running_apu_powers_ac_bus_1() {
        let tester = tester_with().running_engine_2().and().running_apu().run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::ApuGenerator);
    }

    #[test]
    fn when_only_apu_running_apu_powers_ac_bus_1_and_2() {
        let tester = tester_with().running_apu().run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::ApuGenerator);
        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::ApuGenerator);
    }

    #[test]
    fn when_engine_1_running_and_external_power_connected_ext_pwr_powers_ac_bus_2() {
        let tester = tester_with()
            .running_engine_1()
            .and()
            .connected_external_power()
            .run();

        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::External);
    }

    #[test]
    fn when_engine_2_running_and_external_power_connected_ext_pwr_powers_ac_bus_1() {
        let tester = tester_with()
            .running_engine_2()
            .and()
            .connected_external_power()
            .run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::External);
    }

    #[test]
    fn when_only_external_power_connected_ext_pwr_powers_ac_bus_1_and_2() {
        let tester = tester_with().connected_external_power().run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::External);
        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::External);
    }

    #[test]
    fn when_external_power_connected_and_apu_running_external_power_has_priority() {
        let tester = tester_with()
            .connected_external_power()
            .and()
            .running_apu()
            .run();

        assert_eq!(tester.ac_bus_1_output().source(), PowerSource::External);
        assert_eq!(tester.ac_bus_2_output().source(), PowerSource::External);
    }

    #[test]
    fn when_both_engines_running_and_external_power_connected_engines_power_ac_buses() {
        let tester = tester_with()
            .running_engines()
            .and()
            .connected_external_power()
            .run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
    }

    #[test]
    fn when_both_engines_running_and_apu_running_engines_power_ac_buses() {
        let tester = tester_with().running_engines().and().running_apu().run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
    }

    #[test]
    fn ac_bus_1_powers_ac_ess_bus_whenever_it_is_powered() {
        let tester = tester_with().running_engines().run();

        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
    }

    #[test]
    fn when_ac_bus_1_becomes_unpowered_bat_1_powers_ac_ess_bus_for_a_while() {
        let tester = tester_with()
            .running_engines()
            .and()
            .failed_ac_bus_1()
            .run_waiting_until_just_before_ac_ess_feed_transition();

        assert_eq!(tester.ac_ess_bus_output().source(), PowerSource::Battery(1));
    }

    #[test]
    fn when_ac_bus_1_becomes_unpowered_bat_2_powers_dc_ess_bus_for_a_while() {
        let tester = tester_with()
            .running_engines()
            .and()
            .failed_ac_bus_1()
            .run_waiting_until_just_before_ac_ess_feed_transition();

        assert_eq!(tester.dc_ess_bus_output().source(), PowerSource::Battery(2));
    }

    /// # Source
    /// Discord (komp#1821):
    /// > The fault light will extinguish after 3 seconds. That's the time delay before automatic switching is activated in case of AC BUS 1 loss.
    #[test]
    fn with_ac_bus_1_being_unpowered_after_a_delay_ac_bus_2_powers_ac_ess_bus() {
        let tester = tester_with()
            .running_engines()
            .and()
            .failed_ac_bus_1()
            .run_waiting_for_ac_ess_feed_transition();

        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EngineGenerator(2)
        );
    }

    /// # Source
    /// Discord (komp#1821):
    /// > When AC BUS 1 is available again, it will switch back automatically without delay, unless the AC ESS FEED button is on ALTN.
    #[test]
    fn ac_bus_1_powers_ac_ess_bus_immediately_when_ac_bus_1_becomes_powered_after_ac_bus_2_was_powering_ac_ess_bus(
    ) {
        let tester = tester_with()
            .running_engines()
            .and()
            .failed_ac_bus_1()
            .run_waiting_for_ac_ess_feed_transition()
            .then_continue_with()
            .normal_ac_bus_1()
            .run();

        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EngineGenerator(1)
        );
    }

    #[test]
    fn battery_1_powers_ac_ess_bus_when_ac_bus_1_and_2_failed() {
        let tester = tester_with()
            .running_engines()
            .failed_ac_bus_1()
            .and()
            .failed_ac_bus_2()
            .run();

        assert_eq!(tester.ac_ess_bus_output().source(), PowerSource::Battery(1));
    }

    #[test]
    fn when_gen_1_off_and_only_engine_1_running_nothing_powers_ac_buses() {
        let tester = tester_with().running_engine_1().and().gen_1_off().run();

        assert!(tester.ac_bus_1_output().is_unpowered());
        assert!(tester.ac_bus_2_output().is_unpowered());
    }

    #[test]
    fn when_gen_1_off_and_both_engines_running_engine_2_powers_ac_buses() {
        let tester = tester_with().running_engines().and().gen_1_off().run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(2)
        );
        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(2)
        );
    }

    #[test]
    fn when_gen_2_off_and_only_engine_2_running_nothing_powers_ac_buses() {
        let tester = tester_with().running_engine_2().and().gen_2_off().run();

        assert!(tester.ac_bus_1_output().is_unpowered());
        assert!(tester.ac_bus_2_output().is_unpowered());
    }

    #[test]
    fn when_gen_2_off_and_both_engines_running_engine_1_powers_ac_buses() {
        let tester = tester_with().running_engines().and().gen_2_off().run();

        assert_eq!(
            tester.ac_bus_1_output().source(),
            PowerSource::EngineGenerator(1)
        );
        assert_eq!(
            tester.ac_bus_2_output().source(),
            PowerSource::EngineGenerator(1)
        );
    }

    #[test]
    fn when_ac_ess_feed_push_button_altn_ac_bus_2_powers_ac_ess_bus() {
        let tester = tester_with()
            .running_engines()
            .and()
            .ac_ess_feed_altn()
            .run();

        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EngineGenerator(2)
        );
    }

    #[test]
    fn when_only_apu_running_but_apu_gen_push_button_off_nothing_powers_ac_bus_1_and_2() {
        let tester = tester_with().running_apu().and().apu_gen_off().run();

        assert!(tester.ac_bus_1_output().is_unpowered());
        assert!(tester.ac_bus_2_output().is_unpowered());
    }

    #[test]
    fn when_only_external_power_connected_but_ext_pwr_push_button_off_nothing_powers_ac_bus_1_and_2(
    ) {
        let tester = tester_with()
            .connected_external_power()
            .and()
            .ext_pwr_off()
            .run();

        assert!(tester.ac_bus_1_output().is_unpowered());
        assert!(tester.ac_bus_2_output().is_unpowered());
    }

    #[test]
    fn when_ac_bus_1_powered_tr_1_is_powered() {
        let tester = tester_with().running_engines().run();

        assert!(tester.tr_1_output().is_powered());
    }

    #[test]
    fn when_ac_bus_1_unpowered_tr_1_is_unpowered() {
        let tester = tester().run();

        assert!(tester.tr_1_output().is_unpowered());
    }

    #[test]
    fn when_ac_bus_2_powered_tr_2_is_powered() {
        let tester = tester_with().running_engines().run();

        assert!(tester.tr_2_output().is_powered());
    }

    #[test]
    fn when_ac_bus_2_unpowered_tr_2_is_unpowered() {
        let tester = tester().run();

        assert!(tester.tr_2_output().is_unpowered());
    }

    #[test]
    fn when_tr_1_failed_ess_tr_powered() {
        let tester = tester_with().running_engines().and().failed_tr_1().run();

        assert!(tester.tr_ess_output().is_powered());
    }

    #[test]
    fn when_tr_1_unpowered_ess_tr_powered() {
        let tester = tester_with()
            .running_engines()
            .and()
            .failed_ac_bus_1()
            // AC ESS BUS which powers TR ESS is only supplied with power after the delay.
            .run_waiting_for_ac_ess_feed_transition();

        assert!(tester.tr_ess_output().is_powered());
    }

    #[test]
    fn when_tr_2_failed_ess_tr_powered() {
        let tester = tester_with().running_engines().and().failed_tr_2().run();

        assert!(tester.tr_ess_output().is_powered());
    }

    #[test]
    fn when_tr_2_unpowered_ess_tr_powered() {
        let tester = tester_with()
            .running_engines()
            .and()
            .failed_ac_bus_2()
            .run();

        assert!(tester.tr_ess_output().is_powered());
    }

    #[test]
    fn when_tr_1_and_2_normal_ess_tr_unpowered() {
        let tester = tester_with().running_engines().run();

        assert!(tester.tr_ess_output().is_unpowered());
    }

    #[test]
    fn when_ac_bus_1_and_ac_bus_2_are_lost_a_running_emergency_gen_powers_tr_ess() {
        let tester = tester_with()
            .running_engines()
            .failed_ac_bus_1()
            .failed_ac_bus_2()
            .and()
            .running_emergency_generator()
            .run();

        assert!(tester.tr_ess_output().is_powered());
        assert_eq!(
            tester.tr_ess_output().source(),
            PowerSource::EmergencyGenerator
        );
    }

    #[test]
    fn when_ac_bus_1_and_ac_bus_2_are_lost_a_running_emergency_gen_powers_ac_ess_bus() {
        let tester = tester_with()
            .running_engines()
            .failed_ac_bus_1()
            .failed_ac_bus_2()
            .and()
            .running_emergency_generator()
            .run();

        assert!(tester.ac_ess_bus_output().is_powered());
        assert_eq!(
            tester.ac_ess_bus_output().source(),
            PowerSource::EmergencyGenerator
        );
    }

    #[test]
    fn when_ac_bus_1_and_ac_bus_2_are_lost_neither_ac_ess_feed_contactor_is_closed() {
        let tester = tester_with()
            .running_engines()
            .failed_ac_bus_1()
            .and()
            .failed_ac_bus_2()
            .run();

        assert!(tester.both_ac_ess_feed_contactors_open());
    }

    #[test]
    fn when_battery_1_full_it_is_not_powered_by_dc_bat_bus() {
        let tester = tester_with().running_engines().run();

        assert!(tester.battery_1_input().is_unpowered())
    }

    #[test]
    fn when_battery_1_not_full_it_is_powered_by_dc_bat_bus() {
        let tester = tester_with()
            .running_engines()
            .and()
            .empty_battery_1()
            .run();

        assert!(tester.battery_1_input().is_powered());
    }

    #[test]
    fn when_battery_1_not_full_and_button_off_it_is_not_powered_by_dc_bat_bus() {
        let tester = tester_with()
            .running_engines()
            .empty_battery_1()
            .and()
            .bat_1_off()
            .run();

        assert!(tester.battery_1_input().is_unpowered())
    }

    #[test]
    fn when_battery_1_has_charge_powers_hot_bus_1() {
        let tester = tester().run();

        assert!(tester.hot_bus_1_output().is_powered());
    }

    #[test]
    fn when_battery_1_is_empty_hot_bus_1_unpowered() {
        let tester = tester_with().empty_battery_1().run();

        assert!(tester.hot_bus_1_output().is_unpowered());
    }

    #[test]
    fn when_battery_2_full_it_is_not_powered_by_dc_bat_bus() {
        let tester = tester_with().running_engines().run();

        assert!(tester.battery_2_input().is_unpowered())
    }

    #[test]
    fn when_battery_2_not_full_it_is_powered_by_dc_bat_bus() {
        let tester = tester_with()
            .running_engines()
            .and()
            .empty_battery_2()
            .run();

        assert!(tester.battery_2_input().is_powered());
    }

    #[test]
    fn when_battery_2_not_full_and_button_off_it_is_not_powered_by_dc_bat_bus() {
        let tester = tester_with()
            .running_engines()
            .empty_battery_2()
            .and()
            .bat_2_off()
            .run();

        assert!(tester.battery_2_input().is_unpowered())
    }

    #[test]
    fn when_battery_2_has_charge_powers_hot_bus_2() {
        let tester = tester().run();

        assert!(tester.hot_bus_2_output().is_powered());
    }

    #[test]
    fn when_battery_2_is_empty_hot_bus_2_unpowered() {
        let tester = tester_with().empty_battery_2().run();

        assert!(tester.hot_bus_2_output().is_unpowered());
    }

    #[test]
    fn when_bus_tie_off_engine_1_does_not_power_ac_bus_2() {
        let tester = tester_with().running_engine_1().and().bus_tie_off().run();

        assert!(tester.ac_bus_2_output().is_unpowered());
    }

    #[test]
    fn when_bus_tie_off_engine_2_does_not_power_ac_bus_1() {
        let tester = tester_with().running_engine_2().and().bus_tie_off().run();

        assert!(tester.ac_bus_1_output().is_unpowered());
    }

    #[test]
    fn when_bus_tie_off_apu_does_not_power_ac_buses() {
        let tester = tester_with().running_apu().and().bus_tie_off().run();

        assert!(tester.ac_bus_1_output().is_unpowered());
        assert!(tester.ac_bus_2_output().is_unpowered());
    }

    #[test]
    fn when_bus_tie_off_external_power_does_not_power_ac_buses() {
        let tester = tester_with()
            .connected_external_power()
            .and()
            .bus_tie_off()
            .run();

        assert!(tester.ac_bus_1_output().is_unpowered());
        assert!(tester.ac_bus_2_output().is_unpowered());
    }

    fn tester_with() -> ElectricalCircuitTester {
        tester()
    }

    fn tester() -> ElectricalCircuitTester {
        ElectricalCircuitTester::new()
    }

    struct ElectricalCircuitTester {
        engine1: Engine,
        engine2: Engine,
        apu: AuxiliaryPowerUnit,
        ext_pwr: ExternalPowerSource,
        hyd: A320Hydraulic,
        elec: A320Electrical,
        overhead: A320ElectricalOverheadPanel,
        airspeed: Velocity,
        above_ground_level: Length,
    }

    impl ElectricalCircuitTester {
        fn new() -> ElectricalCircuitTester {
            ElectricalCircuitTester {
                engine1: ElectricalCircuitTester::new_stopped_engine(),
                engine2: ElectricalCircuitTester::new_stopped_engine(),
                apu: stopped_apu(),
                ext_pwr: ElectricalCircuitTester::new_disconnected_external_power(),
                hyd: A320Hydraulic::new(),
                elec: A320Electrical::new(),
                overhead: A320ElectricalOverheadPanel::new(),
                airspeed: Velocity::new::<knot>(250.),
                above_ground_level: Length::new::<foot>(5000.),
            }
        }

        fn running_engine_1(mut self) -> ElectricalCircuitTester {
            self.engine1 = ElectricalCircuitTester::new_running_engine();
            self
        }

        fn running_engine_2(mut self) -> ElectricalCircuitTester {
            self.engine2 = ElectricalCircuitTester::new_running_engine();
            self
        }

        fn running_engines(self) -> ElectricalCircuitTester {
            self.running_engine_1().and().running_engine_2()
        }

        fn running_apu(mut self) -> ElectricalCircuitTester {
            self.apu = running_apu();
            self
        }

        fn connected_external_power(mut self) -> ElectricalCircuitTester {
            self.ext_pwr = ElectricalCircuitTester::new_connected_external_power();
            self
        }

        fn empty_battery_1(mut self) -> ElectricalCircuitTester {
            self.elec.battery_1 = Battery::empty(1);
            self
        }

        fn empty_battery_2(mut self) -> ElectricalCircuitTester {
            self.elec.battery_2 = Battery::empty(2);
            self
        }

        fn airspeed(mut self, velocity: Velocity) -> ElectricalCircuitTester {
            self.airspeed = velocity;
            self
        }

        fn on_the_ground(mut self) -> ElectricalCircuitTester {
            self.above_ground_level = Length::new::<foot>(0.);
            self
        }

        fn and(self) -> ElectricalCircuitTester {
            self
        }

        fn then_continue_with(self) -> ElectricalCircuitTester {
            self
        }

        fn failed_ac_bus_1(mut self) -> ElectricalCircuitTester {
            self.elec.ac_bus_1.fail();
            self
        }

        fn failed_ac_bus_2(mut self) -> ElectricalCircuitTester {
            self.elec.ac_bus_2.fail();
            self
        }

        fn failed_tr_1(mut self) -> ElectricalCircuitTester {
            self.elec.tr_1.fail();
            self
        }

        fn failed_tr_2(mut self) -> ElectricalCircuitTester {
            self.elec.tr_2.fail();
            self
        }

        fn normal_ac_bus_1(mut self) -> ElectricalCircuitTester {
            self.elec.ac_bus_1.normal();
            self
        }

        fn running_emergency_generator(mut self) -> ElectricalCircuitTester {
            self.elec.emergency_gen.attempt_start();
            self
        }

        fn gen_1_off(mut self) -> ElectricalCircuitTester {
            self.overhead.gen_1.turn_off();
            self
        }

        fn gen_2_off(mut self) -> ElectricalCircuitTester {
            self.overhead.gen_2.turn_off();
            self
        }

        fn apu_gen_off(mut self) -> ElectricalCircuitTester {
            self.overhead.apu_gen.turn_off();
            self
        }

        fn ext_pwr_off(mut self) -> ElectricalCircuitTester {
            self.overhead.ext_pwr.turn_off();
            self
        }

        fn ac_ess_feed_altn(mut self) -> ElectricalCircuitTester {
            self.overhead.ac_ess_feed.push_altn();
            self
        }

        fn bat_1_off(mut self) -> ElectricalCircuitTester {
            self.overhead.bat_1.push_off();
            self
        }

        fn bat_2_off(mut self) -> ElectricalCircuitTester {
            self.overhead.bat_2.push_off();
            self
        }

        fn bus_tie_off(mut self) -> ElectricalCircuitTester {
            self.overhead.bus_tie.push_off();
            self
        }

        fn ac_bus_1_output(&self) -> Current {
            self.elec.ac_bus_1.output()
        }

        fn ac_bus_2_output(&self) -> Current {
            self.elec.ac_bus_2.output()
        }

        fn ac_ess_bus_output(&self) -> Current {
            self.elec.ac_ess_bus.output()
        }

        fn ac_ess_shed_bus_output(&self) -> Current {
            self.elec.ac_ess_shed_bus.output()
        }

        fn ac_stat_inv_bus_output(&self) -> Current {
            self.elec.ac_stat_inv_bus.output()
        }

        fn tr_1_output(&self) -> Current {
            self.elec.tr_1.output()
        }

        fn tr_2_output(&self) -> Current {
            self.elec.tr_2.output()
        }

        fn tr_ess_output(&self) -> Current {
            self.elec.tr_ess.output()
        }

        fn dc_bus_1_output(&self) -> Current {
            self.elec.dc_bus_1.output()
        }

        fn dc_bus_2_output(&self) -> Current {
            self.elec.dc_bus_2.output()
        }

        fn dc_bat_bus_output(&self) -> Current {
            self.elec.dc_bat_bus.output()
        }

        fn dc_ess_bus_output(&self) -> Current {
            self.elec.dc_ess_bus.output()
        }

        fn dc_ess_shed_bus_output(&self) -> Current {
            self.elec.dc_ess_shed_bus.output()
        }

        fn battery_1_input(&self) -> Current {
            self.elec.battery_1.get_input()
        }

        fn battery_2_input(&self) -> Current {
            self.elec.battery_2.get_input()
        }

        fn hot_bus_1_output(&self) -> Current {
            self.elec.hot_bus_1.output()
        }

        fn hot_bus_2_output(&self) -> Current {
            self.elec.hot_bus_2.output()
        }

        fn both_ac_ess_feed_contactors_open(&self) -> bool {
            self.elec.ac_ess_feed_contactor_1.is_open()
                && self.elec.ac_ess_feed_contactor_2.is_open()
        }

        fn run(mut self) -> ElectricalCircuitTester {
            let context = UpdateContext::new(
                Duration::from_secs(1),
                self.airspeed,
                self.above_ground_level,
                ThermodynamicTemperature::new::<degree_celsius>(0.),
            );
            self.elec.update(
                &context,
                &self.engine1,
                &self.engine2,
                &self.apu,
                &self.ext_pwr,
                &self.hyd,
                &self.overhead,
            );

            self
        }

        fn run_waiting_for(mut self, delta: Duration) -> ElectricalCircuitTester {
            // Firstly run without any time passing at all, such that if the DelayedTrueLogicGate reaches
            // the true state after waiting for the given time it will be reflected in its output.
            let context = UpdateContext::new(
                Duration::from_secs(0),
                self.airspeed,
                self.above_ground_level,
                ThermodynamicTemperature::new::<degree_celsius>(0.),
            );
            self.elec.update(
                &context,
                &self.engine1,
                &self.engine2,
                &self.apu,
                &self.ext_pwr,
                &self.hyd,
                &self.overhead,
            );

            let context = UpdateContext::new(
                delta,
                self.airspeed,
                self.above_ground_level,
                ThermodynamicTemperature::new::<degree_celsius>(0.),
            );
            self.elec.update(
                &context,
                &self.engine1,
                &self.engine2,
                &self.apu,
                &self.ext_pwr,
                &self.hyd,
                &self.overhead,
            );

            self
        }

        fn run_waiting_for_ac_ess_feed_transition(self) -> ElectricalCircuitTester {
            self.run_waiting_for(A320Electrical::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS)
        }

        fn run_waiting_until_just_before_ac_ess_feed_transition(self) -> ElectricalCircuitTester {
            self.run_waiting_for(
                A320Electrical::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS - Duration::from_millis(1),
            )
        }

        fn new_running_engine() -> Engine {
            let mut engine = Engine::new();
            engine.n2 = Ratio::new::<percent>(
                IntegratedDriveGenerator::ENGINE_N2_POWER_UP_OUTPUT_THRESHOLD + 1.,
            );

            engine
        }

        fn new_stopped_engine() -> Engine {
            let mut engine = Engine::new();
            engine.n2 = Ratio::new::<percent>(0.);

            engine
        }

        fn new_disconnected_external_power() -> ExternalPowerSource {
            let ext_pwr = ExternalPowerSource::new();

            ext_pwr
        }

        fn new_connected_external_power() -> ExternalPowerSource {
            let mut ext_pwr = ExternalPowerSource::new();
            ext_pwr.is_connected = true;

            ext_pwr
        }
    }
}
