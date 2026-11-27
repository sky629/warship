//! 중립 요소를 확인합니다. 중립 몬스터는 자리를 지키면서 두 진영 모두와 싸우고, 가라앉으면 정해진 시간 뒤에 다시 나옵니다.
//! 교역항은 어느 진영의 배나 쓰는 상점이고, 상점마다 파는 것이 다릅니다. 테스트의 교역항은 (30, 10)에 있고 `GUN`과 용병만 팝니다. 파는 일은 어느 상점에서나 합니다.
//! 용병은 100골드에 0번 NPC 배 두 척이고, 고른 라인의 자기 진영 항구에서 나옵니다.
//! 라인은 (5, 10)에서 (35, 10)까지 하나입니다.
//! 중립 몬스터의 자리는 (20, 20)이고, 몬스터는 0번 NPC 배(HP 100, 보상 20)입니다. 수입은 없습니다.

use super::support::{GUN, LONG_GUN, Scenario};
use crate::command::{Command, Refusal};
use crate::ids::Team;
use crate::math::Vec2;
use crate::structures::StructureKind;
use crate::tuning::Camp;

fn camp(respawn_ticks: u32) -> Scenario {
    Scenario::with_tuning(|tuning| {
        tuning.economy.income_per_sec = 0;
        let lane = vec![Vec2::new(5.0, 10.0), Vec2::new(35.0, 10.0)];
        tuning.map.lanes.push(lane);
        tuning.map.camps.push(Camp {
            pos: Vec2::new(20.0, 20.0),
            creep: 0,
            respawn_ticks,
        });
    })
}

fn wild(s: &Scenario) -> Vec<(Vec2, u32)> {
    let wild = s.sim.creeps().filter(|creep| creep.team == Team::WILD);
    wild.map(|creep| (creep.pos, creep.hp)).collect()
}

#[test]
fn a_monster_keeps_its_place_and_fights_the_ships_of_both_teams() {
    for team in Team::ALL {
        let mut s = camp(1000);
        let ship = s.ship_at(team, 23.0, 20.0);

        s.run_ticks(40);

        assert_eq!(wild(&s), [(Vec2::new(20.0, 20.0), 100)]);
        assert!(s.ship(ship).hp < 1000, "the monster shot the ship");
    }
}

#[test]
fn a_sunk_monster_pays_its_bounty_and_comes_back_after_a_while() {
    let mut s = camp(1000);
    let ship = s.ship_at(Team::RED, 23.0, 20.0);
    s.arm(ship, GUN);
    let gold = s.ship(ship).gold;

    s.run_ticks(400);
    assert_eq!(wild(&s), []);
    assert_eq!(s.ship(ship).gold, gold + 20);

    // 다시 나온 몬스터는 HP가 가득하고, 곁에 있는 배가 다시 쏩니다.
    s.sail_to(ship, 23.0, 35.0);
    s.run_ticks(1000);
    assert_eq!(wild(&s), [(Vec2::new(20.0, 20.0), 100)]);
}

#[test]
fn a_ship_of_either_team_buys_what_a_trade_port_sells() {
    for team in Team::ALL {
        let mut s = camp(1000);
        let near = s.ship_at(team, 31.0, 10.0);
        let far = s.ship_at(team, 31.0, 30.0);
        s.set_gold(near, 500);
        s.set_gold(far, 500);

        assert_eq!(s.command(near, Command::Buy(GUN)), None);
        assert_eq!(
            s.command(far, Command::Buy(GUN)),
            Some(Refusal::NoShopInRange)
        );
        // 교역항은 `GUN`만 팝니다. 다른 무기는 본진에서 삽니다.
        assert_eq!(
            s.command(near, Command::Buy(LONG_GUN)),
            Some(Refusal::NotSoldHere)
        );
        assert_eq!(s.command(near, Command::Sell { slot: 0 }), None);
    }
}

#[test]
fn hired_creeps_come_out_of_the_harbor_of_the_lane_and_follow_it() {
    let mut s = camp(1000);
    s.structure_at(StructureKind::Harbor, Team::RED, 12.0, 10.0);
    let ship = s.ship_at(Team::RED, 13.0, 10.0);
    s.set_gold(ship, 250);

    assert_eq!(s.command(ship, Command::Hire { lane: 0 }), None);
    s.run_ticks(1);

    assert_eq!(s.ship(ship).gold, 150);
    let hired = s.sim.creeps().filter(|creep| creep.team == Team::RED);
    let lanes: Vec<Option<usize>> = hired.map(|creep| creep.lane).collect();
    assert_eq!(lanes, [Some(0), Some(0)]);
}

#[test]
fn a_hire_is_refused_without_a_lane_a_harbor_or_gold() {
    let mut s = camp(1000);
    s.structure_at(StructureKind::Base, Team::RED, 10.0, 10.0);
    let ship = s.ship_at(Team::RED, 11.0, 10.0);
    s.set_gold(ship, 250);
    assert_eq!(
        s.command(ship, Command::Hire { lane: 9 }),
        Some(Refusal::UnknownItem)
    );
    assert_eq!(
        s.command(ship, Command::Hire { lane: 0 }),
        Some(Refusal::NoHarbor)
    );

    s.structure_at(StructureKind::Harbor, Team::RED, 12.0, 10.0);
    s.set_gold(ship, 99);
    assert_eq!(
        s.command(ship, Command::Hire { lane: 0 }),
        Some(Refusal::NotEnoughGold)
    );
    s.run_ticks(1);
    assert_eq!(s.sim.creeps().count(), 1, "only the monster is there");
}
