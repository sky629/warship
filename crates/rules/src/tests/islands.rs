//! 섬과 길 찾기를 확인합니다. 맵의 (20, 20)에 반지름 4인 섬이 있습니다. 배는 1초(20Tick)에 4타일을 갑니다.

use super::support::{Scenario, assert_close};
use crate::ids::{ShipId, Team};
use crate::math::Vec2;
use crate::sim::Sim;
use crate::tuning::{Island, Tuning};

const CENTER: Vec2 = Vec2::new(20.0, 20.0);
const RADIUS: f32 = 4.0;

fn one_island() -> Scenario {
    Scenario::with_tuning(|tuning| {
        tuning.map.islands.push(Island {
            center: CENTER,
            radius: RADIUS,
        });
    })
}

/// 배를 `ticks`만큼 움직이면서 섬의 한가운데에 가장 가까웠던 거리를 돌려줍니다.
fn nearest_approach(s: &mut Scenario, ship: ShipId, ticks: u32) -> f32 {
    let mut nearest = f32::INFINITY;
    for _ in 0..ticks {
        s.run_ticks(1);
        nearest = nearest.min(s.ship(ship).pos.distance(CENTER));
    }
    nearest
}

#[test]
fn a_ship_sails_around_an_island_to_the_far_side() {
    let mut s = one_island();
    let a = s.ship_at(Team::RED, 10.0, 20.0);
    s.sail_to(a, 30.0, 20.0);

    // 곧은 길은 20타일이고 100Tick이 걸립니다. 돌아서 가는 길은 그보다 길지만 140Tick 안에 닿습니다.
    let nearest = nearest_approach(&mut s, a, 140);

    assert!(nearest >= RADIUS, "the ship crossed the island: {nearest}");
    assert_eq!(s.ship(a).pos, Vec2::new(30.0, 20.0));
}

#[test]
fn the_detour_takes_longer_than_the_straight_line() {
    let mut s = one_island();
    let a = s.ship_at(Team::RED, 10.0, 20.0);
    s.sail_to(a, 30.0, 20.0);

    s.run_ticks(100);

    assert_ne!(s.ship(a).pos, Vec2::new(30.0, 20.0));
}

#[test]
fn a_ship_keeps_its_speed_through_the_corners_of_a_detour() {
    let mut s = one_island();
    let a = s.ship_at(Team::RED, 10.0, 20.0);
    s.sail_to(a, 30.0, 20.0);

    let mut before = s.ship(a).pos;
    for _ in 0..80 {
        s.run_ticks(1);
        let after = s.ship(a).pos;
        // 꺾이는 지점을 지나는 Tick에는 두 선분을 따라 0.2타일을 가므로, 직선거리는 0.2보다 조금 짧을 수 있습니다.
        let moved = before.distance(after);
        assert!(moved > 0.15 && moved < 0.2 + 1e-3, "moved {moved}");
        before = after;
    }
}

#[test]
fn a_path_that_only_passes_beside_an_island_stays_straight() {
    let mut s = one_island();
    let a = s.ship_at(Team::RED, 10.0, 26.0);
    s.sail_to(a, 30.0, 26.0);

    s.run_ticks(50);

    assert_close(s.ship(a).pos.x, 20.0);
    assert_close(s.ship(a).pos.y, 26.0);
}

#[test]
fn a_destination_on_an_island_is_moved_to_its_shore() {
    let mut s = one_island();
    let a = s.ship_at(Team::RED, 10.0, 20.0);
    s.sail_to(a, 18.0, 20.0);

    s.run_ticks(60);

    // 섬의 서쪽 가장자리(x = 16)에서 0.25타일 떨어진 점입니다.
    assert_close(s.ship(a).pos.x, 15.75);
    assert_close(s.ship(a).pos.y, 20.0);
}

#[test]
fn a_ship_on_the_shore_can_leave_and_can_reach_the_far_shore() {
    let mut s = one_island();
    let a = s.ship_at(Team::RED, 10.0, 20.0);
    s.sail_to(a, 20.0, 20.0);
    s.run_ticks(60);

    s.sail_to(a, 23.0, 20.0);
    let nearest = nearest_approach(&mut s, a, 120);

    assert!(nearest >= RADIUS, "the ship crossed the island: {nearest}");
    assert_close(s.ship(a).pos.x, 24.25);
    assert_close(s.ship(a).pos.y, 20.0);
}

#[test]
fn ships_cross_the_default_map_without_touching_an_island() {
    let tuning = Tuning::default();
    let islands = tuning.map.islands.clone();
    let mut s = Scenario::open_sea();
    s.sim = Sim::new(tuning);
    let red = s.sim.add_ship(Team::RED);
    let blue = s.sim.add_ship(Team::BLUE);
    // 두 배는 맵의 반대쪽으로 비스듬히 갑니다. 곧은 길은 라인 사이의 가운데 섬을 지납니다.
    s.sail_to(red, 87.0, 46.0);
    s.sail_to(blue, 9.0, 18.0);

    for _ in 0..600 {
        s.run_ticks(1);
        for ship in [red, blue] {
            let pos = s.ship(ship).pos;
            for island in &islands {
                assert!(pos.distance(island.center) >= island.radius);
            }
        }
    }

    assert_eq!(s.ship(red).pos, Vec2::new(87.0, 46.0));
    assert_eq!(s.ship(blue).pos, Vec2::new(9.0, 18.0));
}
