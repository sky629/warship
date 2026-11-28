//! 골드를 확인합니다. 수입은 1초(20Tick)에 2골드이고, 보상은 배 200, NPC 배 20, 타워 300입니다.

use super::support::{FIRE_GUN, GUN, Scenario};
use crate::ids::Team;
use crate::note::{NOTE_TICKS, NoteKind};
use crate::weapon::Target;

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
    let score = |id| (s.ship(id).kills, s.ship(id).deaths);
    assert_eq!(
        [score(killer), score(helper), score(victim)],
        [(1, 0), (0, 0), (0, 1)]
    );
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

#[test]
fn ships_that_hurt_a_sunk_enemy_share_the_assist_gold_by_the_damage_they_dealt() {
    let mut s = Scenario::open_sea();
    let killer = s.ship_at(Team::RED, 10.0, 10.0);
    let helper = s.ship_at(Team::RED, 30.0, 10.0);
    let late = s.ship_at(Team::RED, 30.0, 12.0);
    let other = s.ship_at(Team::RED, 30.0, 14.0);
    let victim = s.ship_at(Team::BLUE, 11.0, 10.0);
    s.arm(killer, GUN);
    s.run_ticks(100);
    // 도운 두 배는 30과 10을 주었고, 또 다른 배는 30을 주었지만 100Tick보다 오래 전입니다. 가라앉히는 포탄은 10을 줍니다.
    let (tuning, tick) = (s.sim.tuning().clone(), s.sim.tick());
    let hurt = s.sim.world_mut().ships.get_mut(&victim).unwrap();
    hurt.wounds.clear();
    hurt.wound(late, 30, &tuning, tick - 100);
    hurt.wound(helper, 30, &tuning, tick);
    hurt.wound(other, 10, &tuning, tick);
    hurt.hp = 10;
    let before = [killer, helper, late, other].map(|id| s.ship(id).gold);

    s.run_ticks(40);

    assert!(!s.ship(victim).is_afloat());
    let paid = [killer, helper, late, other].map(|id| s.ship(id).gold);
    let income = 2 * 2;
    assert_eq!(paid[0] - before[0], 200 + income, "the bounty is fixed");
    assert_eq!(
        paid[1] - before[1],
        200 * 30 / 40 + income,
        "the helpers split the assist gold among themselves"
    );
    assert_eq!(paid[3] - before[3], 200 * 10 / 40 + income);
    assert_eq!(paid[2] - before[2], income, "an old hit is no help");
    assert!(s.ship(victim).wounds.is_empty());
    let assists = [killer, helper, late, other].map(|id| s.ship(id).assists);
    assert_eq!(assists, [0, 1, 0, 1]);
}

#[test]
fn one_sunk_ship_pays_a_team_at_most_250_gold() {
    let mut s = Scenario::with_tuning(|tuning| tuning.ship = crate::Tuning::default().ship);
    let killer = s.ship_at(Team::RED, 10.0, 10.0);
    let helpers = [12.0, 14.0, 16.0, 18.0].map(|y| s.ship_at(Team::RED, 30.0, y));
    let victim = s.ship_at(Team::BLUE, 11.0, 10.0);
    s.arm(killer, GUN);
    let (tuning, tick) = (s.sim.tuning().clone(), s.sim.tick());
    let hurt = s.sim.world_mut().ships.get_mut(&victim).unwrap();
    for helper in helpers {
        hurt.wound(helper, 1000, &tuning, tick);
    }
    hurt.hp = 10;

    s.run_ticks(2);

    assert!(!s.ship(victim).is_afloat());
    let paid: u32 = s.sim.ships().map(|ship| ship.gold).sum();
    assert_eq!(s.ship(killer).gold, 200);
    assert_eq!(paid, 200 + 4 * (50 / 4), "each of the four helpers gets 12");
}

#[test]
fn a_hit_and_a_bounty_are_noted_for_a_second() {
    let mut s = Scenario::open_sea();
    let killer = s.ship_at(Team::RED, 10.0, 10.0);
    let victim = s.ship_at(Team::BLUE, 11.0, 10.0);
    s.arm(killer, GUN);
    s.set_ship_hp(victim, 10);
    s.run_ticks(2);
    assert!(!s.ship(victim).is_afloat());

    let notes = s.sim.notes().to_vec();
    let hit = NoteKind::Hit {
        by: Some(killer),
        target: Target::Ship(victim),
        seen: [true, false],
    };
    let kinds: Vec<NoteKind> = notes.iter().map(|note| note.kind).collect();
    assert_eq!(kinds, [hit, NoteKind::Gold { ship: killer }]);
    assert_eq!((notes[0].amount, notes[1].amount), (10, 200));
    assert_eq!(notes[0].pos, s.ship(victim).pos);
    assert_eq!(
        notes[1].pos,
        s.ship(killer).pos,
        "the gold floats over the paid ship"
    );
    assert_eq!((notes[0].id, notes[1].id), (0, 1));

    let noted = notes[0].tick;
    s.run_ticks((noted + NOTE_TICKS - 1 - s.sim.tick()) as u32);
    assert_eq!(s.sim.notes().len(), 2, "the notes stay until the last tick");
    s.run_ticks(1);
    assert!(s.sim.notes().is_empty());
}

#[test]
fn the_income_is_not_noted() {
    let mut s = Scenario::open_sea();
    s.ship_at(Team::RED, 10.0, 10.0);
    s.run_ticks(40);
    assert!(s.sim.notes().is_empty());
}

#[test]
fn two_ships_that_shoot_the_same_enemy_get_the_bounty_and_the_assist_gold() {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let b = s.ship_at(Team::RED, 10.0, 11.0);
    let victim = s.ship_at(Team::BLUE, 11.0, 10.0);
    s.arm(a, GUN);
    s.arm(b, GUN);
    s.set_ship_hp(victim, 100);

    let mut ticks = 0;
    while s.ship(victim).is_afloat() {
        s.run_ticks(1);
        ticks += 1;
        assert!(ticks < 400, "the victim never sank");
    }

    let income = 2 * (ticks / 20);
    // 한 배는 격침의 보상을 받고, 다른 배는 도움의 보상을 전부 받습니다.
    let gold = [a, b].map(|id| s.ship(id).gold - income);
    assert_eq!(gold, [200, 200]);
    let kills: u32 = [a, b].iter().map(|id| s.ship(*id).kills).sum();
    let assists: u32 = [a, b].iter().map(|id| s.ship(*id).assists).sum();
    assert_eq!((kills, assists), (1, 1));
}

/// 붉은 배가 화염포로 한 번 맞힌 뒤 `wait` Tick이 지나서, 붉은 타워가 그 푸른 배를 가라앉힙니다. 붉은 배가 수입 밖에 얻은 골드와 격침의 수입니다.
fn gold_after_a_tower_sinks_a_ship_hit(wait: u32) -> (u32, u32) {
    let mut s = Scenario::open_sea();
    let a = s.ship_at(Team::RED, 10.0, 10.0);
    let victim = s.ship_at(Team::BLUE, 11.0, 10.0);
    s.arm(a, FIRE_GUN);
    s.run_ticks(wait);
    assert!(s.ship(victim).hp < 1000, "the ship never hit the victim");
    s.set_ship_hp(victim, 10);
    s.tower_at(Team::RED, 12.0, 10.0);

    let mut ticks = wait;
    while s.ship(victim).is_afloat() {
        s.run_ticks(1);
        ticks += 1;
        assert!(ticks < 190, "the tower never sank the victim");
    }
    let income = 2 * (ticks / 20);
    (s.ship(a).gold - income, s.ship(a).kills)
}

#[test]
fn the_last_ship_to_hit_gets_the_bounty_when_a_tower_sinks_the_enemy() {
    assert_eq!(gold_after_a_tower_sinks_a_ship_hit(60), (200, 1));
}

#[test]
fn a_hit_older_than_the_assist_time_earns_nothing_when_a_tower_sinks_the_enemy() {
    assert_eq!(gold_after_a_tower_sinks_a_ship_hit(160), (0, 0));
}
