//! 플레이어가 한 Tick 동안 요청하는 조작입니다. 의도만 담고, 위치나 피해량은 담지 않습니다.
//!
//! 클라이언트가 보낸 값은 믿을 수 없으므로 `Waypoint::new`가 값을 검사합니다.
//! 이 생성자를 거친 값만 규칙에 들어오므로, 규칙은 입력을 다시 검사하지 않습니다.

use std::collections::BTreeMap;

use crate::ids::ShipId;
use crate::math::Vec2;

/// 배마다 한 Tick 분량의 입력입니다. 목록에 없는 배는 이전에 받은 목적지를 그대로 따릅니다.
/// 번호 순서로 순회하므로 같은 입력은 항상 같은 순서로 처리됩니다.
pub type PlayerInputs = BTreeMap<ShipId, PlayerInput>;

/// 한 플레이어의 한 Tick 분량 조작입니다.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerInput {
    /// 클라이언트가 붙인 순서 번호입니다. 규칙은 이 입력을 처리한 Tick에 `Ship::last_input_seq`에 복사합니다.
    pub seq: u32,
    /// 배가 향할 지점입니다. 없으면 배가 멈춥니다.
    pub destination: Option<Waypoint>,
}

/// 플레이어가 클릭한 이동 지점입니다. 좌표는 항상 유한한 값입니다. 맵 밖의 지점은 배가 움직일 때 맵 안으로 고쳐집니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Waypoint(Vec2);

impl Waypoint {
    /// 유한하지 않은 값(NaN, 무한대)은 받지 않습니다.
    pub fn new(x: f32, y: f32) -> Option<Waypoint> {
        let point = Vec2::new(x, y);
        point.is_finite().then_some(Waypoint(point))
    }

    pub fn get(self) -> Vec2 {
        self.0
    }
}
