//! 라인의 NPC 배를 확인합니다. NPC 배는 HP 100, 1초에 2타일이고, 포는 사거리 5, 피해 10, 쿨다운 10Tick입니다.
//!
//! 시험용 라인은 (10, 20)에서 (30, 20)까지이고, 두 진영의 항구가 그 양 끝에 있습니다.

use super::support::{LONG_GUN, Scenario, assert_close};
use crate::ids::Team;
use crate::math::Vec2;
use crate::structures::StructureKind;
use crate::tuning::{StructureSite, Tuning, Wave};
use crate::weapon::Target;

fn lane(t: &mut Tuning) {
    t.map.lanes = vec![vec![Vec2::new(10.0, 20.0), Vec2::new(30.0, 20.0)]];
}

/// 라인 하나와 양 끝의 항구가 있고, 10번째 Tick부터 100Tick마다 NPC 배 두 척이 5Tick 간격으로 나오는 수치입니다.
fn lane_with_harbors(t: &mut Tuning) {
    lane(t);
    for (team, x) in [(Team::RED, 10.0), (Team::BLUE, 30.0)] {
        t.map.structures.push(StructureSite {
            kind: StructureKind::Harbor,
            team,
            pos: Vec2::new(x, 20.0),
            lane: Some(0),
        });
    }
    t.waves = vec![Wave {
        creep: 0,
        count: 2,
        first_tick: 10,
        period_ticks: 100,
        gap_ticks: 5,
    }];
}

#[test]
fn harbors_send_out_creeps_on_the_schedule() {
    let mut s = Scenario::with_tuning(lane_with_harbors);

    s.run_ticks(9);
    assert_eq!(s.sim.creeps().count(), 0);
    s.run_ticks(1);
    assert_eq!(s.sim.creeps().count(), 2, "one from each harbor");
    s.run_ticks(5);
    assert_eq!(s.sim.creeps().count(), 4, "the second of the wave");
    s.run_ticks(94);
    assert_eq!(s.sim.creeps().count(), 4);
    s.run_ticks(1);
    assert_eq!(s.sim.creeps().count(), 6, "the next wave");
}

#[test]
fn a_creep_sails_along_its_lane_toward_the_enemy() {
    let mut s = Scenario::with_tuning(lane);
    let red = s.creep_at(Team::RED, 12.0, 20.0);
    let blue = s.creep_at(Team::BLUE, 28.0, 20.0);

    // NPC 배는 먼저 자기 쪽 끝의 지점으로 갔다가 상대 쪽으로 갑니다. 1초에 2타일입니다.
    s.run_ticks(20);
    assert_close(s.sim.creep(red).unwrap().pos.x, 10.0);
    assert_close(s.sim.creep(blue).unwrap().pos.x, 30.0);
    s.run_ticks(20);
    assert_close(s.sim.creep(red).unwrap().pos.x, 12.0);
    assert_close(s.sim.creep(blue).unwrap().pos.x, 28.0);
}

#[test]
fn a_creep_stops_to_fight_an_enemy_in_range_and_goes_on_after_it_sinks() {
    let mut s = Scenario::with_tuning(lane);
    let creep = s.creep_at(Team::RED, 10.0, 20.0);
    let enemy = s.ship_at(Team::BLUE, 16.0, 20.0);
    s.set_ship_hp(enemy, 20);

    // 적이 사거리 5 안에 들어올 때까지 나아가고, 그 자리에서 멈춰 쏩니다.
    s.run_ticks(14);
    let stopped_at = s.sim.creep(creep).unwrap().pos.x;
    assert!((11.0..11.2).contains(&stopped_at), "got {stopped_at}");
    s.run_ticks(4);
    assert_eq!(s.sim.creep(creep).unwrap().pos.x, stopped_at);

    // 포탄 두 발에 적이 가라앉으면 다시 나아갑니다.
    s.run_ticks(40);
    assert!(!s.ship(enemy).is_afloat());
    assert!(s.sim.creep(creep).unwrap().pos.x > stopped_at + 1.5);
}

#[test]
fn a_creep_is_a_target_of_a_ships_only_gun_and_is_removed_when_it_sinks() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let creep = s.creep_at(Team::BLUE, 12.0, 10.0);
    s.arm(a, LONG_GUN);

    s.run_ticks(3);
    assert_eq!(s.sim.creep(creep).unwrap().hp, 90);
    assert_eq!(
        s.ship(a).mounts[0].unwrap().target,
        Some(Target::Creep(creep))
    );

    s.run_ticks(100);
    assert!(s.sim.creep(creep).is_none());
}

#[test]
fn a_lane_whose_harbor_is_destroyed_gets_no_more_creeps_from_that_team() {
    let mut s = Scenario::with_tuning(lane_with_harbors);
    let red_harbor = s.sim.structures().find(|site| site.team == Team::RED);
    let red_harbor = red_harbor.unwrap().id;
    s.sim.world_mut().structures.remove(&red_harbor);

    s.run_ticks(10);

    let teams: Vec<Team> = s.sim.creeps().map(|creep| creep.team).collect();
    assert_eq!(teams, [Team::BLUE]);
}
