//! 진영별 시야입니다. 자동 사격은 여기서 보이는 적만 쏩니다. 진영별 스냅샷도 같은 결과를 읽습니다.

use crate::ids::Team;
use crate::math::Vec2;
use crate::tuning::Tuning;
use crate::weapon::Target;
use crate::world::World;

/// 한 Tick에 진영마다 보이는 적입니다.
pub(crate) struct Sight {
    /// `Team::index` 자리의 진영에 보이는 적입니다. 순서는 `World::targets`의 순서입니다.
    seen: [Vec<(Target, Vec2)>; 2],
}

impl Sight {
    /// `team`에 보이는 적과 그 위치입니다.
    pub fn enemies_seen_by(&self, team: Team) -> &[(Target, Vec2)] {
        &self.seen[team.index()]
    }
}

/// 적은 아군의 떠 있는 배나 서 있는 타워 가운데 하나의 시야 거리 안에 있으면 보입니다.
pub(crate) fn look(world: &World, tuning: &Tuning) -> Sight {
    let mut seen = [Vec::new(), Vec::new()];
    for team in Team::ALL {
        let eyes: Vec<(Vec2, f32)> = world
            .targets()
            .filter(|(_, owner, _)| *owner == team)
            .map(|(watcher, _, pos)| {
                let vision = match watcher {
                    Target::Ship(_) => tuning.ship.vision,
                    Target::Tower(_) => tuning.tower.vision,
                };
                (pos, vision)
            })
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
