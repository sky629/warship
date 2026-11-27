//! 입장할 때 한 번 보내는, 접속 중에 바뀌지 않는 정보입니다.

use serde::{Deserialize, Serialize};
use warship_rules::{ShipId, Sim, StructureKind, StructureSpec, WeaponSpec};

use crate::messages::{WireTeam, WireVec2};
use crate::snapshot::WireStructureKind;

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
    /// 선체 표입니다. 스냅샷의 선체 번호가 이 목록의 자리입니다.
    pub hulls: Vec<WireHull>,
    /// 무기 표입니다. 스냅샷의 무기 번호가 이 목록의 자리이고, 상점이 이 무기를 모두 팝니다.
    pub weapons: Vec<WireWeapon>,
    /// 스킬 표입니다. 스냅샷의 스킬 목록과 같은 순서이고, 선체마다 이 가운데 일부를 씁니다.
    pub skills: Vec<WireSkillSpec>,
    pub tower: WireStructureSpec,
    pub harbor: WireStructureSpec,
    pub base: WireStructureSpec,
    /// NPC 배의 종류입니다. 스냅샷의 NPC 배의 종류 번호가 이 목록의 자리입니다.
    pub creeps: Vec<WireCreepSpec>,
    /// 라인마다 NPC 배가 지나가는 지점입니다.
    pub lanes: Vec<Vec<WireVec2>>,
    /// 배가 지나가지 못하는 섬입니다.
    pub islands: Vec<WireIsland>,
}

/// 원 모양의 섬입니다.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireIsland {
    pub center: WireVec2,
    pub radius: f32,
}

impl Welcome {
    pub fn structure(&self, kind: WireStructureKind) -> &WireStructureSpec {
        match kind {
            WireStructureKind::Tower => &self.tower,
            WireStructureKind::Harbor => &self.harbor,
            WireStructureKind::Base => &self.base,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireHull {
    pub name: String,
    pub price: u32,
    pub max_hp: u32,
    /// 무기를 달 수 있는 칸의 수입니다.
    pub mounts: u8,
    /// 이 선체의 배가 쓸 수 있는 스킬의 번호입니다. 클라이언트가 이 순서로 단축키에 놓습니다.
    pub skills: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireSkillSpec {
    pub name: String,
    pub cooldown_ticks: u32,
    pub max_rank: u8,
}

/// 클라이언트가 사거리와 날아가는 포탄을 그리고 상점의 목록을 보여 주는 데 쓰는 무기의 수치입니다.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireWeapon {
    pub name: String,
    /// 상점에서 사는 가격입니다. 건물과 NPC 배의 포에서는 뜻이 없습니다.
    pub price: u32,
    pub range: f32,
    pub cooldown_ticks: u32,
    pub shot_speed: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireStructureSpec {
    pub max_hp: u32,
    /// 건물의 포입니다. 쏘지 않는 건물에는 없습니다.
    pub weapon: Option<WireWeapon>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireCreepSpec {
    pub name: String,
    pub max_hp: u32,
    pub weapon: WireWeapon,
}

fn wire_weapon(spec: &WeaponSpec) -> WireWeapon {
    WireWeapon {
        name: spec.name.to_string(),
        price: spec.price,
        range: spec.range,
        cooldown_ticks: spec.cooldown_ticks,
        shot_speed: spec.shot_speed,
    }
}

fn wire_structure_spec(spec: &StructureSpec) -> WireStructureSpec {
    WireStructureSpec {
        max_hp: spec.max_hp,
        weapon: spec.weapon.as_ref().map(wire_weapon),
    }
}

impl Welcome {
    /// `ship`이 방에 없으면 `None`입니다.
    pub fn from_sim(sim: &Sim, room: &str, ship: ShipId) -> Option<Welcome> {
        let tuning = sim.tuning();
        let structure = |kind| wire_structure_spec(tuning.structures.of(kind));
        let creeps = tuning.creeps.iter().map(|creep| WireCreepSpec {
            name: creep.name.to_string(),
            max_hp: creep.max_hp,
            weapon: wire_weapon(&creep.weapon),
        });
        let lane = |points: &Vec<warship_rules::Vec2>| {
            let wire = points.iter().map(|point| WireVec2::new(point.x, point.y));
            wire.collect()
        };
        Some(Welcome {
            room: room.to_string(),
            ship: ship.0,
            team: sim.ship(ship)?.team.into(),
            tick_hz: tuning.tick_hz,
            map_width: tuning.map.width,
            map_height: tuning.map.height,
            hulls: tuning
                .hulls
                .iter()
                .map(|hull| WireHull {
                    name: hull.name.to_string(),
                    price: hull.price,
                    max_hp: hull.max_hp,
                    mounts: hull.mounts as u8,
                    skills: hull.skills.iter().map(|skill| skill.0).collect(),
                })
                .collect(),
            skills: tuning
                .skills
                .iter()
                .map(|skill| WireSkillSpec {
                    name: skill.name.to_string(),
                    cooldown_ticks: skill.cooldown_ticks,
                    max_rank: skill.max_rank,
                })
                .collect(),
            weapons: tuning.weapons.iter().map(wire_weapon).collect(),
            tower: structure(StructureKind::Tower),
            harbor: structure(StructureKind::Harbor),
            base: structure(StructureKind::Base),
            creeps: creeps.collect(),
            lanes: tuning.map.lanes.iter().map(lane).collect(),
            islands: tuning
                .map
                .islands
                .iter()
                .map(|island| WireIsland {
                    center: WireVec2::new(island.center.x, island.center.y),
                    radius: island.radius,
                })
                .collect(),
        })
    }
}
