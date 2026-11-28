//! 골드와 경험치입니다. 모든 배가 1초마다 수입을 받고, 적을 가라앉히거나 건물을 파괴한 포탄을 쏜 배가 보상을 받습니다.
//! 플레이어의 배가 가라앉으면, 그 배를 최근에 맞힌 다른 배들도 준 피해의 비율만큼 도움의 보상을 받습니다.
//! 그 자리 가까이에 있던 배는 경험치를 받습니다. 배마다 가라앉힌 적의 수와 가라앉은 횟수도 여기서 셉니다.

use crate::ids::ShipId;
use crate::note::NoteKind;
use crate::ship::{Ship, Wound};
use crate::tuning::Tuning;
use crate::weapon::{Kill, Target};
use crate::world::World;

/// 1초에 한 번, 모든 배에 수입을 줍니다. 가라앉아 있는 배도 받습니다.
pub(crate) fn pay_income(world: &mut World, tuning: &Tuning, tick: u64) {
    if !tick.is_multiple_of(u64::from(tuning.tick_hz)) {
        return;
    }
    for ship in world.ships.values_mut() {
        ship.gold += tuning.economy.income_per_sec;
    }
}

/// HP를 0으로 만든 포탄을 쏜 배에 보상 골드를 주고, 그 자리 가까이에 떠 있는 적 진영의 배에 기본 보상과 같은 양의 경험치를 나누어 줍니다.
/// 플레이어의 배가 가라앉으면 `pay_assists`가 도운 배들에 골드를 줍니다.
/// NPC 배나 건물이 쏜 포탄에는 골드가 없습니다. 경험치는 누가 쏘았는지와 관계없이 줍니다.
/// 지우기 전의 개체를 읽으므로, 가라앉은 NPC 배와 파괴된 건물을 지우기 전에 불러야 합니다.
pub(crate) fn pay_bounties(world: &mut World, tuning: &Tuning, kills: &[Kill], tick: u64) {
    for kill in kills {
        let victim = match kill.target {
            Target::Ship(id) => world
                .ships
                .get(&id)
                .map(|ship| (tuning.ship.bounty, ship.team, ship.pos)),
            Target::Creep(id) => world
                .creeps
                .get(&id)
                .map(|creep| (tuning.creeps[creep.kind].bounty, creep.team, creep.pos)),
            Target::Structure(id) => world.structures.get(&id).map(|structure| {
                let bounty = tuning.structures.of(structure.kind).bounty;
                (bounty, structure.team, structure.pos)
            }),
        };
        let Some((bounty, team, pos)) = victim else {
            continue;
        };
        if let Some(ship) = kill.by.and_then(|id| world.ships.get_mut(&id)) {
            // 보상의 숫자는 받은 배의 자리에 띄웁니다. 가라앉은 개체의 자리는 받는 진영에 보이지 않을 수 있습니다.
            let (id, at, gold) = (ship.id, ship.pos, bounty);
            ship.gold += gold;
            match kill.target {
                Target::Ship(_) => ship.kills += 1,
                Target::Creep(_) => ship.creep_kills += 1,
                Target::Structure(_) => {}
            }
            if gold > 0 {
                world.note(tick, at, gold, NoteKind::Gold { ship: id });
            }
        }
        if let Target::Ship(id) = kill.target
            && let Some(ship) = world.ships.get_mut(&id)
        {
            ship.deaths += 1;
            let wounds = std::mem::take(&mut ship.wounds);
            pay_assists(world, tuning, &wounds, kill.by, tick);
        }
        let near = |ship: &&mut Ship| {
            ship.is_afloat()
                && ship.team.is_hostile_to(team)
                && ship.pos.distance(pos) <= tuning.growth.share_range
        };
        let mut sharers: Vec<&mut Ship> = world.ships.values_mut().filter(near).collect();
        let share = bounty / sharers.len().max(1) as u32;
        for ship in &mut sharers {
            ship.xp += share;
        }
    }
}

/// 가라앉은 배를 최근에 맞힌 배들에 도움의 보상을 줍니다. 가라앉힌 배(`killer`)는 격침의 보상을 받으므로 뺍니다.
/// 도운 배들은 `assist_bounty`를 자기들이 준 피해의 비율로 나누어 받습니다. 가라앉힌 배가 준 피해는 이 비율에 넣지 않습니다.
fn pay_assists(
    world: &mut World,
    tuning: &Tuning,
    wounds: &[Wound],
    killer: Option<ShipId>,
    tick: u64,
) {
    let helped = |wound: &&Wound| {
        Some(wound.by) != killer && wound.last + u64::from(tuning.ship.assist_ticks) > tick
    };
    let wounds: Vec<&Wound> = wounds.iter().filter(helped).collect();
    let total: u64 = wounds.iter().map(|wound| u64::from(wound.damage)).sum();
    for wound in wounds {
        let gold = u64::from(tuning.ship.assist_bounty) * u64::from(wound.damage) / total.max(1);
        let Some(ship) = world.ships.get_mut(&wound.by) else {
            continue;
        };
        if gold == 0 {
            continue;
        }
        ship.gold += gold as u32;
        let (id, at) = (ship.id, ship.pos);
        world.note(tick, at, gold as u32, NoteKind::Gold { ship: id });
    }
}
