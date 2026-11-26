//! 골드를 확인합니다. 수입은 1초(20Tick)에 2골드이고, 보상은 배 200, NPC 배 20, 타워 300입니다.

use super::support::{GUN, Scenario};
use crate::ids::Team;

#[test]
fn every_ship_earns_the_income_once_a_second() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);

    s.run_ticks(19);
    assert_eq!(s.ship(a).gold, 0);
    s.run_ticks(1);
    assert_eq!(s.ship(a).gold, 2);
    s.run_ticks(40);
    assert_eq!(s.ship(a).gold, 6);
}

#[test]
fn the_ship_whose_shell_sinks_an_enemy_gets_the_bounty() {
    let mut s = Scenario::open_sea();
    let killer = s.ship_at(Team::RED, 10.0, 10.0);
    let helper = s.ship_at(Team::RED, 10.0, 11.0);
    let victim = s.ship_at(Team::BLUE, 11.0, 10.0);
    s.arm(killer, GUN);
    s.set_ship_hp(victim, 10);

    s.run_ticks(2);

    assert!(!s.ship(victim).is_afloat());
    assert_eq!(s.ship(killer).gold, 200);
    assert_eq!(s.ship(helper).gold, 0, "only the ship that fired is paid");
}

#[test]
fn sinking_a_creep_and_destroying_a_structure_pay_their_bounties() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let tower = s.tower_at(Team::BLUE, 13.0, 10.0);
    s.arm(a, GUN);
    s.set_structure_hp(tower, 10);
    s.run_ticks(4);
    assert!(s.sim.structure(tower).is_none());
    assert_eq!(s.ship(a).gold, 300);

    // NPC 배(HP 100)는 포탄 열 발에 가라앉습니다. 104번째 Tick까지 5초의 수입(10골드)이 들어옵니다.
    let creep = s.creep_at(Team::BLUE, 9.0, 10.0);
    s.run_ticks(100);
    assert!(s.sim.creep(creep).is_none());
    assert_eq!(s.ship(a).gold, 300 + 20 + 10);
}

#[test]
fn a_kill_by_a_tower_pays_nobody() {
    let mut s = Scenario::open_sea();
    s.tower_at(Team::RED, 10.0, 10.0);
    let bystander = s.ship_at(Team::RED, 10.0, 11.0);
    let victim = s.ship_at(Team::BLUE, 12.0, 10.0);
    s.set_ship_hp(victim, 15);

    s.run_ticks(3);

    assert!(!s.ship(victim).is_afloat());
    assert_eq!(s.ship(bystander).gold, 0);
}
