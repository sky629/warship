//! 무기마다 다른 공격 방식을 확인합니다.
//!
//! `SCATTER_GUN`은 표적에 닿을 때 표적의 둘레 2타일 안의 적을 함께 맞힙니다.
//! `BOMB`은 쏠 때 표적이 있던 자리로 느리게(1초에 5타일) 날아가서, 그 자리의 둘레 2타일 안의 적을 맞힙니다.
//! `FIRE_GUN`은 맞힌 뒤 2초 동안 1초마다 피해 5를 더 줍니다. 쿨다운이 200Tick이어서 테스트에서는 한 번만 쏩니다.
//! 세 무기 모두 사거리 5, 피해 10입니다.

use super::support::{BOMB, FIRE_GUN, GUN, SCATTER_GUN, Scenario};
use crate::ids::Team;
use crate::math::Vec2;

#[test]
fn a_scatter_gun_hits_the_target_and_the_enemies_beside_it() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let target = s.ship_at(Team::BLUE, 14.0, 10.0);
    let beside = s.ship_at(Team::BLUE, 15.5, 10.0);
    let apart = s.ship_at(Team::BLUE, 14.0, 12.5);
    let friend = s.ship_at(Team::RED, 14.0, 9.0);
    s.arm(a, SCATTER_GUN);

    s.run_ticks(5);

    assert_eq!(s.ship(target).hp, 990);
    assert_eq!(s.ship(beside).hp, 990);
    assert_eq!(s.ship(apart).hp, 1000, "2.5 tiles from the target");
    assert_eq!(s.ship(friend).hp, 1000);
}

#[test]
fn a_bomb_lands_where_the_target_was_and_misses_a_ship_that_sailed_away() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let runner = s.ship_at(Team::BLUE, 14.0, 10.0);
    let still = s.ship_at(Team::BLUE, 14.5, 11.0);
    s.arm(a, BOMB);
    s.sail_to(runner, 14.0, 30.0);

    s.run_ticks(1);
    let mount = s.ship(a).mounts[0].expect("the bomb is mounted");
    // 배가 움직인 뒤에 쏘므로, 달아나는 배가 한 Tick 간 자리입니다.
    assert_eq!(mount.aim, Some(Vec2::new(14.0, 10.2)));

    // 4타일보다 조금 먼 거리를 17Tick 동안 날아서 18번째 Tick에 떨어집니다. 그동안 달아난 배는 3타일 넘게 갔습니다.
    s.run_ticks(16);
    assert_eq!(s.ship(still).hp, 1000);
    s.run_ticks(1);
    assert_eq!(s.ship(still).hp, 990);
    assert_eq!(s.ship(runner).hp, 1000);
}

#[test]
fn a_fire_gun_keeps_burning_its_target_for_a_while() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let b = s.ship_at(Team::BLUE, 14.0, 10.0);
    s.arm(a, FIRE_GUN);

    s.run_ticks(5);
    assert_eq!(s.ship(b).hp, 990);
    s.run_ticks(20);
    assert_eq!(s.ship(b).hp, 985);
    s.run_ticks(20);
    assert_eq!(s.ship(b).hp, 980);
    s.run_ticks(40);
    assert_eq!(s.ship(b).hp, 980, "the fire is out");
}

#[test]
fn the_ship_that_fired_gets_the_bounty_of_an_enemy_sunk_by_its_blast() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    s.ship_at(Team::BLUE, 14.0, 10.0);
    let beside = s.ship_at(Team::BLUE, 15.0, 10.0);
    s.arm(a, SCATTER_GUN);
    s.arm(beside, GUN);
    s.set_ship_hp(beside, 5);
    let gold = s.ship(a).gold;

    s.run_ticks(5);

    assert!(!s.ship(beside).is_afloat());
    assert_eq!(s.ship(a).gold, gold + 200);
}
