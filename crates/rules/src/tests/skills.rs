use super::support::{BIG_HULL, BRACE, GUN, RELOAD, REPAIR, SALVO, SPRINT, Scenario, assert_close};
use crate::command::{Command, Refusal};
use crate::ids::{ShipId, SkillId, Team};
use crate::structures::StructureKind;

#[test]
fn a_ship_gains_a_level_and_a_skill_point_at_each_experience_step() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    let seen = |s: &Scenario| {
        let tuning = s.sim.tuning();
        (
            s.ship(ship).level(tuning),
            s.ship(ship).skill_points(tuning),
        )
    };
    assert_eq!(seen(&s), (1, 1));
    s.set_xp(ship, 99);
    assert_eq!(seen(&s), (1, 1));
    s.set_xp(ship, 100);
    assert_eq!(seen(&s), (2, 2));
    s.set_xp(ship, 5000);
    assert_eq!(seen(&s), (3, 3), "the table ends at level 3");
}

#[test]
fn ships_near_a_sunk_enemy_share_its_bounty_as_experience() {
    let mut s = Scenario::open_sea();
    let killer = s.ship_at(Team::RED, 10.0, 10.0);
    let near = s.ship_at(Team::RED, 10.0, 18.0);
    let far = s.ship_at(Team::RED, 10.0, 25.0);
    let victim = s.ship_at(Team::BLUE, 13.0, 10.0);
    let bystander = s.ship_at(Team::BLUE, 14.0, 10.0);
    s.arm(killer, GUN);
    s.set_ship_hp(victim, 1);

    s.run_ticks(5);

    assert!(!s.ship(victim).is_afloat());
    let xp = |id| s.ship(id).xp;
    assert_eq!((xp(killer), xp(near), xp(far)), (100, 100, 0));
    assert_eq!((xp(victim), xp(bystander)), (0, 0));
    assert_eq!((s.ship(killer).gold, s.ship(near).gold), (200, 0));
}

#[test]
fn learning_a_skill_spends_a_point_up_to_the_highest_rank() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    assert_eq!(s.command(ship, Command::Learn(REPAIR)), None);
    assert_eq!(
        s.command(ship, Command::Learn(SALVO)),
        Some(Refusal::NoSkillPoint)
    );

    s.set_xp(ship, 300);
    assert_eq!(s.command(ship, Command::Learn(REPAIR)), None);
    assert_eq!(
        s.command(ship, Command::Learn(REPAIR)),
        Some(Refusal::MaxRank)
    );
    assert_eq!(
        s.command(ship, Command::Learn(SkillId(8))),
        Some(Refusal::UnknownItem)
    );

    let ranks: Vec<u8> = s.ship(ship).skills.iter().map(|skill| skill.rank).collect();
    assert_eq!(ranks, [2, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(s.ship(ship).skill_points(s.sim.tuning()), 1);
}

#[test]
fn a_skill_needs_a_rank_and_waits_for_its_cooldown() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    s.set_ship_hp(ship, 500);
    assert_eq!(
        s.command(ship, Command::Cast(REPAIR)),
        Some(Refusal::NotLearned)
    );
    s.command(ship, Command::Learn(REPAIR));

    assert_eq!(s.command(ship, Command::Cast(REPAIR)), None);
    assert_eq!(s.ship(ship).hp, 600);
    s.run_ticks(99);
    assert_eq!(
        s.command(ship, Command::Cast(REPAIR)),
        Some(Refusal::OnCooldown)
    );
    s.run_ticks(1);
    assert_eq!(s.command(ship, Command::Cast(REPAIR)), None);
    assert_eq!(s.ship(ship).hp, 700);
}

#[test]
fn repair_does_not_go_above_the_hull_hp() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    s.set_ship_hp(ship, 950);
    s.command(ship, Command::Learn(REPAIR));

    s.command(ship, Command::Cast(REPAIR));

    assert_eq!(s.ship(ship).hp, 1000);
}

#[test]
fn a_salvo_hurts_every_enemy_ship_and_creep_in_its_radius_on_the_next_tick() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    let ally = s.ship_at(Team::RED, 11.0, 10.0);
    let inside = s.ship_at(Team::BLUE, 14.0, 10.0);
    let outside = s.ship_at(Team::BLUE, 14.1, 10.0);
    let creep = s.creep_at(Team::BLUE, 10.0, 13.0);
    let tower = s.tower_at(Team::BLUE, 10.0, 8.0);
    s.set_xp(ship, 100);
    s.command(ship, Command::Learn(SALVO));
    s.command(ship, Command::Learn(SALVO));

    assert_eq!(s.command(ship, Command::Cast(SALVO)), None);
    assert_eq!(s.ship(inside).hp, 1000, "the damage lands on the next tick");
    s.run_ticks(1);

    let hp = |id| s.ship(id).hp;
    assert_eq!((hp(inside), hp(outside), hp(ally)), (940, 1000, 1000));
    assert_eq!(s.sim.creep(creep).map(|creep| creep.hp), Some(40));
    assert_eq!(s.sim.structure(tower).map(|tower| tower.hp), Some(1000));
}

#[test]
fn a_kill_by_a_salvo_pays_the_bounty() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    let creep = s.creep_at(Team::BLUE, 12.0, 10.0);
    s.sim.world_mut().creeps.get_mut(&creep).unwrap().hp = 30;
    s.command(ship, Command::Learn(SALVO));

    s.command(ship, Command::Cast(SALVO));
    s.run_ticks(1);

    assert_eq!(s.sim.creep(creep), None);
    assert_eq!((s.ship(ship).gold, s.ship(ship).xp), (20, 20));
}

#[test]
fn a_sprint_makes_the_ship_faster_for_its_duration() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    s.command(ship, Command::Learn(SPRINT));
    s.sail_to(ship, 30.0, 10.0);

    s.command(ship, Command::Cast(SPRINT));
    s.run_ticks(20);
    // 20Tick 동안 1초에 6타일의 속도로 갑니다.
    assert_close(s.ship(ship).pos.x, 16.0);

    s.run_ticks(20);
    assert_close(s.ship(ship).pos.x, 20.0);
}

#[test]
fn a_sunk_ship_can_learn_but_cannot_cast() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    let enemy = s.ship_at(Team::BLUE, 13.0, 10.0);
    s.arm(enemy, GUN);
    s.set_ship_hp(ship, 1);
    s.run_ticks(5);
    assert!(!s.ship(ship).is_afloat());

    assert_eq!(s.command(ship, Command::Learn(REPAIR)), None);
    assert_eq!(s.command(ship, Command::Cast(REPAIR)), Some(Refusal::Sunk));
}

/// 자기 진영의 본진 옆에 400골드를 가진 배를 놓습니다. 1번 선체의 가격이 400골드입니다.
fn ship_at_the_shop() -> (Scenario, ShipId) {
    let mut s = Scenario::open_sea();
    s.structure_at(StructureKind::Base, Team::RED, 10.0, 10.0);
    let ship = s.ship_at(Team::RED, 11.0, 10.0);
    s.set_gold(ship, 400);
    (s, ship)
}

#[test]
fn a_ship_learns_and_casts_only_the_skills_of_its_hull() {
    let (mut s, ship) = ship_at_the_shop();
    s.command(ship, Command::ChangeHull(BIG_HULL));

    assert_eq!(
        s.command(ship, Command::Learn(SPRINT)),
        Some(Refusal::NotOnThisHull)
    );
    assert_eq!(
        s.command(ship, Command::Cast(SPRINT)),
        Some(Refusal::NotOnThisHull)
    );
    assert_eq!(s.command(ship, Command::Learn(RELOAD)), None);
}

#[test]
fn changing_the_hull_returns_the_points_of_the_skills_the_new_hull_lacks() {
    let (mut s, ship) = ship_at_the_shop();
    s.set_xp(ship, 100);
    s.command(ship, Command::Learn(SPRINT));
    s.command(ship, Command::Learn(REPAIR));

    assert_eq!(s.command(ship, Command::ChangeHull(BIG_HULL)), None);

    let after = s.ship(ship);
    let rank = |skill: SkillId| after.skills[usize::from(skill.0)].rank;
    assert_eq!((rank(SPRINT), rank(REPAIR)), (0, 1));
    assert_eq!(after.skill_points(s.sim.tuning()), 1);
    assert_eq!(s.command(ship, Command::Learn(BRACE)), None);
}

#[test]
fn a_reload_lets_every_weapon_fire_again_at_once() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    s.ship_at(Team::BLUE, 13.0, 10.0);
    s.arm(ship, GUN);
    s.command(ship, Command::Learn(RELOAD));
    s.run_ticks(1);
    let ready_at = |s: &Scenario| s.ship(ship).mounts[0].map(|mount| mount.ready_at);
    assert_eq!(ready_at(&s), Some(11), "the gun fired on the first tick");

    assert_eq!(s.command(ship, Command::Cast(RELOAD)), None);
    assert_eq!(ready_at(&s), Some(1));

    s.run_ticks(1);
    assert_eq!(ready_at(&s), Some(12), "the gun fired again");
}

#[test]
fn a_brace_reduces_the_damage_the_ship_takes_for_its_duration() {
    let mut s = Scenario::open_sea();
    let ship = s.ship_at(Team::RED, 10.0, 10.0);
    let enemy = s.ship_at(Team::BLUE, 13.0, 10.0);
    s.arm(enemy, GUN);
    s.command(ship, Command::Learn(BRACE));

    assert_eq!(s.command(ship, Command::Cast(BRACE)), None);
    // 포탄은 4, 14, 24번째 Tick에 닿습니다. 효과는 20번째 Tick까지이고, 피해 10의 40%를 줄입니다.
    s.run_ticks(15);
    assert_eq!(s.ship(ship).hp, 988);

    s.run_ticks(10);
    assert_eq!(s.ship(ship).hp, 978);
}
