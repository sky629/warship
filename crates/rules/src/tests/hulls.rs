//! 선체의 고유한 수치와, 선체에 따라 쓰는 스킬을 확인합니다.
//! 선체는 받는 피해를 줄이거나, 스스로 수리하거나, 교역을 할 수 있습니다. 교역은 교역선만 합니다.

use super::support::{BIG_HULL, RALLY, SNARE, Scenario, TRADE, assert_close};
use crate::command::{Command, Refusal};
use crate::ids::{HullId, Team};
use crate::structures::StructureKind;

#[test]
fn an_armored_hull_takes_less_damage_and_a_crewed_hull_repairs_itself() {
    let mut s = Scenario::with_tuning(|tuning| {
        tuning.hulls[0].armor_percent = 30;
        tuning.hulls[0].repair_per_sec = 5;
    });
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    let tuning = s.sim.tuning().clone();

    assert_eq!(s.ship(ship).braced(&tuning, 100, s.sim.tick()), 70);

    s.set_ship_hp(ship, 500);
    s.run_ticks(20);
    assert_eq!(s.ship(ship).hp, 505);
}

#[test]
fn a_rally_repairs_the_ship_and_the_friends_near_it() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    let near = s.ship_at(Team::RED, 13.0, 10.0);
    let far = s.ship_at(Team::RED, 10.0, 15.0);
    let enemy = s.ship_at(Team::BLUE, 10.0, 12.0);
    for id in [ship, near, far, enemy] {
        s.set_ship_hp(id, 500);
    }
    s.command(ship, Command::Learn(RALLY));

    assert_eq!(s.command(ship, Command::Cast(RALLY)), None);

    // 4타일 안의 같은 진영의 배가 단계마다 60을 회복합니다.
    assert_eq!(s.ship(ship).hp, 560);
    assert_eq!(s.ship(near).hp, 560);
    assert_eq!(s.ship(far).hp, 500);
    assert_eq!(s.ship(enemy).hp, 500);

    s.set_ship_hp(near, 990);
    s.run_ticks(100);
    s.command(ship, Command::Cast(RALLY));
    assert_eq!(s.ship(near).hp, 1000, "a repair stops at the highest HP");
}

#[test]
fn a_snare_slows_the_enemy_ships_near_it_for_its_duration() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    let friend = s.ship_at(Team::RED, 10.0, 12.0);
    let enemy = s.ship_at(Team::BLUE, 12.0, 10.0);
    let far = s.ship_at(Team::BLUE, 10.0, 16.0);
    s.command(ship, Command::Learn(SNARE));
    for (id, y) in [(friend, 12.0), (enemy, 10.0), (far, 16.0)] {
        s.sail_to(id, 39.0, y);
    }

    assert_eq!(s.command(ship, Command::Cast(SNARE)), None);
    s.run_ticks(20);

    // 4타일 안의 적은 20Tick 동안 속도가 단계마다 40% 줄어서 1초에 2.4타일을 갑니다.
    assert_close(s.ship(enemy).pos.x, 14.4);
    assert_close(s.ship(friend).pos.x, 14.0);
    assert_close(s.ship(far).pos.x, 14.0);

    s.run_ticks(20);
    assert_close(s.ship(enemy).pos.x, 18.4);
}

#[test]
fn only_a_trader_hull_trades() {
    let mut s = Scenario::open_sea();
    s.structure_at(StructureKind::Base, Team::RED, 10.0, 10.0);
    let ship = s.ship_at(Team::RED, 11.0, 10.0);
    s.set_gold(ship, 2000);

    assert_eq!(s.command(ship, Command::Contract(TRADE)), None);
    assert_eq!(
        s.command(ship, Command::ChangeHull(BIG_HULL)),
        Some(Refusal::CarryingFreight),
        "a warship has no hold for the contract"
    );

    s.command(ship, Command::Sell { slot: 0 });
    assert_eq!(s.command(ship, Command::ChangeHull(BIG_HULL)), None);
    assert_eq!(
        s.command(ship, Command::Contract(TRADE)),
        Some(Refusal::NotATrader)
    );
    assert_eq!(s.command(ship, Command::ChangeHull(HullId(0))), None);
    assert_eq!(s.command(ship, Command::Contract(TRADE)), None);
}
