//! 은신 스킬을 확인합니다. 은신한 배는 적의 배와 NPC 배에게 보이지 않고, 적의 건물에게는 보입니다.
//! 무기를 쏘거나 다른 스킬을 쓰면 은신이 풀립니다.

use super::support::{CLOAK, GUN, REPAIR, Scenario};
use crate::command::Command;
use crate::ids::Team;
use crate::weapon::Target;

#[test]
fn a_cloaked_ship_is_hidden_from_enemy_ships_for_its_duration() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    let enemy = s.ship_at(Team::BLUE, 14.0, 10.0);
    s.command(ship, Command::Learn(CLOAK));
    s.run_ticks(1);
    assert!(s.sim.sees(Team::BLUE, Target::Ship(ship)));

    assert_eq!(s.command(ship, Command::Cast(CLOAK)), None);
    s.run_ticks(1);
    assert!(!s.sim.sees(Team::BLUE, Target::Ship(ship)));
    assert!(s.sim.sees(Team::RED, Target::Ship(enemy)), "it still sees");
    assert!(s.ship(ship).cloaked(s.sim.tick()));

    // 은신은 단계마다 40Tick 동안 이어집니다.
    s.run_ticks(38);
    assert!(!s.sim.sees(Team::BLUE, Target::Ship(ship)));
    s.run_ticks(2);
    assert!(s.sim.sees(Team::BLUE, Target::Ship(ship)));
}

#[test]
fn an_enemy_structure_sees_a_cloaked_ship_and_a_creep_does_not() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    s.creep_at(Team::BLUE, 14.0, 10.0);
    s.command(ship, Command::Learn(CLOAK));
    s.command(ship, Command::Cast(CLOAK));
    s.run_ticks(1);
    assert!(!s.sim.sees(Team::BLUE, Target::Ship(ship)));

    s.tower_at(Team::BLUE, 10.0, 16.0);
    s.run_ticks(1);
    assert!(s.sim.sees(Team::BLUE, Target::Ship(ship)));
}

#[test]
fn firing_a_gun_or_casting_another_skill_ends_the_cloak() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    s.set_xp(ship, 100);
    s.command(ship, Command::Learn(CLOAK));
    s.command(ship, Command::Learn(REPAIR));
    s.command(ship, Command::Cast(CLOAK));
    s.run_ticks(1);
    assert!(s.ship(ship).cloaked(s.sim.tick()));

    s.command(ship, Command::Cast(REPAIR));
    assert!(!s.ship(ship).cloaked(s.sim.tick()));

    // 쿨다운(100Tick)이 지난 뒤에 다시 은신하고, 적이 사거리에 들어오면 포가 쏘면서 은신이 풀립니다.
    s.run_ticks(100);
    s.arm(ship, GUN);
    s.command(ship, Command::Cast(CLOAK));
    s.run_ticks(1);
    assert!(s.ship(ship).cloaked(s.sim.tick()));
    s.ship_at(Team::BLUE, 13.0, 10.0);
    s.run_ticks(2);
    assert!(!s.ship(ship).cloaked(s.sim.tick()));
    assert!(s.sim.sees(Team::BLUE, Target::Ship(ship)));
}
