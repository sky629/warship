//! 자동 사격을 확인합니다.
//!
//! `GUN`은 사거리 5, 피해 10, 쿨다운 10Tick이고, 포탄은 한 Tick에 1타일을 납니다.
//! `LONG_GUN`은 사거리가 12이고 배만 쏩니다. 배의 HP는 1000이고 시야는 9입니다.

use super::support::{GUN, LONG_GUN, Scenario};
use crate::ids::Team;

use crate::weapon::Target;

#[test]
fn a_gun_fires_by_itself_and_the_damage_lands_after_the_flight_time() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let b = s.ship_at(Team::BLUE, 14.0, 10.0);
    s.arm(a, GUN);

    // 첫 Tick에 쏘고, 4타일을 4Tick 동안 날아서 5번째 Tick에 닿습니다.
    s.run_ticks(4);
    assert_eq!(s.ship(b).hp, 1000);
    s.run_ticks(1);
    assert_eq!(s.ship(b).hp, 990);
}

#[test]
fn a_gun_reaches_an_enemy_at_exactly_its_range_and_not_beyond() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let at_range = s.ship_at(Team::BLUE, 10.0, 15.0);
    s.arm(a, GUN);
    s.run_ticks(6);
    assert_eq!(s.ship(at_range).hp, 990);

    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let beyond = s.ship_at(Team::BLUE, 10.0, 15.5);
    s.arm(a, GUN);
    s.run_ticks(40);
    assert_eq!(s.ship(beyond).hp, 1000);
    assert_eq!(s.ship(a).mounts[0].unwrap().target, None);
}

#[test]
fn a_gun_waits_for_its_cooldown_between_shots() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let b = s.ship_at(Team::BLUE, 11.0, 10.0);
    s.arm(a, GUN);

    // 1, 11, 21번째 Tick에 쏘고, 포탄은 다음 Tick에 닿습니다.
    s.run_ticks(21);
    assert_eq!(s.ship(b).hp, 980);
    s.run_ticks(1);
    assert_eq!(s.ship(b).hp, 970);
}

#[test]
fn every_mount_fires_on_its_own() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let b = s.ship_at(Team::BLUE, 11.0, 10.0);
    s.arm(a, GUN);
    s.arm(a, GUN);

    s.run_ticks(2);

    assert_eq!(s.ship(b).hp, 980);
}

#[test]
fn a_gun_does_not_fire_at_an_ally() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let ally = s.ship_at(Team::RED, 11.0, 10.0);
    s.arm(a, GUN);

    s.run_ticks(30);

    assert_eq!(s.ship(ally).hp, 1000);
}

#[test]
fn a_gun_fires_at_the_nearest_enemy_and_shows_its_target() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let far = s.ship_at(Team::BLUE, 14.0, 10.0);
    let near = s.ship_at(Team::BLUE, 10.0, 12.0);
    s.arm(a, GUN);

    s.run_ticks(6);

    assert_eq!(s.ship(near).hp, 990);
    assert_eq!(s.ship(far).hp, 1000);
    assert_eq!(
        s.ship(a).mounts[0].unwrap().target,
        Some(Target::Ship(near))
    );
}

#[test]
fn a_gun_does_not_fire_at_an_enemy_in_range_that_its_team_cannot_see() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let b = s.ship_at(Team::BLUE, 21.0, 10.0);
    s.arm(a, LONG_GUN);

    // 거리 11은 사거리 12 안이지만 시야 9 밖입니다.
    s.run_ticks(40);
    assert_eq!(s.ship(b).hp, 1000);

    // 아군이 적을 보면 쏩니다.
    s.ship_at(Team::RED, 21.0, 18.0);
    s.run_ticks(12);
    assert_eq!(s.ship(b).hp, 990);
}

#[test]
fn a_ships_only_gun_leaves_towers_alone() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    // 타워의 포(사거리 6)가 닿지 않는 거리입니다.
    let tower = s.tower_at(Team::BLUE, 17.0, 10.0);
    s.arm(a, LONG_GUN);

    s.run_ticks(40);

    assert_eq!(s.sim.structure(tower).unwrap().hp, 1000);
}

#[test]
fn a_shell_follows_a_target_that_leaves_the_range() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let b = s.ship_at(Team::BLUE, 14.9, 10.0);
    s.arm(a, GUN);

    // 첫 Tick에 한 번 쏜 뒤 적이 사거리 밖으로 달아납니다. 다음 사격(11번째 Tick)에는 적이 사거리 밖에 있습니다.
    s.run_ticks(1);
    s.sail_to(b, 30.0, 10.0);
    s.run_ticks(19);

    assert_eq!(s.ship(b).hp, 990, "only the shell that was fired lands");
    assert!(s.ship(b).pos.x > 15.0);
}

#[test]
fn a_ship_sinks_at_zero_hp_and_takes_no_part_in_the_fight_after_that() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let b = s.ship_at(Team::BLUE, 11.0, 10.0);
    s.arm(a, GUN);
    s.arm(b, GUN);
    s.set_ship_hp(b, 10);

    // 두 배가 첫 Tick에 서로 쏘고, 두 번째 Tick에 포탄이 닿습니다.
    s.run_ticks(2);
    assert!(!s.ship(b).is_afloat());
    assert_eq!(s.ship(b).hp, 0);
    assert_eq!(s.ship(a).hp, 990, "the shell fired before sinking lands");

    s.run_ticks(30);
    assert_eq!(s.ship(a).hp, 990, "a sunk ship does not fire");
    assert_eq!(
        s.ship(a).mounts[0].unwrap().target,
        None,
        "a sunk ship is not a target"
    );
}
