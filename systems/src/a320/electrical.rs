use super::A320Hydraulic;
use crate::{
    apu::AuxiliaryPowerUnit,
    electrical::{
        combine_electric_sources, Battery, CombinedElectricSource, Contactor, ElectricSource,
        ElectricalBus, ElectricalBusStateFactory, ElectricalBusType, EmergencyGenerator,
        EngineGenerator, ExternalPowerSource, PowerSupply, Powerable, StaticInverter,
        TransformerRectifier,
    },
    engine::Engine,
    overhead::{
        AutoOffFaultPushButton, FaultReleasePushButton, NormalAltnFaultPushButton,
        OnOffAvailablePushButton, OnOffFaultPushButton,
    },
    shared::DelayedTrueLogicGate,
    simulator::{
        SimulatorElement, SimulatorElementVisitable, SimulatorElementVisitor, SimulatorReader,
        SimulatorWriter, UpdateContext,
    },
};
use std::time::Duration;
use uom::si::{f64::*, velocity::knot};

pub struct A320Electrical {
    alternating_current: A320AlternatingCurrentElectrical,
    direct_current: A320DirectCurrentElectrical,
}
impl A320Electrical {
    pub fn new() -> A320Electrical {
        A320Electrical {
            alternating_current: A320AlternatingCurrentElectrical::new(),
            direct_current: A320DirectCurrentElectrical::new(),
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
        self.alternating_current
            .update(context, engine1, engine2, apu, ext_pwr, hydraulic, overhead);

        self.direct_current.update_with_alternating_current_state(
            context,
            overhead,
            &self.alternating_current,
        );

        self.alternating_current
            .update_with_direct_current_state(context, &self.direct_current);

        self.debug_assert_invariants();
    }

    fn ac_bus_1(&self) -> &ElectricalBus {
        self.alternating_current.ac_bus_1()
    }

    fn ac_bus_2(&self) -> &ElectricalBus {
        self.alternating_current.ac_bus_2()
    }

    fn ac_ess_bus(&self) -> &ElectricalBus {
        self.alternating_current.ac_ess_bus()
    }

    fn ac_ess_shed_bus(&self) -> &ElectricalBus {
        self.alternating_current.ac_ess_shed_bus()
    }

    fn ac_stat_inv_bus(&self) -> &ElectricalBus {
        self.alternating_current.ac_stat_inv_bus()
    }

    fn dc_bus_1(&self) -> &ElectricalBus {
        self.direct_current.dc_bus_1()
    }

    fn dc_bus_2(&self) -> &ElectricalBus {
        self.direct_current.dc_bus_2()
    }

    fn dc_ess_bus(&self) -> &ElectricalBus {
        self.direct_current.dc_ess_bus()
    }

    fn dc_ess_shed_bus(&self) -> &ElectricalBus {
        self.direct_current.dc_ess_shed_bus()
    }

    fn dc_bat_bus(&self) -> &ElectricalBus {
        self.direct_current.dc_bat_bus()
    }

    fn hot_bus_1(&self) -> &ElectricalBus {
        self.direct_current.hot_bus_1()
    }

    fn hot_bus_2(&self) -> &ElectricalBus {
        self.direct_current.hot_bus_2()
    }

    fn debug_assert_invariants(&self) {
        self.alternating_current.debug_assert_invariants();
        self.direct_current.debug_assert_invariants();
    }
}
impl ElectricalBusStateFactory for A320Electrical {
    fn create_power_supply(&self) -> PowerSupply {
        let mut state = PowerSupply::new();
        state.add(self.ac_bus_1());
        state.add(self.ac_bus_2());
        state.add(self.ac_ess_bus());
        state.add(self.ac_ess_shed_bus());
        state.add(self.ac_stat_inv_bus());
        state.add(self.dc_bus_1());
        state.add(self.dc_bus_2());
        state.add(self.dc_ess_bus());
        state.add(self.dc_ess_shed_bus());
        state.add(self.dc_bat_bus());
        state.add(self.hot_bus_1());
        state.add(self.hot_bus_2());

        state
    }
}
impl SimulatorElementVisitable for A320Electrical {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        self.alternating_current.accept(visitor);
        self.direct_current.accept(visitor);
        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for A320Electrical {}

trait AlternatingCurrentState {
    fn ac_bus_1_and_2_unpowered(&self) -> bool;
    fn tr_1_and_2_available(&self) -> bool;
    fn ac_1_and_2_and_emergency_gen_unpowered(&self) -> bool;
    fn ac_1_and_2_and_emergency_gen_unpowered_and_velocity_equal_to_or_greater_than_50_knots(
        &self,
        context: &UpdateContext,
    ) -> bool;
    fn tr_1(&self) -> &TransformerRectifier;
    fn tr_2(&self) -> &TransformerRectifier;
    fn tr_ess(&self) -> &TransformerRectifier;
}

struct A320AlternatingCurrentElectrical {
    main_power_sources: A320MainPowerSources,
    ac_ess_feed_contactors: A320AcEssFeedContactors,
    ac_bus_1: ElectricalBus,
    ac_bus_2: ElectricalBus,
    ac_ess_bus: ElectricalBus,
    ac_ess_shed_bus: ElectricalBus,
    ac_ess_shed_contactor: Contactor,
    tr_1: TransformerRectifier,
    tr_2: TransformerRectifier,
    tr_ess: TransformerRectifier,
    ac_ess_to_tr_ess_contactor: Contactor,
    emergency_gen: EmergencyGenerator,
    emergency_gen_contactor: Contactor,
    static_inv_to_ac_ess_bus_contactor: Contactor,
    ac_stat_inv_bus: ElectricalBus,
}
impl A320AlternatingCurrentElectrical {
    fn new() -> Self {
        A320AlternatingCurrentElectrical {
            main_power_sources: A320MainPowerSources::new(),
            ac_ess_feed_contactors: A320AcEssFeedContactors::new(),
            ac_bus_1: ElectricalBus::new(ElectricalBusType::AlternatingCurrent(1)),
            ac_bus_2: ElectricalBus::new(ElectricalBusType::AlternatingCurrent(2)),
            ac_ess_bus: ElectricalBus::new(ElectricalBusType::AlternatingCurrentEssential),
            ac_ess_shed_bus: ElectricalBus::new(ElectricalBusType::AlternatingCurrentEssentialShed),
            ac_ess_shed_contactor: Contactor::new("8XH"),
            tr_1: TransformerRectifier::new(1),
            tr_2: TransformerRectifier::new(2),
            tr_ess: TransformerRectifier::new(3),
            ac_ess_to_tr_ess_contactor: Contactor::new("15XE1"),
            emergency_gen: EmergencyGenerator::new(),
            emergency_gen_contactor: Contactor::new("2XE"),
            static_inv_to_ac_ess_bus_contactor: Contactor::new("15XE2"),
            ac_stat_inv_bus: ElectricalBus::new(
                ElectricalBusType::AlternatingCurrentStaticInverter,
            ),
        }
    }

    fn update(
        &mut self,
        context: &UpdateContext,
        engine1: &Engine,
        engine2: &Engine,
        apu: &AuxiliaryPowerUnit,
        ext_pwr: &ExternalPowerSource,
        hydraulic: &A320Hydraulic,
        overhead: &A320ElectricalOverheadPanel,
    ) {
        self.emergency_gen.update(
            // ON GROUND BAT ONLY SPEED <= 100 kts scenario. We'll probably need to move this logic into
            // the ram air turbine, emergency generator and hydraulic implementation.
            hydraulic.is_blue_pressurised()
                && context.indicated_airspeed > Velocity::new::<knot>(100.),
        );

        self.main_power_sources
            .update(context, engine1, engine2, apu, ext_pwr, overhead);

        self.ac_bus_1
            .powered_by(&self.main_power_sources.ac_bus_1_electric_sources());
        self.ac_bus_2
            .powered_by(&self.main_power_sources.ac_bus_2_electric_sources());

        self.tr_1.powered_by(&self.ac_bus_1);
        self.tr_2.powered_by(&self.ac_bus_2);

        self.ac_ess_feed_contactors
            .update(context, &self.ac_bus_1, &self.ac_bus_2, overhead);

        self.ac_ess_bus
            .powered_by(&self.ac_ess_feed_contactors.electric_sources());

        self.emergency_gen_contactor.close_when(
            self.ac_bus_1.is_unpowered()
                && self.ac_bus_2.is_unpowered()
                && self.emergency_gen.is_powered(),
        );
        self.emergency_gen_contactor.powered_by(&self.emergency_gen);

        self.ac_ess_to_tr_ess_contactor.powered_by(&self.ac_ess_bus);
        self.ac_ess_to_tr_ess_contactor
            .or_powered_by(&self.emergency_gen_contactor);

        self.ac_ess_to_tr_ess_contactor.close_when(
            (!self.tr_1_and_2_available() && self.ac_ess_feed_contactors.provides_power())
                || self.emergency_gen_contactor.is_powered(),
        );

        self.ac_ess_bus
            .or_powered_by(&self.ac_ess_to_tr_ess_contactor);

        self.ac_ess_shed_contactor.powered_by(&self.ac_ess_bus);

        self.tr_ess.powered_by(&self.ac_ess_to_tr_ess_contactor);
        self.tr_ess.or_powered_by(&self.emergency_gen_contactor);

        self.update_shedding();
    }

    fn update_with_direct_current_state<T: DirectCurrentState>(
        &mut self,
        context: &UpdateContext,
        dc_state: &T,
    ) {
        self.ac_stat_inv_bus.powered_by(dc_state.static_inverter());
        self.static_inv_to_ac_ess_bus_contactor
            .close_when(self.should_close_15xe2_contactor(context));
        self.static_inv_to_ac_ess_bus_contactor
            .powered_by(dc_state.static_inverter());
        self.ac_ess_bus
            .or_powered_by(&self.static_inv_to_ac_ess_bus_contactor);
    }

    fn update_shedding(&mut self) {
        let ac_bus_or_emergency_gen_provides_power = self.ac_bus_1.is_powered()
            || self.ac_bus_2.is_powered()
            || self.emergency_gen.is_powered();
        self.ac_ess_shed_contactor
            .close_when(ac_bus_or_emergency_gen_provides_power);

        self.ac_ess_shed_bus.powered_by(&self.ac_ess_shed_contactor);
    }

    /// Determines if 15XE2 should be closed. 15XE2 is the contactor which connects
    /// the static inverter to the AC ESS BUS.
    fn should_close_15xe2_contactor(&self, context: &UpdateContext) -> bool {
        self.ac_1_and_2_and_emergency_gen_unpowered_and_velocity_equal_to_or_greater_than_50_knots(
            context,
        )
    }

    fn debug_assert_invariants(&self) {
        debug_assert!(self.static_inverter_or_emergency_gen_powers_ac_ess_bus());
    }

    fn static_inverter_or_emergency_gen_powers_ac_ess_bus(&self) -> bool {
        !(self.static_inv_to_ac_ess_bus_contactor.is_closed()
            && self.ac_ess_to_tr_ess_contactor.is_closed())
    }

    fn ac_bus_1(&self) -> &ElectricalBus {
        &self.ac_bus_1
    }

    fn ac_bus_2(&self) -> &ElectricalBus {
        &self.ac_bus_2
    }

    fn ac_ess_bus(&self) -> &ElectricalBus {
        &self.ac_ess_bus
    }

    fn ac_ess_shed_bus(&self) -> &ElectricalBus {
        &self.ac_ess_shed_bus
    }

    fn ac_stat_inv_bus(&self) -> &ElectricalBus {
        &self.ac_stat_inv_bus
    }
}
impl AlternatingCurrentState for A320AlternatingCurrentElectrical {
    fn ac_bus_1_and_2_unpowered(&self) -> bool {
        self.ac_bus_1.is_unpowered() && self.ac_bus_2.is_unpowered()
    }

    fn tr_1_and_2_available(&self) -> bool {
        self.tr_1.is_powered() && self.tr_2.is_powered()
    }

    fn ac_1_and_2_and_emergency_gen_unpowered(&self) -> bool {
        self.ac_bus_1_and_2_unpowered() && self.emergency_gen.is_unpowered()
    }

    fn ac_1_and_2_and_emergency_gen_unpowered_and_velocity_equal_to_or_greater_than_50_knots(
        &self,
        context: &UpdateContext,
    ) -> bool {
        self.ac_1_and_2_and_emergency_gen_unpowered()
            && context.indicated_airspeed >= Velocity::new::<knot>(50.)
    }

    fn tr_1(&self) -> &TransformerRectifier {
        &self.tr_1
    }

    fn tr_2(&self) -> &TransformerRectifier {
        &self.tr_2
    }

    fn tr_ess(&self) -> &TransformerRectifier {
        &self.tr_ess
    }
}
impl SimulatorElementVisitable for A320AlternatingCurrentElectrical {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        self.emergency_gen.accept(visitor);
        self.main_power_sources.accept(visitor);
        self.ac_ess_feed_contactors.accept(visitor);
        self.tr_1.accept(visitor);
        self.tr_2.accept(visitor);
        self.tr_ess.accept(visitor);

        self.ac_ess_shed_contactor.accept(visitor);
        self.ac_ess_to_tr_ess_contactor.accept(visitor);
        self.emergency_gen_contactor.accept(visitor);
        self.static_inv_to_ac_ess_bus_contactor.accept(visitor);

        self.ac_bus_1.accept(visitor);
        self.ac_bus_2.accept(visitor);
        self.ac_ess_bus.accept(visitor);
        self.ac_ess_shed_bus.accept(visitor);
        self.ac_stat_inv_bus.accept(visitor);

        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for A320AlternatingCurrentElectrical {}

trait DirectCurrentState {
    fn static_inverter(&self) -> &StaticInverter;
}

struct A320DirectCurrentElectrical {
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
    static_inverter: StaticInverter,
    hot_bus_1: ElectricalBus,
    hot_bus_2: ElectricalBus,
    tr_1_contactor: Contactor,
    tr_2_contactor: Contactor,
    tr_ess_contactor: Contactor,
}
impl A320DirectCurrentElectrical {
    fn new() -> Self {
        A320DirectCurrentElectrical {
            dc_bus_1: ElectricalBus::new(ElectricalBusType::DirectCurrent(1)),
            dc_bus_1_tie_contactor: Contactor::new("1PC1"),
            dc_bus_2: ElectricalBus::new(ElectricalBusType::DirectCurrent(2)),
            dc_bus_2_tie_contactor: Contactor::new("1PC2"),
            dc_bat_bus: ElectricalBus::new(ElectricalBusType::DirectCurrentBattery),
            dc_ess_bus: ElectricalBus::new(ElectricalBusType::DirectCurrentEssential),
            dc_bat_bus_to_dc_ess_bus_contactor: Contactor::new("4PC"),
            dc_ess_shed_bus: ElectricalBus::new(ElectricalBusType::DirectCurrentEssentialShed),
            dc_ess_shed_contactor: Contactor::new("8PH"),
            battery_1: Battery::full(10),
            battery_1_contactor: Contactor::new("6PB1"),
            battery_2: Battery::full(11),
            battery_2_contactor: Contactor::new("6PB2"),
            battery_2_to_dc_ess_bus_contactor: Contactor::new("2XB2"),
            battery_1_to_static_inv_contactor: Contactor::new("2XB1"),
            static_inverter: StaticInverter::new(),
            hot_bus_1: ElectricalBus::new(ElectricalBusType::DirectCurrentHot(1)),
            hot_bus_2: ElectricalBus::new(ElectricalBusType::DirectCurrentHot(2)),
            tr_1_contactor: Contactor::new("5PU1"),
            tr_2_contactor: Contactor::new("5PU2"),
            tr_ess_contactor: Contactor::new("3PE"),
        }
    }

    fn update_with_alternating_current_state<T: AlternatingCurrentState>(
        &mut self,
        context: &UpdateContext,
        overhead: &A320ElectricalOverheadPanel,
        ac_state: &T,
    ) {
        self.tr_1_contactor.close_when(ac_state.tr_1().is_powered());
        self.tr_1_contactor.powered_by(ac_state.tr_1());

        self.tr_2_contactor.close_when(ac_state.tr_2().is_powered());
        self.tr_2_contactor.powered_by(ac_state.tr_2());

        self.tr_ess_contactor
            .close_when(!ac_state.tr_1_and_2_available() && ac_state.tr_ess().is_powered());
        self.tr_ess_contactor.powered_by(ac_state.tr_ess());

        self.dc_bus_1.powered_by(&self.tr_1_contactor);
        self.dc_bus_2.powered_by(&self.tr_2_contactor);

        self.dc_bus_1_tie_contactor.powered_by(&self.dc_bus_1);
        self.dc_bus_2_tie_contactor.powered_by(&self.dc_bus_2);

        self.dc_bus_1_tie_contactor
            .close_when(self.dc_bus_1.is_powered() || self.dc_bus_2.is_powered());
        self.dc_bus_2_tie_contactor.close_when(
            (!self.dc_bus_1.is_powered() && self.dc_bus_2.is_powered())
                || (!self.dc_bus_2.is_powered() && self.dc_bus_1.is_powered()),
        );

        self.dc_bat_bus.powered_by(&self.dc_bus_1_tie_contactor);
        self.dc_bat_bus.or_powered_by(&self.dc_bus_2_tie_contactor);

        self.dc_bus_1_tie_contactor.or_powered_by(&self.dc_bat_bus);
        self.dc_bus_2_tie_contactor.or_powered_by(&self.dc_bat_bus);
        self.dc_bus_1.or_powered_by(&self.dc_bus_1_tie_contactor);
        self.dc_bus_2.or_powered_by(&self.dc_bus_2_tie_contactor);

        self.battery_1_contactor.powered_by(&self.dc_bat_bus);
        self.battery_2_contactor.powered_by(&self.dc_bat_bus);

        // TODO: The actual logic for battery contactors is more complex, however
        // not all systems is relates to are implemented yet. We'll have to get back to this later.
        let airspeed_below_100_knots = context.indicated_airspeed < Velocity::new::<knot>(100.);
        let batteries_should_supply_bat_bus =
            ac_state.ac_bus_1_and_2_unpowered() && airspeed_below_100_knots;
        self.battery_1_contactor.close_when(
            overhead.bat_1_is_auto()
                && (!self.battery_1.is_full() || batteries_should_supply_bat_bus),
        );
        self.battery_2_contactor.close_when(
            overhead.bat_2_is_auto()
                && (!self.battery_2.is_full() || batteries_should_supply_bat_bus),
        );

        self.battery_1.powered_by(&self.battery_1_contactor);
        self.battery_2.powered_by(&self.battery_2_contactor);

        self.battery_1_contactor.or_powered_by(&self.battery_1);
        self.battery_2_contactor.or_powered_by(&self.battery_2);

        self.dc_bat_bus
            .or_powered_by_both_batteries(&self.battery_1_contactor, &self.battery_2_contactor);

        self.hot_bus_1.powered_by(&self.battery_1_contactor);
        self.hot_bus_1.or_powered_by(&self.battery_1);
        self.hot_bus_2.powered_by(&self.battery_2_contactor);
        self.hot_bus_2.or_powered_by(&self.battery_2);

        self.dc_bat_bus_to_dc_ess_bus_contactor
            .powered_by(&self.dc_bat_bus);
        self.dc_bat_bus_to_dc_ess_bus_contactor
            .close_when(ac_state.tr_1_and_2_available());
        self.battery_2_to_dc_ess_bus_contactor
            .powered_by(&self.battery_2);

        let should_close_2xb_contactor = self.should_close_2xb_contactors(context, ac_state);
        self.battery_2_to_dc_ess_bus_contactor
            .close_when(should_close_2xb_contactor);

        self.battery_1_to_static_inv_contactor
            .close_when(should_close_2xb_contactor);

        self.battery_1_to_static_inv_contactor
            .powered_by(&self.battery_1);

        self.static_inverter
            .powered_by(&self.battery_1_to_static_inv_contactor);

        self.dc_ess_bus
            .powered_by(&self.dc_bat_bus_to_dc_ess_bus_contactor);
        self.dc_ess_bus.or_powered_by(&self.tr_ess_contactor);
        self.dc_ess_bus
            .or_powered_by(&self.battery_2_to_dc_ess_bus_contactor);

        self.dc_ess_shed_contactor.powered_by(&self.dc_ess_bus);
        self.dc_ess_shed_contactor
            .close_when(self.battery_2_to_dc_ess_bus_contactor.is_open());
        self.dc_ess_shed_bus.powered_by(&self.dc_ess_shed_contactor);
    }

    /// Determines if the 2XB contactors should be closed. 2XB are the two contactors
    /// which connect BAT2 to DC ESS BUS; and BAT 1 to the static inverter.
    fn should_close_2xb_contactors<T: AlternatingCurrentState>(
        &self,
        context: &UpdateContext,
        ac_state: &T,
    ) -> bool {
        (self.battery_contactors_closed_and_speed_less_than_50_knots(context)
            && ac_state.ac_1_and_2_and_emergency_gen_unpowered())
            || ac_state.ac_1_and_2_and_emergency_gen_unpowered_and_velocity_equal_to_or_greater_than_50_knots(context)
    }

    fn battery_contactors_closed_and_speed_less_than_50_knots(
        &self,
        context: &UpdateContext,
    ) -> bool {
        context.indicated_airspeed < Velocity::new::<knot>(50.)
            && self.battery_1_contactor.is_closed()
            && self.battery_2_contactor.is_closed()
    }

    fn debug_assert_invariants(&self) {
        debug_assert!(self.battery_never_powers_dc_ess_shed());
        debug_assert!(self.max_one_source_powers_dc_ess_bus());
        debug_assert!(
            self.batteries_power_both_static_inv_and_dc_ess_bus_at_the_same_time_or_not_at_all()
        );
    }

    fn battery_never_powers_dc_ess_shed(&self) -> bool {
        !(self.battery_2_to_dc_ess_bus_contactor.is_closed()
            && self.dc_ess_shed_contactor.is_closed())
    }

    fn max_one_source_powers_dc_ess_bus(&self) -> bool {
        (!self.battery_2_to_dc_ess_bus_contactor.is_closed()
            && !self.dc_bat_bus_to_dc_ess_bus_contactor.is_closed()
            && !self.tr_ess_contactor.is_closed())
            || (self.battery_2_to_dc_ess_bus_contactor.is_closed()
                ^ self.dc_bat_bus_to_dc_ess_bus_contactor.is_closed()
                ^ self.tr_ess_contactor.is_closed())
    }

    fn batteries_power_both_static_inv_and_dc_ess_bus_at_the_same_time_or_not_at_all(
        &self,
    ) -> bool {
        self.battery_1_to_static_inv_contactor.is_closed()
            == self.battery_2_to_dc_ess_bus_contactor.is_closed()
    }

    fn dc_bus_1(&self) -> &ElectricalBus {
        &self.dc_bus_1
    }

    fn dc_bus_2(&self) -> &ElectricalBus {
        &self.dc_bus_2
    }

    fn dc_ess_bus(&self) -> &ElectricalBus {
        &self.dc_ess_bus
    }

    fn dc_ess_shed_bus(&self) -> &ElectricalBus {
        &self.dc_ess_shed_bus
    }

    fn dc_bat_bus(&self) -> &ElectricalBus {
        &self.dc_bat_bus
    }

    fn hot_bus_1(&self) -> &ElectricalBus {
        &self.hot_bus_1
    }

    fn hot_bus_2(&self) -> &ElectricalBus {
        &self.hot_bus_2
    }
}
impl DirectCurrentState for A320DirectCurrentElectrical {
    fn static_inverter(&self) -> &StaticInverter {
        &self.static_inverter
    }
}
impl SimulatorElementVisitable for A320DirectCurrentElectrical {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        self.battery_1.accept(visitor);
        self.battery_2.accept(visitor);
        self.static_inverter.accept(visitor);

        self.dc_bus_1_tie_contactor.accept(visitor);
        self.dc_bus_2_tie_contactor.accept(visitor);
        self.dc_bat_bus_to_dc_ess_bus_contactor.accept(visitor);
        self.dc_ess_shed_contactor.accept(visitor);
        self.battery_1_contactor.accept(visitor);
        self.battery_2_contactor.accept(visitor);
        self.battery_2_to_dc_ess_bus_contactor.accept(visitor);
        self.battery_1_to_static_inv_contactor.accept(visitor);
        self.tr_1_contactor.accept(visitor);
        self.tr_2_contactor.accept(visitor);
        self.tr_ess_contactor.accept(visitor);

        self.dc_bus_1.accept(visitor);
        self.dc_bus_2.accept(visitor);
        self.dc_bat_bus.accept(visitor);
        self.dc_ess_bus.accept(visitor);
        self.dc_ess_shed_bus.accept(visitor);
        self.hot_bus_1.accept(visitor);
        self.hot_bus_2.accept(visitor);

        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for A320DirectCurrentElectrical {}

struct A320MainPowerSources {
    engine_1_gen: EngineGenerator,
    engine_1_gen_contactor: Contactor,
    engine_2_gen: EngineGenerator,
    engine_2_gen_contactor: Contactor,
    bus_tie_1_contactor: Contactor,
    bus_tie_2_contactor: Contactor,
    apu_gen_contactor: Contactor,
    ext_pwr_contactor: Contactor,
}
impl A320MainPowerSources {
    fn new() -> Self {
        A320MainPowerSources {
            engine_1_gen: EngineGenerator::new(1),
            engine_1_gen_contactor: Contactor::new("9XU1"),
            engine_2_gen: EngineGenerator::new(2),
            engine_2_gen_contactor: Contactor::new("9XU2"),
            bus_tie_1_contactor: Contactor::new("11XU1"),
            bus_tie_2_contactor: Contactor::new("11XU2"),
            apu_gen_contactor: Contactor::new("3XS"),
            ext_pwr_contactor: Contactor::new("3XG"),
        }
    }

    fn update(
        &mut self,
        context: &UpdateContext,
        engine1: &Engine,
        engine2: &Engine,
        apu: &AuxiliaryPowerUnit,
        ext_pwr: &ExternalPowerSource,
        overhead: &A320ElectricalOverheadPanel,
    ) {
        self.engine_1_gen.update(context, engine1, &overhead.idg_1);
        self.engine_2_gen.update(context, engine2, &overhead.idg_2);

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
            && apu.is_powered()
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

        self.apu_gen_contactor.powered_by(apu);
        self.ext_pwr_contactor.powered_by(ext_pwr);

        self.engine_1_gen_contactor.powered_by(&self.engine_1_gen);
        self.bus_tie_1_contactor
            .powered_by(&self.engine_1_gen_contactor);
        self.bus_tie_1_contactor
            .or_powered_by(&self.apu_gen_contactor);
        self.bus_tie_1_contactor
            .or_powered_by(&self.ext_pwr_contactor);

        self.engine_2_gen_contactor.powered_by(&self.engine_2_gen);
        self.bus_tie_2_contactor
            .powered_by(&self.engine_2_gen_contactor);
        self.bus_tie_2_contactor
            .or_powered_by(&self.apu_gen_contactor);
        self.bus_tie_2_contactor
            .or_powered_by(&self.ext_pwr_contactor);

        self.bus_tie_1_contactor
            .or_powered_by(&self.bus_tie_2_contactor);
        self.bus_tie_2_contactor
            .or_powered_by(&self.bus_tie_1_contactor);
    }

    fn ac_bus_1_electric_sources(&self) -> CombinedElectricSource {
        combine_electric_sources(vec![
            &self.engine_1_gen_contactor,
            &self.bus_tie_1_contactor,
        ])
    }

    fn ac_bus_2_electric_sources(&self) -> CombinedElectricSource {
        combine_electric_sources(vec![
            &self.engine_2_gen_contactor,
            &self.bus_tie_2_contactor,
        ])
    }
}
impl SimulatorElementVisitable for A320MainPowerSources {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        self.engine_1_gen.accept(visitor);
        self.engine_2_gen.accept(visitor);

        self.engine_1_gen_contactor.accept(visitor);
        self.engine_2_gen_contactor.accept(visitor);
        self.bus_tie_1_contactor.accept(visitor);
        self.bus_tie_2_contactor.accept(visitor);
        self.apu_gen_contactor.accept(visitor);
        self.ext_pwr_contactor.accept(visitor);

        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for A320MainPowerSources {}

struct A320AcEssFeedContactors {
    ac_ess_feed_contactor_1: Contactor,
    ac_ess_feed_contactor_2: Contactor,
    ac_ess_feed_contactor_delay_logic_gate: DelayedTrueLogicGate,
}
impl A320AcEssFeedContactors {
    const AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS: Duration = Duration::from_secs(3);

    fn new() -> Self {
        A320AcEssFeedContactors {
            ac_ess_feed_contactor_1: Contactor::new("3XC1"),
            ac_ess_feed_contactor_2: Contactor::new("3XC2"),
            ac_ess_feed_contactor_delay_logic_gate: DelayedTrueLogicGate::new(
                A320AcEssFeedContactors::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS,
            ),
        }
    }

    fn update(
        &mut self,
        context: &UpdateContext,
        ac_bus_1: &ElectricalBus,
        ac_bus_2: &ElectricalBus,
        overhead: &A320ElectricalOverheadPanel,
    ) {
        self.ac_ess_feed_contactor_delay_logic_gate
            .update(context, ac_bus_1.is_unpowered());

        self.ac_ess_feed_contactor_1.close_when(
            ac_bus_1.is_powered()
                && (!self.ac_ess_feed_contactor_delay_logic_gate.output()
                    && overhead.ac_ess_feed_is_normal()),
        );
        self.ac_ess_feed_contactor_2.close_when(
            ac_bus_2.is_powered()
                && (self.ac_ess_feed_contactor_delay_logic_gate.output()
                    || overhead.ac_ess_feed_is_altn()),
        );

        self.ac_ess_feed_contactor_1.powered_by(ac_bus_1);
        self.ac_ess_feed_contactor_2.powered_by(ac_bus_2);
    }

    fn electric_sources(&self) -> CombinedElectricSource {
        combine_electric_sources(vec![
            &self.ac_ess_feed_contactor_1,
            &self.ac_ess_feed_contactor_2,
        ])
    }

    fn provides_power(&self) -> bool {
        self.electric_sources().output().is_powered()
    }
}
impl SimulatorElementVisitable for A320AcEssFeedContactors {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        self.ac_ess_feed_contactor_1.accept(visitor);
        self.ac_ess_feed_contactor_2.accept(visitor);

        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for A320AcEssFeedContactors {}

pub struct A320ElectricalOverheadPanel {
    bat_1: AutoOffFaultPushButton,
    bat_2: AutoOffFaultPushButton,
    idg_1: FaultReleasePushButton,
    idg_2: FaultReleasePushButton,
    gen_1: OnOffFaultPushButton,
    gen_2: OnOffFaultPushButton,
    apu_gen: OnOffFaultPushButton,
    bus_tie: AutoOffFaultPushButton,
    ac_ess_feed: NormalAltnFaultPushButton,
    galy_and_cab: AutoOffFaultPushButton,
    ext_pwr: OnOffAvailablePushButton,
    commercial: OnOffFaultPushButton,
}
impl A320ElectricalOverheadPanel {
    pub fn new() -> A320ElectricalOverheadPanel {
        A320ElectricalOverheadPanel {
            bat_1: AutoOffFaultPushButton::new_auto("ELEC_BAT_10"),
            bat_2: AutoOffFaultPushButton::new_auto("ELEC_BAT_11"),
            idg_1: FaultReleasePushButton::new_in("ELEC_IDG_1"),
            idg_2: FaultReleasePushButton::new_in("ELEC_IDG_2"),
            gen_1: OnOffFaultPushButton::new_on("ELEC_ENG_GEN_1"),
            gen_2: OnOffFaultPushButton::new_on("ELEC_ENG_GEN_2"),
            apu_gen: OnOffFaultPushButton::new_on("ELEC_APU_GEN"),
            bus_tie: AutoOffFaultPushButton::new_auto("ELEC_BUS_TIE"),
            ac_ess_feed: NormalAltnFaultPushButton::new_normal("ELEC_AC_ESS_FEED"),
            galy_and_cab: AutoOffFaultPushButton::new_auto("ELEC_GALY_AND_CAB"),
            ext_pwr: OnOffAvailablePushButton::new_off("ELEC_EXT_PWR"),
            commercial: OnOffFaultPushButton::new_on("ELEC_COMMERCIAL"),
        }
    }

    pub fn update_after_elec(&mut self, electrical: &A320Electrical) {
        self.ac_ess_feed
            .set_fault(electrical.ac_ess_bus().is_unpowered());
    }

    fn generator_1_is_on(&self) -> bool {
        self.gen_1.is_on()
    }

    fn generator_2_is_on(&self) -> bool {
        self.gen_2.is_on()
    }

    pub fn external_power_is_available(&self) -> bool {
        self.ext_pwr.is_available()
    }

    pub fn external_power_is_on(&self) -> bool {
        self.ext_pwr.is_on()
    }

    pub fn apu_generator_is_on(&self) -> bool {
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

    #[cfg(test)]
    fn ac_ess_feed_has_fault(&self) -> bool {
        self.ac_ess_feed.has_fault()
    }
}
impl SimulatorElementVisitable for A320ElectricalOverheadPanel {
    fn accept(&mut self, visitor: &mut Box<&mut dyn SimulatorElementVisitor>) {
        self.bat_1.accept(visitor);
        self.bat_2.accept(visitor);
        self.idg_1.accept(visitor);
        self.idg_2.accept(visitor);
        self.gen_1.accept(visitor);
        self.gen_2.accept(visitor);
        self.apu_gen.accept(visitor);
        self.bus_tie.accept(visitor);
        self.ac_ess_feed.accept(visitor);
        self.galy_and_cab.accept(visitor);
        self.ext_pwr.accept(visitor);
        self.commercial.accept(visitor);

        visitor.visit(&mut Box::new(self));
    }
}
impl SimulatorElement for A320ElectricalOverheadPanel {}

#[cfg(test)]
mod a320_electrical_circuit_tests {
    use crate::{
        apu::tests::{running_apu, stopped_apu},
        electrical::{Current, ElectricPowerSource},
    };

    use uom::si::{
        length::foot, ratio::percent, thermodynamic_temperature::degree_celsius, velocity::knot,
    };

    use super::*;

    #[test]
    fn everything_off_batteries_empty() {
        let tester = tester_with()
            .bat_1_off()
            .empty_battery_1()
            .bat_2_off()
            .empty_battery_2()
            .and()
            .airspeed(Velocity::new::<knot>(0.))
            .run();

        assert_eq!(tester.ac_bus_1_output(), Current::none());
        assert_eq!(tester.ac_bus_2_output(), Current::none());
        assert_eq!(tester.ac_ess_bus_output(), Current::none());
        assert_eq!(tester.ac_ess_shed_bus_output(), Current::none());
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(tester.tr_1_input(), Current::none());
        assert_eq!(tester.tr_2_input(), Current::none());
        assert_eq!(tester.tr_ess_input(), Current::none());
        assert_eq!(tester.dc_bus_1_output(), Current::none());
        assert_eq!(tester.dc_bus_2_output(), Current::none());
        assert_eq!(tester.dc_bat_bus_output(), Current::none());
        assert_eq!(tester.dc_ess_bus_output(), Current::none());
        assert_eq!(tester.dc_ess_shed_bus_output(), Current::none());
        assert_eq!(tester.hot_bus_1_output(), Current::none());
        assert_eq!(tester.hot_bus_2_output(), Current::none());
    }

    #[test]
    fn everything_off() {
        let tester = tester_with()
            .bat_1_off()
            .bat_2_off()
            .and()
            .airspeed(Velocity::new::<knot>(0.))
            .run();

        assert_eq!(tester.ac_bus_1_output(), Current::none());
        assert_eq!(tester.ac_bus_2_output(), Current::none());
        assert_eq!(tester.ac_ess_bus_output(), Current::none());
        assert_eq!(tester.ac_ess_shed_bus_output(), Current::none());
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(tester.tr_1_input(), Current::none());
        assert_eq!(tester.tr_2_input(), Current::none());
        assert_eq!(tester.tr_ess_input(), Current::none());
        assert_eq!(tester.dc_bus_1_output(), Current::none());
        assert_eq!(tester.dc_bus_2_output(), Current::none());
        assert_eq!(tester.dc_bat_bus_output(), Current::none());
        assert_eq!(tester.dc_ess_bus_output(), Current::none());
        assert_eq!(tester.dc_ess_shed_bus_output(), Current::none());
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_norm_conf() {
        let tester = tester_with().running_engines().run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(
            tester.tr_1_input(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.tr_2_input(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(tester.tr_ess_input(), Current::none());
        assert_eq!(
            tester.dc_bus_1_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_bus_2_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(2))
        );
        assert_eq!(
            tester.dc_bat_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_only_gen_1_available() {
        let tester = tester_with().running_engine_1().run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(
            tester.tr_1_input(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.tr_2_input(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(tester.tr_ess_input(), Current::none());
        assert_eq!(
            tester.dc_bus_1_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_bus_2_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(2))
        );
        assert_eq!(
            tester.dc_bat_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_only_gen_2_available() {
        let tester = tester_with().running_engine_2().run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(
            tester.tr_1_input(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.tr_2_input(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(tester.tr_ess_input(), Current::none());
        assert_eq!(
            tester.dc_bus_1_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_bus_2_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(2))
        );
        assert_eq!(
            tester.dc_bat_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_only_apu_gen_available() {
        let tester = tester_with().running_apu().run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::ApuGenerator)
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::ApuGenerator)
        );
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::ApuGenerator)
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::ApuGenerator)
        );
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(
            tester.tr_1_input(),
            Current::some(ElectricPowerSource::ApuGenerator)
        );
        assert_eq!(
            tester.tr_2_input(),
            Current::some(ElectricPowerSource::ApuGenerator)
        );
        assert_eq!(tester.tr_ess_input(), Current::none());
        assert_eq!(
            tester.dc_bus_1_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_bus_2_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(2))
        );
        assert_eq!(
            tester.dc_bat_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
    }

    /// # Source
    /// Derived from A320 manual electrical distribution table
    /// (doesn't list external power, but we'll assume it's the same as other generators).
    #[test]
    fn distribution_table_only_external_power_available() {
        let tester = tester_with()
            .connected_external_power()
            .and()
            .ext_pwr_on()
            .run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::External)
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::External)
        );
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::External)
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::External)
        );
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(
            tester.tr_1_input(),
            Current::some(ElectricPowerSource::External)
        );
        assert_eq!(
            tester.tr_2_input(),
            Current::some(ElectricPowerSource::External)
        );
        assert_eq!(tester.tr_ess_input(), Current::none());
        assert_eq!(
            tester.dc_bus_1_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_bus_2_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(2))
        );
        assert_eq!(
            tester.dc_bat_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_emergency_config_before_emergency_gen_available() {
        let tester = tester().run();

        assert_eq!(tester.ac_bus_1_output(), Current::none());
        assert_eq!(tester.ac_bus_2_output(), Current::none());
        assert_eq!(tester.ac_ess_shed_bus_output(), Current::none());
        assert_eq!(
            tester.static_inverter_input(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.ac_stat_inv_bus_output(),
            Current::some(ElectricPowerSource::StaticInverter)
        );
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::StaticInverter)
        );
        assert_eq!(tester.tr_1_input(), Current::none());
        assert_eq!(tester.tr_2_input(), Current::none());
        assert_eq!(tester.tr_ess_input(), Current::none());
        assert_eq!(tester.dc_bus_1_output(), Current::none());
        assert_eq!(tester.dc_bus_2_output(), Current::none());
        assert_eq!(tester.dc_bat_bus_output(), Current::none());
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
        assert_eq!(tester.dc_ess_shed_bus_output(), Current::none());
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_emergency_config_after_emergency_gen_available() {
        let tester = tester_with().running_emergency_generator().run();

        assert_eq!(tester.ac_bus_1_output(), Current::none());
        assert_eq!(tester.ac_bus_2_output(), Current::none());
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EmergencyGenerator)
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::EmergencyGenerator)
        );
        assert_eq!(tester.tr_1_input(), Current::none());
        assert_eq!(tester.tr_2_input(), Current::none());
        assert_eq!(
            tester.tr_ess_input(),
            Current::some(ElectricPowerSource::EmergencyGenerator)
        );
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(tester.dc_bus_1_output(), Current::none());
        assert_eq!(tester.dc_bus_2_output(), Current::none());
        assert_eq!(tester.dc_bat_bus_output(), Current::none());
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(3))
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(3))
        );
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_tr_1_fault() {
        let tester = tester_with().running_engines().and().failed_tr_1().run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(
            tester.tr_1_input(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.tr_2_input(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.tr_ess_input(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.dc_bus_1_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(2))
        );
        assert_eq!(
            tester.dc_bus_2_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(2))
        );
        assert_eq!(
            tester.dc_bat_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(2))
        );
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(3))
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(3))
        );
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_tr_2_fault() {
        let tester = tester_with().running_engines().and().failed_tr_2().run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(
            tester.tr_1_input(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.tr_2_input(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.tr_ess_input(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.dc_bus_1_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_bus_2_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_bat_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(3))
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(3))
        );
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
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
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(
            tester.tr_1_input(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.tr_2_input(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.tr_ess_input(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(tester.dc_bus_1_output(), Current::none());
        assert_eq!(tester.dc_bus_2_output(), Current::none());
        assert_eq!(tester.dc_bat_bus_output(), Current::none());
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(3))
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(3))
        );
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
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

        assert_eq!(tester.ac_bus_1_output(), Current::none());
        assert_eq!(tester.ac_bus_2_output(), Current::none());
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EmergencyGenerator)
        );
        assert_eq!(
            tester.ac_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::EmergencyGenerator)
        );
        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_stat_inv_bus_output(), Current::none());
        assert_eq!(tester.tr_1_input(), Current::none());
        assert_eq!(tester.tr_2_input(), Current::none());
        assert_eq!(
            tester.tr_ess_input(),
            Current::some(ElectricPowerSource::EmergencyGenerator)
        );
        assert_eq!(tester.dc_bus_1_output(), Current::none());
        assert_eq!(tester.dc_bus_2_output(), Current::none());
        assert_eq!(tester.dc_bat_bus_output(), Current::none());
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(3))
        );
        assert_eq!(
            tester.dc_ess_shed_bus_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(3))
        );
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
    }

    /// # Source
    /// A320 manual electrical distribution table
    #[test]
    fn distribution_table_on_ground_bat_only_rat_stall_or_speed_between_50_to_100_knots() {
        let tester = tester_with()
            .running_emergency_generator()
            .airspeed(Velocity::new::<knot>(50.0))
            .and()
            .on_the_ground()
            .run();

        assert_eq!(tester.ac_bus_1_output(), Current::none());
        assert_eq!(tester.ac_bus_2_output(), Current::none());
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::StaticInverter)
        );
        assert_eq!(tester.ac_ess_shed_bus_output(), Current::none());
        assert_eq!(
            tester.static_inverter_input(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.ac_stat_inv_bus_output(),
            Current::some(ElectricPowerSource::StaticInverter)
        );
        assert_eq!(tester.tr_1_input(), Current::none());
        assert_eq!(tester.tr_2_input(), Current::none());
        assert_eq!(tester.tr_ess_input(), Current::none());
        assert_eq!(tester.dc_bus_1_output(), Current::none());
        assert_eq!(tester.dc_bus_2_output(), Current::none());
        assert_eq!(
            tester.dc_bat_bus_output(),
            Current::some(ElectricPowerSource::Batteries)
        );
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
        assert_eq!(tester.dc_ess_shed_bus_output(), Current::none());
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
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

        assert_eq!(tester.ac_bus_1_output(), Current::none());
        assert_eq!(tester.ac_bus_2_output(), Current::none());
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::none(),
            "AC ESS BUS shouldn't be powered below 50 knots when on batteries only."
        );
        assert_eq!(tester.ac_ess_shed_bus_output(), Current::none());
        assert_eq!(
            tester.static_inverter_input(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.ac_stat_inv_bus_output(),
            Current::some(ElectricPowerSource::StaticInverter)
        );
        assert_eq!(tester.tr_1_input(), Current::none());
        assert_eq!(tester.tr_2_input(), Current::none());
        assert_eq!(tester.tr_ess_input(), Current::none());
        assert_eq!(tester.dc_bus_1_output(), Current::none());
        assert_eq!(tester.dc_bus_2_output(), Current::none());
        assert_eq!(
            tester.dc_bat_bus_output(),
            Current::some(ElectricPowerSource::Batteries)
        );
        assert_eq!(
            tester.dc_ess_bus_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
        assert_eq!(tester.dc_ess_shed_bus_output(), Current::none());
        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::Battery(11))
        );
    }

    #[test]
    fn create_power_supply_returns_power_supply() {
        let tester = tester_with().running_engines().run();
        let power_supply = tester.create_power_supply();

        assert!(power_supply.is_powered(&ElectricalBusType::AlternatingCurrent(1)));
        assert!(power_supply.is_powered(&ElectricalBusType::AlternatingCurrent(2)));
        assert!(power_supply.is_powered(&ElectricalBusType::AlternatingCurrentEssential));
        assert!(power_supply.is_powered(&ElectricalBusType::AlternatingCurrentEssentialShed));
        assert!(!power_supply.is_powered(&ElectricalBusType::AlternatingCurrentStaticInverter));
        assert!(!power_supply.is_powered(&ElectricalBusType::AlternatingCurrentStaticInverter));
        assert!(power_supply.is_powered(&ElectricalBusType::DirectCurrent(1)));
        assert!(power_supply.is_powered(&ElectricalBusType::DirectCurrent(2)));
        assert!(power_supply.is_powered(&ElectricalBusType::DirectCurrentBattery));
        assert!(power_supply.is_powered(&ElectricalBusType::DirectCurrentEssential));
        assert!(power_supply.is_powered(&ElectricalBusType::DirectCurrentEssentialShed));
        assert!(power_supply.is_powered(&ElectricalBusType::DirectCurrentHot(1)));
        assert!(power_supply.is_powered(&ElectricalBusType::DirectCurrentHot(2)));
    }

    #[test]
    fn when_engine_1_and_apu_running_apu_powers_ac_bus_2() {
        let tester = tester_with().running_engine_1().and().running_apu().run();

        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::ApuGenerator)
        );
    }

    #[test]
    fn when_engine_2_and_apu_running_apu_powers_ac_bus_1() {
        let tester = tester_with().running_engine_2().and().running_apu().run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::ApuGenerator)
        );
    }

    #[test]
    fn when_only_apu_running_apu_powers_ac_bus_1_and_2() {
        let tester = tester_with().running_apu().run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::ApuGenerator)
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::ApuGenerator)
        );
    }

    #[test]
    fn when_engine_1_running_and_external_power_connected_ext_pwr_powers_ac_bus_2() {
        let tester = tester_with()
            .running_engine_1()
            .connected_external_power()
            .and()
            .ext_pwr_on()
            .run();

        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::External)
        );
    }

    #[test]
    fn when_engine_2_running_and_external_power_connected_ext_pwr_powers_ac_bus_1() {
        let tester = tester_with()
            .running_engine_2()
            .connected_external_power()
            .and()
            .ext_pwr_on()
            .run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::External)
        );
    }

    #[test]
    fn when_only_external_power_connected_ext_pwr_powers_ac_bus_1_and_2() {
        let tester = tester_with()
            .connected_external_power()
            .and()
            .ext_pwr_on()
            .run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::External)
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::External)
        );
    }

    #[test]
    fn when_external_power_connected_and_apu_running_external_power_has_priority() {
        let tester = tester_with()
            .connected_external_power()
            .ext_pwr_on()
            .and()
            .running_apu()
            .run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::External)
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::External)
        );
    }

    #[test]
    fn when_both_engines_running_and_external_power_connected_engines_power_ac_buses() {
        let tester = tester_with()
            .running_engines()
            .and()
            .connected_external_power()
            .run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
    }

    #[test]
    fn when_both_engines_running_and_apu_running_engines_power_ac_buses() {
        let tester = tester_with().running_engines().and().running_apu().run();

        assert_eq!(
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
    }

    #[test]
    fn ac_bus_1_powers_ac_ess_bus_whenever_it_is_powered() {
        let tester = tester_with().running_engines().run();

        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
    }

    #[test]
    fn when_ac_bus_1_becomes_unpowered_but_ac_bus_2_powered_nothing_powers_ac_ess_bus_for_a_while()
    {
        let tester = tester_with()
            .running_engine_2()
            .and()
            .bus_tie_off()
            .run_waiting_until_just_before_ac_ess_feed_transition();

        assert_eq!(tester.static_inverter_input(), Current::none());
        assert_eq!(tester.ac_ess_bus_output(), Current::none());
    }

    #[test]
    fn when_ac_bus_1_becomes_unpowered_but_ac_bus_2_powered_nothing_powers_dc_ess_bus_for_a_while()
    {
        let tester = tester_with()
            .running_engine_2()
            .and()
            .bus_tie_off()
            .run_waiting_until_just_before_ac_ess_feed_transition();

        assert_eq!(tester.dc_ess_bus_output(), Current::none());
    }

    #[test]
    fn bat_only_low_airspeed_when_a_single_battery_contactor_closed_static_inverter_has_no_input() {
        let tester = tester_with()
            .bat_1_auto()
            .bat_2_off()
            .and()
            .airspeed(Velocity::new::<knot>(49.))
            .run_waiting_for(Duration::from_secs(1_000));

        assert_eq!(tester.static_inverter_input(), Current::none());
    }

    #[test]
    fn bat_only_low_airspeed_when_both_battery_contactors_closed_static_inverter_has_input() {
        let tester = tester_with()
            .bat_1_auto()
            .bat_2_auto()
            .and()
            .airspeed(Velocity::new::<knot>(49.))
            .run_waiting_for(Duration::from_secs(1_000));

        assert_eq!(
            tester.static_inverter_input(),
            Current::some(ElectricPowerSource::Battery(10))
        );
    }

    #[test]
    fn when_airspeed_above_50_and_ac_bus_1_and_2_unpowered_and_emergency_gen_off_static_inverter_powers_ac_ess_bus(
    ) {
        let tester = tester_with()
            .airspeed(Velocity::new::<knot>(51.))
            .run_waiting_for(Duration::from_secs(1_000));

        assert_eq!(
            tester.static_inverter_input(),
            Current::some(ElectricPowerSource::Battery(10))
        );
        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::StaticInverter)
        );
    }

    /// # Source
    /// Discord (komp#1821):
    /// > The fault light will extinguish after 3 seconds. That's the time delay before automatic switching is activated in case of AC BUS 1 loss.
    #[test]
    fn with_ac_bus_1_being_unpowered_after_a_delay_ac_bus_2_powers_ac_ess_bus() {
        let tester = tester_with()
            .running_engine_2()
            .and()
            .bus_tie_off()
            .run_waiting_for_ac_ess_feed_transition();

        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
    }

    /// # Source
    /// Discord (komp#1821):
    /// > When AC BUS 1 is available again, it will switch back automatically without delay, unless the AC ESS FEED button is on ALTN.
    #[test]
    fn ac_bus_1_powers_ac_ess_bus_immediately_when_ac_bus_1_becomes_powered_after_ac_bus_2_was_powering_ac_ess_bus(
    ) {
        let tester = tester_with()
            .running_engine_2()
            .and()
            .bus_tie_off()
            .run_waiting_for_ac_ess_feed_transition()
            .then_continue_with()
            .running_engine_1()
            .and()
            .bus_tie_auto()
            .run();

        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
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
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
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
            tester.ac_bus_1_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
        assert_eq!(
            tester.ac_bus_2_output(),
            Current::some(ElectricPowerSource::EngineGenerator(1))
        );
    }

    #[test]
    fn when_ac_ess_feed_push_button_altn_engine_gen_2_powers_ac_ess_bus() {
        let tester = tester_with()
            .running_engines()
            .and()
            .ac_ess_feed_altn()
            .run();

        assert_eq!(
            tester.ac_ess_bus_output(),
            Current::some(ElectricPowerSource::EngineGenerator(2))
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
    fn when_ac_bus_1_and_ac_bus_2_are_lost_neither_ac_ess_feed_contactor_is_closed() {
        let tester = tester_with().run();

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
    fn when_battery_1_is_empty_and_dc_bat_bus_unpowered_hot_bus_1_unpowered() {
        let tester = tester_with().empty_battery_1().run();

        assert!(tester.hot_bus_1_output().is_unpowered());
    }

    #[test]
    fn when_battery_1_is_empty_and_dc_bat_bus_powered_hot_bus_1_powered() {
        let tester = tester_with()
            .running_engines()
            .and()
            .empty_battery_1()
            .run();

        assert_eq!(
            tester.hot_bus_1_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
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
    fn when_battery_2_is_empty_and_dc_bat_bus_unpowered_hot_bus_2_unpowered() {
        let tester = tester_with().empty_battery_2().run();

        assert!(tester.hot_bus_2_output().is_unpowered());
    }

    #[test]
    fn when_battery_2_is_empty_and_dc_bat_bus_powered_hot_bus_2_powered() {
        let tester = tester_with()
            .running_engines()
            .and()
            .empty_battery_2()
            .run();

        assert_eq!(
            tester.hot_bus_2_output(),
            Current::some(ElectricPowerSource::TransformerRectifier(1))
        );
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

    #[test]
    fn when_dc_bus_1_and_dc_bus_2_unpowered_dc_bus_2_to_dc_bat_remains_open() {
        let tester = tester().run();

        assert!(tester.dc_bus_2_tie_contactor_is_open());
    }

    #[test]
    fn when_ac_ess_bus_powered_ac_ess_feed_does_not_have_fault() {
        let tester = tester_with().running_engines().run();

        assert!(!tester.ac_ess_feed_has_fault());
    }

    #[test]
    fn when_ac_ess_bus_is_unpowered_ac_ess_feed_has_fault() {
        let tester = tester_with().airspeed(Velocity::new::<knot>(0.)).run();

        assert!(tester.ac_ess_feed_has_fault());
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
            self.elec.direct_current.battery_1 = Battery::empty(1);
            self
        }

        fn empty_battery_2(mut self) -> ElectricalCircuitTester {
            self.elec.direct_current.battery_2 = Battery::empty(2);
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

        fn failed_tr_1(mut self) -> ElectricalCircuitTester {
            self.elec.alternating_current.tr_1.fail();
            self
        }

        fn failed_tr_2(mut self) -> ElectricalCircuitTester {
            self.elec.alternating_current.tr_2.fail();
            self
        }

        fn running_emergency_generator(mut self) -> ElectricalCircuitTester {
            self.elec.alternating_current.emergency_gen.attempt_start();
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

        fn ext_pwr_on(mut self) -> ElectricalCircuitTester {
            self.overhead.ext_pwr.turn_on();
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

        fn bat_1_auto(mut self) -> ElectricalCircuitTester {
            self.overhead.bat_1.push_auto();
            self
        }

        fn bat_2_off(mut self) -> ElectricalCircuitTester {
            self.overhead.bat_2.push_off();
            self
        }

        fn bat_2_auto(mut self) -> ElectricalCircuitTester {
            self.overhead.bat_2.push_auto();
            self
        }

        fn bus_tie_auto(mut self) -> ElectricalCircuitTester {
            self.overhead.bus_tie.push_auto();
            self
        }

        fn bus_tie_off(mut self) -> ElectricalCircuitTester {
            self.overhead.bus_tie.push_off();
            self
        }

        fn ac_bus_1_output(&self) -> Current {
            self.elec.alternating_current.ac_bus_1.output()
        }

        fn ac_bus_2_output(&self) -> Current {
            self.elec.alternating_current.ac_bus_2.output()
        }

        fn ac_ess_bus_output(&self) -> Current {
            self.elec.alternating_current.ac_ess_bus.output()
        }

        fn ac_ess_shed_bus_output(&self) -> Current {
            self.elec.alternating_current.ac_ess_shed_bus.output()
        }

        fn ac_stat_inv_bus_output(&self) -> Current {
            self.elec.alternating_current.ac_stat_inv_bus.output()
        }

        fn static_inverter_input(&self) -> Current {
            self.elec.direct_current.static_inverter.get_input()
        }

        fn tr_1_input(&self) -> Current {
            self.elec.alternating_current.tr_1.get_input()
        }

        fn tr_2_input(&self) -> Current {
            self.elec.alternating_current.tr_2.get_input()
        }

        fn tr_ess_input(&self) -> Current {
            self.elec.alternating_current.tr_ess.get_input()
        }

        fn dc_bus_1_output(&self) -> Current {
            self.elec.direct_current.dc_bus_1.output()
        }

        fn dc_bus_2_output(&self) -> Current {
            self.elec.direct_current.dc_bus_2.output()
        }

        fn dc_bat_bus_output(&self) -> Current {
            self.elec.direct_current.dc_bat_bus.output()
        }

        fn dc_ess_bus_output(&self) -> Current {
            self.elec.direct_current.dc_ess_bus.output()
        }

        fn dc_ess_shed_bus_output(&self) -> Current {
            self.elec.direct_current.dc_ess_shed_bus.output()
        }

        fn battery_1_input(&self) -> Current {
            self.elec.direct_current.battery_1.get_input()
        }

        fn battery_2_input(&self) -> Current {
            self.elec.direct_current.battery_2.get_input()
        }

        fn hot_bus_1_output(&self) -> Current {
            self.elec.direct_current.hot_bus_1.output()
        }

        fn hot_bus_2_output(&self) -> Current {
            self.elec.direct_current.hot_bus_2.output()
        }

        fn ac_ess_feed_has_fault(&self) -> bool {
            self.overhead.ac_ess_feed_has_fault()
        }

        fn create_power_supply(&self) -> PowerSupply {
            self.elec.create_power_supply()
        }

        fn both_ac_ess_feed_contactors_open(&self) -> bool {
            self.elec
                .alternating_current
                .ac_ess_feed_contactors
                .ac_ess_feed_contactor_1
                .is_open()
                && self
                    .elec
                    .alternating_current
                    .ac_ess_feed_contactors
                    .ac_ess_feed_contactor_2
                    .is_open()
        }

        fn dc_bus_2_tie_contactor_is_open(&self) -> bool {
            self.elec.direct_current.dc_bus_2_tie_contactor.is_open()
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
            self.overhead.update_after_elec(&self.elec);

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
            self.run_waiting_for(A320AcEssFeedContactors::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS)
        }

        fn run_waiting_until_just_before_ac_ess_feed_transition(self) -> ElectricalCircuitTester {
            self.run_waiting_for(
                A320AcEssFeedContactors::AC_ESS_FEED_TO_AC_BUS_2_DELAY_IN_SECONDS
                    - Duration::from_millis(1),
            )
        }

        fn new_running_engine() -> Engine {
            let mut engine = Engine::new(1);
            engine.n2 = Ratio::new::<percent>(80.);

            engine
        }

        fn new_stopped_engine() -> Engine {
            let mut engine = Engine::new(1);
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
