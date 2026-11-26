//! 한 판의 진행을 확인합니다. 가라앉은 배는 100Tick 뒤에 다시 나타나고, 본진을 잃은 진영이 집니다.

use super::support::{GUN, Scenario};
use crate::ids::Team;
use crate::math::Vec2;
use crate::sim::{Outcome, Sim};
use crate::structures::StructureKind;
use crate::tuning::Tuning;

#[test]
fn a_sunk_ship_comes_back_at_its_spawn_with_full_hp_and_its_gold() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let b = s.ship_at(Team::BLUE, 11.0, 10.0);
    s.arm(a, GUN);
    s.arm(b, GUN);
    s.set_ship_hp(b, 10);

    // 두 번째 Tick에 가라앉고, 그로부터 100Tick 뒤인 102번째 Tick에 다시 나타납니다.
    s.run_ticks(101);
    assert!(!s.ship(b).is_afloat());
    s.run_ticks(1);

    let back = s.ship(b);
    assert!(back.is_afloat());
    assert_eq!(back.hp, 1000);
    assert_eq!(back.pos, Vec2::new(39.0, 39.0), "the blue spawn");
    assert_eq!(back.gold, 10, "five seconds of income");
    assert!(back.mounts[0].is_some(), "it keeps its weapons");
}

#[test]
fn the_team_that_loses_its_base_loses_and_the_room_stops() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let base = s.structure_at(StructureKind::Base, Team::BLUE, 14.0, 10.0);
    s.arm(a, GUN);
    s.set_structure_hp(base, 10);
    assert_eq!(s.sim.outcome(), None);

    s.run_ticks(10);

    assert_eq!(s.sim.outcome(), Some(Outcome::Victory(Team::RED)));
    let (tick, pos) = (s.sim.tick(), s.ship(a).pos);
    s.sail_to(a, 30.0, 10.0);
    s.run_ticks(20);
    assert_eq!(
        (s.sim.tick(), s.ship(a).pos),
        (tick, pos),
        "nothing moves after the end"
    );
}

#[test]
fn two_bases_that_fall_on_the_same_tick_are_a_draw() {
    let mut s = Scenario::open_sea();
    let red = s.structure_at(StructureKind::Base, Team::RED, 5.0, 5.0);
    let blue = s.structure_at(StructureKind::Base, Team::BLUE, 35.0, 35.0);
    s.set_structure_hp(red, 0);
    s.set_structure_hp(blue, 0);

    s.run_ticks(1);

    assert_eq!(s.sim.outcome(), Some(Outcome::Draw));
}

/// 기본 맵에서 사람 없이 NPC 배만으로 한 판이 끝까지 진행되는지 확인합니다.
/// 두 진영의 수치가 같으면 NPC 배끼리 라인 한가운데에서 비기므로, 붉은 진영은 본진만 남기고 그 HP를 낮춰서 시작합니다.
/// 푸른 진영의 NPC 배가 라인을 끝까지 지나 본진을 파괴해야 합니다.
#[test]
fn the_default_game_plays_to_the_end_with_creeps_alone() {
    let mut sim = Sim::new(Tuning::default());
    let structures = &mut sim.world_mut().structures;
    structures.retain(|_, site| site.team == Team::BLUE || site.kind == StructureKind::Base);
    for site in structures
        .values_mut()
        .filter(|site| site.team == Team::RED)
    {
        site.hp = 300;
    }

    let no_input = Default::default();
    let half_an_hour = 20 * 60 * 30;
    while sim.outcome().is_none() && sim.tick() < half_an_hour {
        sim.step(&no_input);
    }

    assert_eq!(
        sim.outcome(),
        Some(Outcome::Victory(Team::BLUE)),
        "after {} ticks",
        sim.tick()
    );
}
