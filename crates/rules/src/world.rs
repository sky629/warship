//! 방 하나에 있는 모든 개체입니다.

use std::collections::BTreeMap;

use crate::creep::{CampState, Creep};
use crate::ids::{CreepId, ShipId, StructureId, Team};
use crate::math::Vec2;
use crate::ship::{Life, Ship};
use crate::structures::Structure;
use crate::trade;
use crate::weapon::{Impact, Target};

#[derive(Default)]
pub(crate) struct World {
    pub ships: BTreeMap<ShipId, Ship>,
    pub creeps: BTreeMap<CreepId, Creep>,
    pub structures: BTreeMap<StructureId, Structure>,
    /// 쏘았지만 아직 표적에 닿지 않은 포탄의 피해입니다.
    pub impacts: Vec<Impact>,
    /// 중립 몬스터의 자리의 상태입니다. 순서는 `MapSpec::camps`의 순서입니다.
    pub camps: Vec<CampState>,
    /// 고용했지만 아직 나오지 않은 용병의 진영과 라인입니다.
    pub hires: Vec<(Team, usize)>,
}

impl World {
    /// 표적이 될 수 있는 개체(떠 있는 배, NPC 배, 서 있는 건물)를 배, NPC 배, 건물의 순서로, 종류 안에서는 번호 순서로 나열합니다.
    pub fn targets(&self) -> impl Iterator<Item = (Target, Team, Vec2)> + '_ {
        let ships = self
            .ships
            .values()
            .filter(|ship| ship.is_afloat())
            .map(|ship| (Target::Ship(ship.id), ship.team, ship.pos));
        let creeps = self
            .creeps
            .values()
            .filter(|creep| creep.hp > 0)
            .map(|creep| (Target::Creep(creep.id), creep.team, creep.pos));
        let structures = self
            .structures
            .values()
            .filter(|structure| structure.hp > 0)
            .map(|structure| {
                (
                    Target::Structure(structure.id),
                    structure.team,
                    structure.pos,
                )
            });
        ships.chain(creeps).chain(structures)
    }

    /// 표적의 HP를 줄이고, 이 피해로 HP가 0이 되었는지 돌려줍니다. 이미 사라졌거나 가라앉은 표적이면 아무 일도 하지 않습니다.
    /// HP가 0이 된 배는 가라앉고 `respawn_at` Tick에 다시 나타납니다. 가라앉는 배는 교역의 계약과 화물을 잃습니다.
    pub fn damage(&mut self, target: Target, amount: u32, respawn_at: u64) -> bool {
        let hp = match target {
            Target::Ship(id) => {
                let Some(ship) = self.ships.get_mut(&id).filter(|ship| ship.is_afloat()) else {
                    return false;
                };
                ship.hp = ship.hp.saturating_sub(amount);
                if ship.hp == 0 {
                    ship.life = Life::Sunk { respawn_at };
                    trade::lose_freight(ship);
                }
                ship.hp
            }
            Target::Creep(id) => {
                let Some(creep) = self.creeps.get_mut(&id).filter(|creep| creep.hp > 0) else {
                    return false;
                };
                creep.hp = creep.hp.saturating_sub(amount);
                creep.hp
            }
            Target::Structure(id) => {
                let found = self.structures.get_mut(&id);
                let Some(structure) = found.filter(|structure| structure.hp > 0) else {
                    return false;
                };
                structure.hp = structure.hp.saturating_sub(amount);
                structure.hp
            }
        };
        hp == 0
    }
}
