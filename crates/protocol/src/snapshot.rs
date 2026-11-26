//! Tick마다 보내는 방의 상태입니다. 진영마다 따로 만들고, 받는 진영에 보이지 않는 적의 배는 싣지 않습니다.
//!
//! 포탄은 싣지 않습니다. 클라이언트는 무기가 마지막으로 쏜 Tick(`ready_at`에서 쿨다운을 뺀 값)과 표적에서 날아가는 포탄을 그립니다.

use serde::{Deserialize, Serialize};
use warship_rules::{Ship, Sim, Target, Team, Tower};

use crate::messages::WireTeam;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// 서버의 Tick 번호입니다.
    pub tick: u64,
    /// 번호가 작은 배부터입니다. 받는 진영의 배는 모두 있고, 적의 배는 이번 Tick에 보인 것만 있습니다.
    pub ships: Vec<WireShip>,
    /// 번호가 작은 타워부터입니다. 타워는 두 진영에 모두 보이고, 파괴된 타워는 없습니다.
    pub towers: Vec<WireTower>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireShip {
    pub id: u32,
    pub team: WireTeam,
    pub x: f32,
    pub y: f32,
    pub hp: u32,
    pub sunk: bool,
    /// 무기가 달린 칸만 있습니다.
    pub mounts: Vec<WireMount>,
    /// 서버가 마지막으로 처리한 이 배의 입력 순서 번호입니다.
    pub last_input_seq: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireMount {
    /// 무기가 달린 칸의 번호입니다(0부터).
    pub slot: u8,
    /// `Welcome::weapons`의 자리입니다.
    pub weapon: u16,
    /// 마지막으로 쏜 표적입니다.
    pub target: Option<WireTarget>,
    /// 이 Tick부터 다시 쏠 수 있습니다. 표적이 있으면 마지막 사격은 이 값에서 쿨다운을 뺀 Tick에 있었습니다.
    pub ready_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireTarget {
    Ship(u32),
    Tower(u32),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireTower {
    pub id: u32,
    pub team: WireTeam,
    pub x: f32,
    pub y: f32,
    pub hp: u32,
    pub target: Option<WireTarget>,
    pub ready_at: u64,
}

impl Snapshot {
    /// `team`에 보이는 방의 상태입니다. 게임 상태를 전송 형식으로 바꾸는 곳은 이 함수 하나입니다.
    pub fn from_sim(sim: &Sim, team: Team) -> Snapshot {
        let shown = |ship: &&Ship| ship.team == team || sim.sees(team, Target::Ship(ship.id));
        Snapshot {
            tick: sim.tick(),
            ships: sim.ships().filter(shown).map(wire_ship).collect(),
            towers: sim.towers().map(wire_tower).collect(),
        }
    }
}

fn wire_ship(ship: &Ship) -> WireShip {
    let mounts = ship.mounts.iter().enumerate().filter_map(|(slot, mount)| {
        let mount = mount.as_ref()?;
        Some(WireMount {
            slot: slot as u8,
            weapon: mount.weapon.0,
            target: mount.target.map(wire_target),
            ready_at: mount.ready_at,
        })
    });
    WireShip {
        id: ship.id.0,
        team: ship.team.into(),
        x: ship.pos.x,
        y: ship.pos.y,
        hp: ship.hp,
        sunk: !ship.is_afloat(),
        mounts: mounts.collect(),
        last_input_seq: ship.last_input_seq,
    }
}

fn wire_tower(tower: &Tower) -> WireTower {
    WireTower {
        id: tower.id.0,
        team: tower.team.into(),
        x: tower.pos.x,
        y: tower.pos.y,
        hp: tower.hp,
        target: tower.target.map(wire_target),
        ready_at: tower.ready_at,
    }
}

fn wire_target(target: Target) -> WireTarget {
    match target {
        Target::Ship(id) => WireTarget::Ship(id.0),
        Target::Tower(id) => WireTarget::Tower(id.0),
    }
}
