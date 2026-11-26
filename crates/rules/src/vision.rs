//! 진영별 시야입니다. 자동 사격은 여기서 보이는 적만 쏩니다. 진영별 스냅샷도 같은 결과를 읽습니다.

use crate::ids::Team;
use crate::math::Vec2;
use crate::tuning::Tuning;
use crate::weapon::Target;
use crate::world::World;

/// 한 Tick에 진영마다 보이는 적입니다. 기본값은 아무도 보이지 않는 상태입니다.
#[derive(Default)]
pub(crate) struct Sight {
    /// `Team::index` 자리의 진영에 보이는 적입니다. 순서는 `World::targets`의 순서입니다.
    seen: [Vec<(Target, Vec2)>; 2],
}

impl Sight {
    /// `team`에 보이는 적과 그 위치입니다.
    pub fn enemies_seen_by(&self, team: Team) -> &[(Target, Vec2)] {
        &self.seen[team.index()]
    }

    pub fn sees(&self, team: Team, target: Target) -> bool {
        self.enemies_seen_by(team)
            .iter()
            .any(|(seen, _)| *seen == target)
    }
}

/// 개체가 자기 진영에 주는 시야 거리입니다.
fn vision_of(world: &World, tuning: &Tuning, watcher: Target) -> f32 {
    match watcher {
        Target::Ship(_) => tuning.ship.vision,
        Target::Creep(id) => world
            .creeps
            .get(&id)
            .map_or(0.0, |creep| tuning.creeps[creep.kind].vision),
        Target::Structure(id) => world
            .structures
            .get(&id)
            .map_or(0.0, |structure| tuning.structures.of(structure.kind).vision),
    }
}

/// 적은 아군의 떠 있는 배, NPC 배, 서 있는 건물 가운데 하나의 시야 거리 안에 있으면 보입니다.
pub(crate) fn look(world: &World, tuning: &Tuning) -> Sight {
    let mut seen = [Vec::new(), Vec::new()];
    for team in Team::ALL {
        let eyes: Vec<(Vec2, f32)> = world
            .targets()
            .filter(|(_, owner, _)| *owner == team)
            .map(|(watcher, _, pos)| (pos, vision_of(world, tuning, watcher)))
            .collect();
        seen[team.index()] = world
            .targets()
            .filter(|(_, owner, _)| owner.is_hostile_to(team))
            .filter(|(_, _, pos)| {
                eyes.iter()
                    .any(|(eye, vision)| eye.distance(*pos) <= *vision)
            })
            .map(|(target, _, pos)| (target, pos))
            .collect();
    }
    Sight { seen }
}
