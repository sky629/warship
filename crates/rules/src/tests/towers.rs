//! 타워의 사격, 파괴, 시야를 확인합니다. 타워의 포는 사거리 6, 피해 15, 쿨다운 10Tick이고 배만 쏩니다.

use super::support::{GUN, LONG_GUN, Scenario};
use crate::ids::Team;
use crate::weapon::Target;

#[test]
fn a_tower_shoots_the_nearest_enemy_ship_in_range() {
    let mut s = Scenario::open_sea();
    let tower = s.tower_at(Team::RED, 10.0, 10.0);
    let near = s.ship_at(Team::BLUE, 13.0, 10.0);
    let far = s.ship_at(Team::BLUE, 15.0, 10.0);
    let ally = s.ship_at(Team::RED, 11.0, 10.0);

    s.run_ticks(4);

    assert_eq!(s.ship(near).hp, 985);
    assert_eq!(s.ship(far).hp, 1000);
    assert_eq!(s.ship(ally).hp, 1000);
    assert_eq!(s.sim.tower(tower).unwrap().target, Some(Target::Ship(near)));
}

#[test]
fn a_tower_at_zero_hp_is_removed() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let tower = s.tower_at(Team::BLUE, 12.0, 10.0);
    s.arm(a, GUN);
    s.set_tower_hp(tower, 10);

    s.run_ticks(3);

    assert!(s.sim.tower(tower).is_none());
    assert_eq!(s.sim.towers().count(), 0);
}

#[test]
fn a_tower_lets_its_team_see() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let b = s.ship_at(Team::BLUE, 21.0, 10.0);
    s.arm(a, LONG_GUN);
    // 적에게서 8 떨어진 아군 타워입니다. 타워의 시야(9) 안이고 타워의 사거리(6) 밖입니다.
    s.tower_at(Team::RED, 21.0, 18.0);

    s.run_ticks(12);

    assert_eq!(s.ship(b).hp, 990);
}
