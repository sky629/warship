//! 라인을 따라가는 NPC 배입니다. 항구에서 일정에 따라 나오고, 라인의 지점을 차례로 지나 적의 본진으로 갑니다.
//! 쏠 수 있는 적이 사거리 안에 있으면 멈춰서 싸우고, 없으면 다시 나아갑니다. 사격은 배의 무기와 같은 규칙입니다.

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
    pub lane: usize,
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
                lane,
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
        let Some(point) = tuning.map.path_point(creep.team, creep.lane, creep.leg) else {
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
