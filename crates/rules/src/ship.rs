//! 플레이어가 조종하는 배와 그 이동입니다.

use crate::ids::{ShipId, Team, WeaponId};
use crate::input::PlayerInputs;
use crate::math::Vec2;
use crate::tuning::Tuning;
use crate::weapon::Target;
use crate::world::World;

/// 배 한 척이 달 수 있는 무기의 수입니다.
pub const MOUNTS: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub struct Ship {
    pub id: ShipId,
    pub team: Team,
    pub pos: Vec2,
    pub hp: u32,
    pub life: Life,
    /// 배가 향하는 지점입니다. 없으면 멈춰 있습니다.
    pub destination: Option<Vec2>,
    /// 무기를 다는 칸입니다. 빈 칸은 `None`입니다.
    pub mounts: [Option<Mount>; MOUNTS],
    /// 마지막으로 처리한 입력의 순서 번호입니다.
    pub last_input_seq: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Life {
    Afloat,
    /// HP가 0이 된 배입니다. 움직이지 않고, 쏘지 않고, 표적이 되지 않고, 시야를 주지 않습니다.
    Sunk,
}

/// 칸 하나에 달린 무기입니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mount {
    pub weapon: WeaponId,
    /// 이 Tick부터 다시 쏠 수 있습니다.
    pub ready_at: u64,
    /// 마지막으로 쏜 표적입니다. 쏠 수 있는데 쏠 적이 없으면 `None`이 됩니다.
    pub target: Option<Target>,
}

impl Ship {
    pub(crate) fn new(id: ShipId, team: Team, pos: Vec2, tuning: &Tuning) -> Ship {
        let mut mounts = [None; MOUNTS];
        for (mount, &weapon) in mounts.iter_mut().zip(&tuning.ship.start_weapons) {
            *mount = Some(Mount {
                weapon,
                ready_at: 0,
                target: None,
            });
        }
        Ship {
            id,
            team,
            pos,
            hp: tuning.ship.max_hp,
            life: Life::Afloat,
            destination: None,
            mounts,
            last_input_seq: 0,
        }
    }

    pub fn is_afloat(&self) -> bool {
        self.life == Life::Afloat
    }
}

/// 입력을 받은 배의 목적지를 바꾸고, 떠 있는 배를 목적지 쪽으로 한 Tick만큼 움직입니다.
/// 한 Tick에 갈 수 있는 거리 안에 목적지가 있으면 목적지에 정확히 섭니다.
pub(crate) fn move_ships(world: &mut World, inputs: &PlayerInputs, tuning: &Tuning) {
    let step = tuning.ship.speed / tuning.tick_hz as f32;
    for ship in world.ships.values_mut() {
        if let Some(input) = inputs.get(&ship.id) {
            ship.last_input_seq = input.seq;
            ship.destination = input.destination.map(|point| tuning.map.clamp(point.get()));
        }
        if !ship.is_afloat() {
            continue;
        }
        let Some(destination) = ship.destination else {
            continue;
        };
        let offset = destination - ship.pos;
        let distance = offset.length();
        ship.pos = if distance <= step {
            destination
        } else {
            ship.pos + offset * (step / distance)
        };
    }
}
