//! `Sim`은 방 하나의 게임 상태 전체이며, 규칙 크레이트의 공개 진입점입니다.

use crate::command::{self, Command};
use crate::creep::{self, Creep};
use crate::economy;
use crate::ids::{CreepId, ShipId, StructureId, Team};
use crate::input::PlayerInputs;
use crate::note::Note;
use crate::ship::{self, Ship};
use crate::shop;
use crate::structures::{self, Structure};
use crate::trade;
use crate::tuning::Tuning;
use crate::vision::{self, Sight};
use crate::weapon::{self, Target};
use crate::world::World;

/// 방 하나의 게임입니다. 바깥 코드는 `&Sim`으로 상태를 읽고, `join`, `remove_ship`, `hand_over`, `command`, `step`으로만 상태를 바꿉니다.
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
    /// 오래된 피해와 보상의 기록을 잊고, 가라앉았던 배가 다시 나타나고, 항구가 NPC 배를 내보내고, 빈 자리에 중립 몬스터가 나오고, 고용한 용병이 나오고, 배와 NPC 배가 움직이고, 진영별 시야를 정하고,
    /// 쏠 수 있는 무기가 쏘고, 닿을 때가 된 포탄이 피해를 주고, 보상과 수입을 주고, 교역의 화물을 싣고 내리고, 상점의 배를 수리하고, 선원 장비가 배를 수리하고, 사라진 것을 지우고, 승패를 판정합니다.
    /// 무기는 이번 Tick의 피해가 들어가기 전의 상태를 보고 쏘므로, 같은 Tick에 가라앉는 배도 그 Tick의 사격은 마칩니다.
    pub fn step(&mut self, inputs: &PlayerInputs) {
        if self.outcome.is_some() {
            return;
        }
        self.tick += 1;
        let (world, tuning) = (&mut self.world, &self.tuning);
        world.forget_notes(self.tick);
        ship::respawn(world, tuning, self.tick);
        creep::spawn(world, tuning, self.tick, &mut self.next_creep);
        creep::spawn_camps(world, tuning, self.tick, &mut self.next_creep);
        creep::spawn_hired(world, tuning, &mut self.next_creep);
        ship::move_ships(world, inputs, tuning, self.tick);
        creep::move_creeps(world, &self.sight, tuning);
        self.sight = vision::look(world, tuning, self.tick);
        weapon::fire_all(world, &self.sight, tuning, self.tick);
        let kills = weapon::land_impacts(world, &self.sight, tuning, self.tick);
        economy::pay_bounties(world, tuning, &kills, self.tick);
        economy::pay_income(world, tuning, self.tick);
        trade::haul(world, tuning, self.tick);
        shop::repair(world, tuning, self.tick);
        ship::crew_repairs(world, tuning, self.tick);
        creep::remove_sunk(world);
        self.outcome = match structures::remove_destroyed(world).as_slice() {
            [] => None,
            [loser] => Some(Outcome::Victory(loser.enemy())),
            _ => Some(Outcome::Draw),
        };
    }

    /// 배의 명령 하나를 Tick과 Tick 사이에 처리합니다. 결과는 `Ship::last_command_seq`와 `Ship::refusal`에 남습니다.
    /// 없는 배의 명령, 이미 처리한 순서 번호의 명령, 대전이 끝난 뒤의 명령은 무시합니다.
    pub fn command(&mut self, ship: ShipId, seq: u32, command: Command) {
        if self.outcome.is_none() {
            command::apply(&mut self.world, &self.tuning, self.tick, ship, seq, command);
        }
    }

    /// 배를 조종하는 쪽이 바뀌었습니다. 새 조종자는 입력과 명령의 순서 번호를 1부터 다시 세므로, 배가 기억하는 번호와 거절한 이유를 지웁니다.
    /// 배는 새 입력을 받을 때까지 멈춥니다. 없는 배는 무시합니다.
    pub fn hand_over(&mut self, ship: ShipId) {
        if let Some(ship) = self.world.ships.get_mut(&ship) {
            ship.last_input_seq = 0;
            ship.last_command_seq = 0;
            ship.refusal = None;
            ship.route.clear();
        }
    }

    /// 이 배가 자기 진영의 상점(서 있는 본진이나 항구 가까이)에 있는지입니다. 없는 배는 `false`입니다.
    pub fn at_home(&self, ship: ShipId) -> bool {
        let ship = self.world.ships.get(&ship);
        ship.is_some_and(|ship| shop::at_home(&self.world, &self.tuning, ship))
    }

    /// 이 배가 수리를 받는 자리(자기 진영의 서 있는 본진 가까이)에 있는지입니다. 없는 배는 `false`입니다.
    pub fn at_dock(&self, ship: ShipId) -> bool {
        let ship = self.world.ships.get(&ship);
        ship.is_some_and(|ship| shop::at_dock(&self.world, &self.tuning, ship))
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

    /// 최근 `NOTE_TICKS` 동안의 피해와 골드의 보상입니다. 오래된 것이 앞입니다.
    pub fn notes(&self) -> &[Note] {
        &self.world.notes
    }

    #[cfg(test)]
    pub(crate) fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}
