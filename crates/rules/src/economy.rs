//! 골드와 경험치입니다. 모든 배가 1초마다 수입을 받고, 적을 가라앉히거나 건물을 파괴한 포탄을 쏜 배가 보상을 받습니다.
//! 그 자리 가까이에 있던 배는 경험치를 받습니다.

use crate::ship::Ship;
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

/// HP를 0으로 만든 포탄을 쏜 배에 보상 골드를 주고, 그 자리 가까이에 떠 있는 적 진영의 배에 같은 양의 경험치를 나누어 줍니다.
/// NPC 배나 건물이 쏜 포탄에는 골드가 없습니다. 경험치는 누가 쏘았는지와 관계없이 줍니다.
/// 지우기 전의 개체를 읽으므로, 가라앉은 NPC 배와 파괴된 건물을 지우기 전에 불러야 합니다.
pub(crate) fn pay_bounties(world: &mut World, tuning: &Tuning, kills: &[Kill]) {
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
            ship.gold += bounty;
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
