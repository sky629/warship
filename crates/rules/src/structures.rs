//! 건물입니다. 타워와 본진은 사거리 안의 보이는 적을 쏘고, 항구는 라인의 NPC 배를 내보냅니다.
//! HP가 0이 된 건물은 맵에서 사라지고, 본진이 사라진 진영은 집니다.

use crate::ids::{StructureId, Team};
use crate::math::Vec2;
use crate::weapon::Target;
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureKind {
    Tower,
    /// 라인 하나의 NPC 배를 내보내는 건물입니다. 파괴되면 그 진영은 그 라인에 NPC 배를 더 내보내지 못합니다.
    Harbor,
    /// 진영마다 하나입니다. 파괴되면 그 진영이 집니다.
    Base,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Structure {
    pub id: StructureId,
    pub kind: StructureKind,
    pub team: Team,
    pub pos: Vec2,
    pub hp: u32,
    /// 항구가 맡은 라인의 번호입니다. 항구가 아니면 없습니다.
    pub lane: Option<usize>,
    /// 이 Tick부터 다시 쏠 수 있습니다.
    pub ready_at: u64,
    /// 마지막으로 쏜 표적입니다. 쏠 수 있는데 쏠 적이 없으면 `None`이 됩니다.
    pub target: Option<Target>,
}

impl StructureKind {
    /// 건물이 차지하는 원의 반지름입니다. 배는 이 안에 서지 않습니다.
    pub fn footprint(self) -> f32 {
        match self {
            StructureKind::Tower => 1.0,
            StructureKind::Harbor => 1.7,
            StructureKind::Base => 2.2,
        }
    }
}

/// `p`가 서 있는 건물이 차지하는 원에서 `room`보다 가까우면, 그 원에서 `room`만큼 떨어진 둘레에서 `from`에 가장 가까운 점입니다. 아니면 `p`입니다.
/// `room`은 배의 길이의 절반입니다. 배는 가는 길에 건물을 지날 수 있지만, 건물과 겹쳐서 서지는 않습니다.
pub(crate) fn beside(world: &World, p: Vec2, from: Vec2, room: f32) -> Vec2 {
    let mut standing = world.structures.values().filter(|site| site.hp > 0);
    let reach = |kind: StructureKind| kind.footprint() + room;
    let Some(site) = standing.find(|site| site.pos.distance(p) < reach(site.kind)) else {
        return p;
    };
    let offset = from - site.pos;
    let distance = offset.length();
    // 배가 건물의 한가운데에 있으면 동쪽으로 옮깁니다.
    let direction = if distance > 0.0 {
        offset * (1.0 / distance)
    } else {
        Vec2::new(1.0, 0.0)
    };
    site.pos + direction * reach(site.kind)
}

/// HP가 0이 된 건물을 맵에서 지우고, 본진을 잃은 진영을 돌려줍니다.
pub(crate) fn remove_destroyed(world: &mut World) -> Vec<Team> {
    let fallen = world
        .structures
        .values()
        .filter(|structure| structure.hp == 0 && structure.kind == StructureKind::Base)
        .map(|structure| structure.team)
        .collect();
    world.structures.retain(|_, structure| structure.hp > 0);
    fallen
}
