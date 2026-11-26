//! 골드입니다. 모든 배가 1초마다 수입을 받고, 적을 가라앉히거나 건물을 파괴한 포탄을 쏜 배가 보상을 받습니다.

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

/// HP를 0으로 만든 포탄을 쏜 배에 보상을 줍니다. NPC 배나 건물이 쏜 포탄에는 보상이 없습니다.
/// 지우기 전의 개체를 읽으므로, 가라앉은 NPC 배와 파괴된 건물을 지우기 전에 불러야 합니다.
pub(crate) fn pay_bounties(world: &mut World, tuning: &Tuning, kills: &[Kill]) {
    for kill in kills {
        let bounty = match kill.target {
            Target::Ship(_) => tuning.ship.bounty,
            Target::Creep(id) => world
                .creeps
                .get(&id)
                .map_or(0, |creep| tuning.creeps[creep.kind].bounty),
            Target::Structure(id) => world
                .structures
                .get(&id)
                .map_or(0, |structure| tuning.structures.of(structure.kind).bounty),
        };
        if let Some(ship) = kill.by.and_then(|id| world.ships.get_mut(&id)) {
            ship.gold += bounty;
        }
    }
}
