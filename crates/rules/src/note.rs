//! 화면이 숫자로 보여 주는 일입니다. 포탄이 준 피해와, 배가 보상으로 받은 골드입니다.
//!
//! 규칙은 이 일을 `NOTE_TICKS` 동안 기억합니다. 스냅샷이 그동안 계속 싣고, 화면은 그동안 숫자를 띄웁니다.
//! 1초마다 들어오는 수입과 판매의 환불은 적지 않습니다.

use crate::ids::ShipId;
use crate::math::Vec2;
use crate::weapon::Target;

/// 일어난 일을 기억하는 Tick 수입니다.
pub const NOTE_TICKS: u64 = 20;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    /// 방 안에서 일어난 순서의 번호입니다. 같은 일을 스냅샷 사이에서 알아보는 데 씁니다.
    pub id: u32,
    /// 일어난 Tick입니다.
    pub tick: u64,
    /// 일어난 자리입니다. 피해는 맞은 개체의 자리이고, 보상은 가라앉은 개체의 자리이며, 교역의 보상은 배의 자리입니다.
    pub pos: Vec2,
    pub amount: u32,
    pub kind: NoteKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoteKind {
    /// `target`이 피해를 받았습니다. `by`는 포탄을 쏜 플레이어의 배이고, NPC 배와 건물이 쏜 포탄에는 없습니다.
    /// `seen`은 맞은 순간에 `target`이 진영(`Team::index`의 순서)마다 적으로 보이고 있었는지입니다.
    Hit {
        by: Option<ShipId>,
        target: Target,
        seen: [bool; 2],
    },
    /// `ship`이 골드를 받았습니다.
    Gold { ship: ShipId },
}
