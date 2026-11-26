//! 입장할 때 한 번 보내는, 접속 중에 바뀌지 않는 정보입니다.

use serde::{Deserialize, Serialize};
use warship_rules::{ShipId, Sim};

use crate::messages::WireTeam;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Welcome {
    /// 해석된 방 코드입니다.
    pub room: String,
    /// 이 접속이 조종하는 배의 번호입니다.
    pub ship: u32,
    pub team: WireTeam,
    pub tick_hz: u32,
    pub map_width: f32,
    pub map_height: f32,
    pub ship_max_hp: u32,
    pub tower_max_hp: u32,
    /// 무기 표입니다. 스냅샷의 무기 번호가 이 목록의 자리입니다.
    pub weapons: Vec<WireWeapon>,
    /// 타워의 포입니다.
    pub tower_weapon: WireWeapon,
}

/// 클라이언트가 사거리와 날아가는 포탄을 그리는 데 쓰는 무기의 수치입니다.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireWeapon {
    pub name: String,
    pub range: f32,
    pub cooldown_ticks: u32,
    pub shot_speed: f32,
}

impl Welcome {
    /// `ship`이 방에 없으면 `None`입니다.
    pub fn from_sim(sim: &Sim, room: &str, ship: ShipId) -> Option<Welcome> {
        let tuning = sim.tuning();
        let weapon = |spec: &warship_rules::WeaponSpec| WireWeapon {
            name: spec.name.to_string(),
            range: spec.range,
            cooldown_ticks: spec.cooldown_ticks,
            shot_speed: spec.shot_speed,
        };
        Some(Welcome {
            room: room.to_string(),
            ship: ship.0,
            team: sim.ship(ship)?.team.into(),
            tick_hz: tuning.tick_hz,
            map_width: tuning.map.width,
            map_height: tuning.map.height,
            ship_max_hp: tuning.ship.max_hp,
            tower_max_hp: tuning.tower.max_hp,
            weapons: tuning.weapons.iter().map(weapon).collect(),
            tower_weapon: weapon(&tuning.tower.weapon),
        })
    }
}
