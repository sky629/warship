//! NPC 배입니다. 라인의 NPC 배는 항구에서 일정에 따라 나오고, 라인의 지점을 차례로 지나 적의 본진으로 갑니다.
//! 쏠 수 있는 적이 사거리 안에 있으면 멈춰서 싸우고, 없으면 다시 나아갑니다. 사격은 배의 무기와 같은 규칙입니다.
//! 중립 몬스터(`Team::WILD`)는 맵에 적힌 자리(`MapSpec::camps`)를 지키고, 가라앉으면 정해진 시간 뒤에 다시 나옵니다.

use crate::ids::{CreepId, Team};
use crate::math::Vec2;
use crate::structures::StructureKind;
use crate::tuning::Tuning;
use crate::vision::Sight;
use crate::weapon::{Target, can_hit};
use crate::world::World;

#[derive(Clone, Debug, PartialEq)]
pub struct Creep {
    pub id: CreepId,
    pub team: Team,
    /// `Tuning::creeps`의 행 번호입니다.
    pub kind: usize,
    /// 따라가는 라인의 번호입니다. 자리를 지키는 중립 몬스터에는 없습니다.
    pub lane: Option<usize>,
    /// 지금 향하는 지점이 길에서 몇 번째인지입니다(`MapSpec::path_point`).
    pub leg: usize,
    pub pos: Vec2,
    pub hp: u32,
    /// 이 Tick부터 다시 쏠 수 있습니다.
    pub ready_at: u64,
    /// 마지막으로 쏜 표적입니다. 쏠 수 있는데 쏠 적이 없으면 `None`이 됩니다.
    pub target: Option<Target>,
}

/// 이번 Tick에 나올 차례인 NPC 배를 서 있는 항구마다 한 척씩 내보냅니다.
pub(crate) fn spawn(world: &mut World, tuning: &Tuning, tick: u64, next_id: &mut u32) {
    for wave in &tuning.waves {
        let Some(since_first) = tick.checked_sub(wave.first_tick) else {
            continue;
        };
        // 무리 안에서 몇 번째 배가 나올 Tick인지 확인합니다.
        let into_wave = since_first % u64::from(wave.period_ticks);
        let due =
            (0..u64::from(wave.count)).any(|nth| nth * u64::from(wave.gap_ticks) == into_wave);
        if !due {
            continue;
        }
        let harbors: Vec<(Team, usize, Vec2)> = world
            .structures
            .values()
            .filter(|structure| structure.kind == StructureKind::Harbor && structure.hp > 0)
            .filter_map(|harbor| Some((harbor.team, harbor.lane?, harbor.pos)))
            .collect();
        for (team, lane, pos) in harbors {
            let id = CreepId(*next_id);
            *next_id += 1;
            let creep = Creep {
                id,
                team,
                kind: wave.creep,
                lane: Some(lane),
                leg: 0,
                pos,
                hp: tuning.creeps[wave.creep].max_hp,
                ready_at: 0,
                target: None,
            };
            world.creeps.insert(id, creep);
        }
    }
}

/// 용병이 서로 겹치지 않게 항구의 남북으로 벌려 놓는 간격입니다.
const HIRED_GAP: f32 = 0.8;

/// 고용한 용병을 그 라인의 항구에서 내보냅니다. 그사이에 항구가 파괴되었으면 나오지 않습니다.
pub(crate) fn spawn_hired(world: &mut World, tuning: &Tuning, next_id: &mut u32) {
    let spec = tuning.hire;
    for (team, lane) in std::mem::take(&mut world.hires) {
        let harbor = world.structures.values().find(|site| {
            site.kind == StructureKind::Harbor
                && site.hp > 0
                && site.team == team
                && site.lane == Some(lane)
        });
        let Some(pos) = harbor.map(|harbor| harbor.pos) else {
            continue;
        };
        for nth in 0..spec.count {
            let id = CreepId(*next_id);
            *next_id += 1;
            let offset = (f32::from(nth) - f32::from(spec.count - 1) / 2.0) * HIRED_GAP;
            let creep = Creep {
                id,
                team,
                kind: spec.creep,
                lane: Some(lane),
                leg: 0,
                pos: pos + Vec2::new(0.0, offset),
                hp: tuning.creeps[spec.creep].max_hp,
                ready_at: 0,
                target: None,
            };
            world.creeps.insert(id, creep);
        }
    }
}

/// 중립 몬스터의 자리 하나의 상태입니다.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CampState {
    /// 지금 그 자리를 지키는 중립 몬스터입니다.
    pub creep: Option<CreepId>,
    /// 자리가 비어 있을 때, 이 Tick에 다음 중립 몬스터가 나옵니다.
    pub ready_at: u64,
}

/// 중립 몬스터가 가라앉은 자리에 다시 나올 Tick을 정하고, 그 Tick이 된 자리에 중립 몬스터를 내보냅니다.
pub(crate) fn spawn_camps(world: &mut World, tuning: &Tuning, tick: u64, next_id: &mut u32) {
    world
        .camps
        .resize(tuning.map.camps.len(), CampState::default());
    for (index, camp) in tuning.map.camps.iter().enumerate() {
        let state = world.camps[index];
        if let Some(id) = state.creep {
            if !world.creeps.contains_key(&id) {
                world.camps[index] = CampState {
                    creep: None,
                    ready_at: tick + u64::from(camp.respawn_ticks),
                };
            }
            continue;
        }
        if tick < state.ready_at {
            continue;
        }
        let id = CreepId(*next_id);
        *next_id += 1;
        let creep = Creep {
            id,
            team: Team::WILD,
            kind: camp.creep,
            lane: None,
            leg: 0,
            pos: camp.pos,
            hp: tuning.creeps[camp.creep].max_hp,
            ready_at: 0,
            target: None,
        };
        world.creeps.insert(id, creep);
        world.camps[index].creep = Some(id);
    }
}

/// NPC 배를 길의 다음 지점 쪽으로 한 Tick만큼 움직입니다. 지난 Tick에 보인 적 가운데 쏠 수 있는 적이 사거리 안에 있으면 움직이지 않습니다.
pub(crate) fn move_creeps(world: &mut World, sight: &Sight, tuning: &Tuning) {
    for creep in world.creeps.values_mut() {
        let spec = &tuning.creeps[creep.kind];
        let fighting = sight
            .enemies_seen_by(creep.team)
            .iter()
            .any(|&(target, pos)| {
                can_hit(&spec.weapon, target) && creep.pos.distance(pos) <= spec.weapon.range
            });
        if fighting {
            continue;
        }
        let point = creep
            .lane
            .and_then(|lane| tuning.map.path_point(creep.team, lane, creep.leg));
        let Some(point) = point else {
            continue;
        };
        let step = spec.speed / tuning.tick_hz as f32;
        let offset = point - creep.pos;
        let distance = offset.length();
        if distance <= step {
            creep.pos = point;
            creep.leg += 1;
        } else {
            creep.pos = creep.pos + offset * (step / distance);
        }
    }
}

/// HP가 0이 된 NPC 배를 지웁니다.
pub(crate) fn remove_sunk(world: &mut World) {
    world.creeps.retain(|_, creep| creep.hp > 0);
}
