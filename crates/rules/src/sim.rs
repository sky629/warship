//! `Sim`은 방 하나의 게임 상태 전체이며, 규칙 크레이트의 공개 진입점입니다.

use crate::ids::{ShipId, Team, TowerId};
use crate::input::PlayerInputs;
use crate::ship::{self, Ship};
use crate::structures::{self, Tower};
use crate::tuning::Tuning;
use crate::vision::{self, Sight};
use crate::weapon::{self, Target};
use crate::world::World;

/// 방 하나의 게임입니다. 바깥 코드는 `&Sim`으로 상태를 읽고, `add_ship`과 `step`으로만 상태를 바꿉니다.
pub struct Sim {
    tuning: Tuning,
    /// 방이 만들어진 뒤 실행한 Tick의 수입니다.
    tick: u64,
    world: World,
    /// 마지막 Tick에 진영마다 보인 적입니다. 첫 Tick을 실행하기 전에는 아무도 보이지 않습니다.
    sight: Sight,
    next_ship: u32,
}

/// 두 진영이 모두 가득 차서 배를 더 받을 수 없습니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomFull {
    /// 방 전체의 정원입니다.
    pub capacity: usize,
}

impl Sim {
    /// 배가 없는 방을 만들고 맵에 적힌 타워를 세웁니다. 수치는 `Tuning::validate`를 통과한 것이어야 합니다.
    pub fn new(tuning: Tuning) -> Sim {
        let mut world = World::default();
        for (index, site) in tuning.map.towers.iter().enumerate() {
            let id = TowerId(index as u32 + 1);
            let tower = Tower {
                id,
                team: site.team,
                pos: site.pos,
                hp: tuning.tower.max_hp,
                ready_at: 0,
                target: None,
            };
            world.towers.insert(id, tower);
        }
        Sim {
            tuning,
            tick: 0,
            world,
            sight: Sight::default(),
            next_ship: 1,
        }
    }

    /// 새 플레이어의 배를 배가 적은 진영에 추가합니다. 두 진영의 배의 수가 같으면 `Team::RED`에 추가합니다.
    /// 두 진영이 모두 가득 차 있으면 거절합니다.
    pub fn join(&mut self) -> Result<ShipId, RoomFull> {
        let size = |team| self.ships().filter(|ship| ship.team == team).count();
        let (red, blue) = (size(Team::RED), size(Team::BLUE));
        if red.min(blue) >= self.tuning.team_size {
            return Err(RoomFull {
                capacity: self.tuning.team_size * 2,
            });
        }
        let team = if blue < red { Team::BLUE } else { Team::RED };
        Ok(self.add_ship(team))
    }

    /// 배를 즉시 제거합니다. 없는 번호는 무시하므로 같은 퇴장을 두 번 처리해도 안전합니다.
    pub fn remove_ship(&mut self, id: ShipId) {
        self.world.ships.remove(&id);
    }

    /// 새 배를 최대 HP로 그 진영의 시작 자리에 추가하고 번호를 돌려줍니다.
    pub(crate) fn add_ship(&mut self, team: Team) -> ShipId {
        let id = ShipId(self.next_ship);
        self.next_ship += 1;
        let spawn = self.tuning.map.spawns[team.index()];
        self.world
            .ships
            .insert(id, Ship::new(id, team, spawn, &self.tuning));
        id
    }

    /// Tick 하나를 실행합니다. 배가 움직이고, 진영별 시야를 정하고, 쏠 수 있는 무기가 쏘고, 닿을 때가 된 포탄이 피해를 줍니다.
    /// 무기는 이번 Tick의 피해가 들어가기 전의 상태를 보고 쏘므로, 같은 Tick에 가라앉는 배도 그 Tick의 사격은 마칩니다.
    pub fn step(&mut self, inputs: &PlayerInputs) {
        self.tick += 1;
        ship::move_ships(&mut self.world, inputs, &self.tuning);
        self.sight = vision::look(&self.world, &self.tuning);
        weapon::fire_all(&mut self.world, &self.sight, &self.tuning, self.tick);
        weapon::land_impacts(&mut self.world, self.tick);
        structures::remove_destroyed(&mut self.world);
    }

    /// 마지막 Tick에 `team`이 이 적을 보았는지입니다. 진영별 스냅샷은 보인 적만 싣습니다.
    pub fn sees(&self, team: Team, target: Target) -> bool {
        self.sight.sees(team, target)
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn tuning(&self) -> &Tuning {
        &self.tuning
    }

    /// 번호가 작은 배부터입니다. 가라앉은 배도 들어 있습니다.
    pub fn ships(&self) -> impl Iterator<Item = &Ship> {
        self.world.ships.values()
    }

    pub fn ship(&self, id: ShipId) -> Option<&Ship> {
        self.world.ships.get(&id)
    }

    /// 번호가 작은 타워부터입니다. 파괴된 타워는 없습니다.
    pub fn towers(&self) -> impl Iterator<Item = &Tower> {
        self.world.towers.values()
    }

    pub fn tower(&self, id: TowerId) -> Option<&Tower> {
        self.world.towers.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}
