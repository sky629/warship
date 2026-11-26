//! 타워입니다. 제자리에서 사거리 안의 보이는 적을 쏘고, HP가 0이 되면 맵에서 사라집니다.

use crate::ids::{Team, TowerId};
use crate::math::Vec2;
use crate::weapon::Target;
use crate::world::World;

#[derive(Clone, Debug, PartialEq)]
pub struct Tower {
    pub id: TowerId,
    pub team: Team,
    pub pos: Vec2,
    pub hp: u32,
    /// 이 Tick부터 다시 쏠 수 있습니다.
    pub ready_at: u64,
    /// 마지막으로 쏜 표적입니다. 쏠 수 있는데 쏠 적이 없으면 `None`이 됩니다.
    pub target: Option<Target>,
}

/// HP가 0이 된 타워를 맵에서 지웁니다.
pub(crate) fn remove_destroyed(world: &mut World) {
    world.towers.retain(|_, tower| tower.hp > 0);
}
