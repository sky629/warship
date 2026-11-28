//! 배의 이동을 확인합니다. 배는 1초(20Tick)에 4타일을 갑니다.

use super::support::{Scenario, assert_close};
use crate::ids::Team;
use crate::input::Waypoint;
use crate::math::Vec2;

#[test]
fn a_ship_sails_toward_its_destination_at_its_speed() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    s.sail_to(a, 20.0, 10.0);

    s.run_ticks(20);

    assert_close(s.ship(a).pos.x, 14.0);
    assert_close(s.ship(a).pos.y, 10.0);
}

#[test]
fn a_ship_stops_exactly_at_its_destination() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    s.sail_to(a, 11.0, 13.0);

    s.run_ticks(60);

    assert_eq!(s.ship(a).pos, Vec2::new(11.0, 13.0));
}

#[test]
fn a_ship_without_an_order_stays_where_it_is() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);

    s.run_ticks(20);

    assert_eq!(s.ship(a).pos, Vec2::new(10.0, 10.0));
}

#[test]
fn a_ship_keeps_its_destination_on_ticks_without_input_and_stops_when_told() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    s.sail_to(a, 20.0, 10.0);
    s.run_ticks(1);

    let no_input = Default::default();
    for _ in 0..9 {
        s.sim.step(&no_input);
    }
    assert_close(s.ship(a).pos.x, 12.0);

    s.stop(a);
    s.run_ticks(10);
    assert_close(s.ship(a).pos.x, 12.0);
}

#[test]
fn a_destination_outside_the_map_is_moved_to_the_edge() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 2.0, 10.0);
    s.sail_to(a, -50.0, 10.0);

    s.run_ticks(40);

    assert_eq!(s.ship(a).pos, Vec2::new(0.0, 10.0));
}

#[test]
fn a_waypoint_that_is_not_finite_is_refused() {
    assert!(Waypoint::new(f32::NAN, 1.0).is_none());
    assert!(Waypoint::new(1.0, f32::INFINITY).is_none());
    assert!(Waypoint::new(1.0, 2.0).is_some());
}

#[test]
fn a_sunk_ship_does_not_move() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    s.sim
        .world_mut()
        .damage(crate::weapon::Target::Ship(a), 1000, u64::MAX);
    s.sail_to(a, 20.0, 10.0);

    s.run_ticks(20);

    assert!(!s.ship(a).is_afloat());
    assert_eq!(s.ship(a).pos, Vec2::new(10.0, 10.0));
}

#[test]
fn a_ship_sent_onto_a_structure_stops_half_its_length_from_the_edge_on_the_near_side() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let tower = s.tower_at(Team::RED, 20.0, 10.0);
    s.sail_to(a, 20.0, 10.0);

    s.run_ticks(100);

    // 타워의 둘레는 반지름 1이고, 배의 길이는 2입니다.
    assert_close(s.ship(a).pos.x, 18.0);
    assert_close(s.ship(a).pos.y, 10.0);

    // 파괴된 건물의 자리에는 설 수 있습니다.
    s.set_structure_hp(tower, 0);
    s.run_ticks(1);
    s.sail_to(a, 20.0, 10.0);
    s.run_ticks(20);
    assert_close(s.ship(a).pos.x, 20.0);
}
