//! `Sim`은 방 하나의 게임 상태 전체이며, 규칙 크레이트의 공개 진입점입니다.

use crate::creep::{self, Creep};
use crate::economy;
use crate::ids::{CreepId, ShipId, StructureId, Team};
use crate::input::PlayerInputs;
use crate::ship::{self, Ship};
use crate::structures::{self, Structure};
use crate::tuning::Tuning;
use crate::vision::{self, Sight};
use crate::weapon::{self, Target};
use crate::world::World;

/// 방 하나의 게임입니다. 바깥 코드는 `&Sim`으로 상태를 읽고, `join`, `remove_ship`, `step`으로만 상태를 바꿉니다.
pub struct Sim {
    tuning: Tuning,
    /// 방이 만들어진 뒤 실행한 Tick의 수입니다. 대전이 끝나면 늘지 않습니다.
    tick: u64,
    world: World,
    /// 마지막 Tick에 진영마다 보인 적입니다. 첫 Tick을 실행하기 전에는 아무도 보이지 않습니다.
    sight: Sight,
    next_ship: u32,
    next_creep: u32,
    /// 대전의 결과입니다. 본진이 파괴되기 전에는 없습니다.
    outcome: Option<Outcome>,
}

/// 대전이 끝난 결과입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// 상대의 본진을 파괴한 진영입니다.
    Victory(Team),
    /// 두 본진이 같은 Tick에 파괴되었습니다.
    Draw,
}

/// 두 진영이 모두 가득 차서 배를 더 받을 수 없습니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomFull {
    /// 방 전체의 정원입니다.
    pub capacity: usize,
}

impl Sim {
    /// 배가 없는 방을 만들고 맵에 적힌 건물을 세웁니다. 수치는 `Tuning::validate`를 통과한 것이어야 합니다.
    pub fn new(tuning: Tuning) -> Sim {
        let mut world = World::default();
        for (index, site) in tuning.map.structures.iter().enumerate() {
            let id = StructureId(index as u32 + 1);
            let structure = Structure {
                id,
                kind: site.kind,
                team: site.team,
                pos: site.pos,
                hp: tuning.structures.of(site.kind).max_hp,
                lane: site.lane,
                ready_at: 0,
                target: None,
            };
            world.structures.insert(id, structure);
        }
        Sim {
            tuning,
            tick: 0,
            world,
            sight: Sight::default(),
            next_ship: 1,
            next_creep: 1,
            outcome: None,
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

    /// Tick 하나를 실행합니다. 대전이 끝난 뒤에는 아무 일도 하지 않습니다.
    ///
    /// 가라앉았던 배가 다시 나타나고, 항구가 NPC 배를 내보내고, 배와 NPC 배가 움직이고, 진영별 시야를 정하고,
    /// 쏠 수 있는 무기가 쏘고, 닿을 때가 된 포탄이 피해를 주고, 보상과 수입을 주고, 사라진 것을 지우고, 승패를 판정합니다.
    /// 무기는 이번 Tick의 피해가 들어가기 전의 상태를 보고 쏘므로, 같은 Tick에 가라앉는 배도 그 Tick의 사격은 마칩니다.
    pub fn step(&mut self, inputs: &PlayerInputs) {
        if self.outcome.is_some() {
            return;
        }
        self.tick += 1;
        let (world, tuning) = (&mut self.world, &self.tuning);
        ship::respawn(world, tuning, self.tick);
        creep::spawn(world, tuning, self.tick, &mut self.next_creep);
        ship::move_ships(world, inputs, tuning);
        creep::move_creeps(world, &self.sight, tuning);
        self.sight = vision::look(world, tuning);
        weapon::fire_all(world, &self.sight, tuning, self.tick);
        let kills = weapon::land_impacts(world, tuning, self.tick);
        economy::pay_bounties(world, tuning, &kills);
        economy::pay_income(world, tuning, self.tick);
        creep::remove_sunk(world);
        self.outcome = match structures::remove_destroyed(world).as_slice() {
            [] => None,
            [loser] => Some(Outcome::Victory(loser.enemy())),
            _ => Some(Outcome::Draw),
        };
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

    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// 번호가 작은 배부터입니다. 가라앉은 배도 들어 있습니다.
    pub fn ships(&self) -> impl Iterator<Item = &Ship> {
        self.world.ships.values()
    }

    pub fn ship(&self, id: ShipId) -> Option<&Ship> {
        self.world.ships.get(&id)
    }

    /// 번호가 작은 NPC 배부터입니다. 가라앉은 NPC 배는 없습니다.
    pub fn creeps(&self) -> impl Iterator<Item = &Creep> {
        self.world.creeps.values()
    }

    pub fn creep(&self, id: CreepId) -> Option<&Creep> {
        self.world.creeps.get(&id)
    }

    /// 번호가 작은 건물부터입니다. 파괴된 건물은 없습니다.
    pub fn structures(&self) -> impl Iterator<Item = &Structure> {
        self.world.structures.values()
    }

    pub fn structure(&self, id: StructureId) -> Option<&Structure> {
        self.world.structures.get(&id)
    }

    #[cfg(test)]
    pub(crate) fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}
