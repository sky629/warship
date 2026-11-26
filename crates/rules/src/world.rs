//! 방 하나에 있는 모든 개체입니다.

use std::collections::BTreeMap;

use crate::ids::{ShipId, Team, TowerId};
use crate::math::Vec2;
use crate::ship::{Life, Ship};
use crate::structures::Tower;
use crate::weapon::{Impact, Target};

#[derive(Default)]
pub(crate) struct World {
    pub ships: BTreeMap<ShipId, Ship>,
    pub towers: BTreeMap<TowerId, Tower>,
    /// 쏘았지만 아직 표적에 닿지 않은 포탄의 피해입니다.
    pub impacts: Vec<Impact>,
}

impl World {
    /// 표적이 될 수 있는 개체(떠 있는 배와 서 있는 타워)를 번호 순서로, 배부터 나열합니다.
    pub fn targets(&self) -> impl Iterator<Item = (Target, Team, Vec2)> + '_ {
        let ships = self
            .ships
            .values()
            .filter(|ship| ship.is_afloat())
            .map(|ship| (Target::Ship(ship.id), ship.team, ship.pos));
        let towers = self
            .towers
            .values()
            .filter(|tower| tower.hp > 0)
            .map(|tower| (Target::Tower(tower.id), tower.team, tower.pos));
        ships.chain(towers)
    }

    /// 표적의 HP를 줄입니다. HP가 0이 된 배는 가라앉습니다. 이미 사라진 표적이면 아무 일도 하지 않습니다.
    pub fn damage(&mut self, target: Target, amount: u32) {
        match target {
            Target::Ship(id) => {
                if let Some(ship) = self.ships.get_mut(&id).filter(|ship| ship.is_afloat()) {
                    ship.hp = ship.hp.saturating_sub(amount);
                    if ship.hp == 0 {
                        ship.life = Life::Sunk;
                    }
                }
            }
            Target::Tower(id) => {
                if let Some(tower) = self.towers.get_mut(&id) {
                    tower.hp = tower.hp.saturating_sub(amount);
                }
            }
        }
    }
}
