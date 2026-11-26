//! 스냅샷을 보관하고, 조금 이전의 방 모습을 보간해서 화면에 그릴 `Frame`으로 만듭니다.
//!
//! 스냅샷은 Tick마다 오지만 화면은 그보다 자주 그립니다. 가장 최근 스냅샷을 그대로 그리면 배가 끊겨서 움직이므로,
//! `DELAY_SECS` 이전의 시각을 감싸는 두 스냅샷 사이를 보간합니다.
//!
//! 포탄은 스냅샷에 없습니다. 무기가 마지막으로 쏜 Tick과 표적에서, 지금 날아가고 있는 포탄의 위치를 계산합니다.

use std::collections::VecDeque;

use warship_protocol::{Snapshot, Welcome, WireShip, WireTarget, WireTeam, WireWeapon};
use warship_rules::Vec2;

/// 화면이 보여 주는 방의 모습은 이 시간만큼 이전의 것입니다.
const DELAY_SECS: f64 = 0.1;
/// 이 시간보다 오래된 스냅샷은 버립니다.
const KEEP_SECS: f64 = 1.0;

/// 화면에 그릴 한 순간의 방입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub ships: Vec<ShipView>,
    pub towers: Vec<TowerView>,
    /// 날아가고 있는 포탄입니다.
    pub shots: Vec<ShotView>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShipView {
    pub id: u32,
    pub team: WireTeam,
    pub pos: Vec2,
    /// 최대 HP에 대한 비율입니다(0 이상 1 이하).
    pub hp: f32,
    pub sunk: bool,
    /// 이 접속이 조종하는 배입니다.
    pub mine: bool,
    /// 달린 무기의 사거리입니다. 같은 사거리는 한 번만 있습니다.
    pub ranges: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TowerView {
    pub id: u32,
    pub team: WireTeam,
    pub pos: Vec2,
    /// 최대 HP에 대한 비율입니다(0 이상 1 이하).
    pub hp: f32,
    pub range: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShotView {
    pub pos: Vec2,
    pub team: WireTeam,
}

/// 받은 스냅샷과 받은 시각(초)입니다. 오래된 것이 앞입니다.
#[derive(Default)]
pub(crate) struct History {
    snapshots: VecDeque<(f64, Snapshot)>,
}

impl History {
    pub fn push(&mut self, now: f64, snapshot: Snapshot) {
        self.snapshots.push_back((now, snapshot));
        while self.snapshots.len() > 2 && self.snapshots[1].0 < now - KEEP_SECS {
            self.snapshots.pop_front();
        }
    }

    pub fn frame(&self, welcome: &Welcome, now: f64) -> Option<Frame> {
        let shown_at = now - DELAY_SECS;
        // 보여 줄 시각 이후에 받은 첫 스냅샷이 `next`이고, 그 바로 앞이 `prev`입니다.
        let after = self.snapshots.iter().position(|(at, _)| *at >= shown_at);
        let (prev, next) = match after {
            Some(0) => (&self.snapshots[0], &self.snapshots[0]),
            Some(index) => (&self.snapshots[index - 1], &self.snapshots[index]),
            None => {
                let last = self.snapshots.back()?;
                (last, last)
            }
        };
        let span = next.0 - prev.0;
        let alpha = if span > 0.0 {
            ((shown_at - prev.0) / span).clamp(0.0, 1.0) as f32
        } else {
            1.0
        };
        Some(build(welcome, &prev.1, &next.1, alpha))
    }
}

fn lerp(a: Vec2, b: Vec2, alpha: f32) -> Vec2 {
    a + (b - a) * alpha
}

/// 두 스냅샷 사이의 `alpha` 지점(0은 `prev`, 1은 `next`)의 방입니다. 위치만 보간하고, 나머지는 `next`의 값입니다.
/// `next`에만 있는 배(막 보이기 시작한 적)는 `next`의 자리에 그립니다.
fn build(welcome: &Welcome, prev: &Snapshot, next: &Snapshot, alpha: f32) -> Frame {
    let tick = prev.tick as f32 + (next.tick - prev.tick) as f32 * alpha;
    let ship_pos = |ship: &WireShip| {
        let to = Vec2::new(ship.x, ship.y);
        let from = prev.ships.iter().find(|old| old.id == ship.id);
        from.map_or(to, |old| lerp(Vec2::new(old.x, old.y), to, alpha))
    };
    let position_of = |target: WireTarget| match target {
        WireTarget::Ship(id) => next.ships.iter().find(|ship| ship.id == id).map(ship_pos),
        WireTarget::Tower(id) => {
            let tower = next.towers.iter().find(|tower| tower.id == id)?;
            Some(Vec2::new(tower.x, tower.y))
        }
    };
    let per_tick = |weapon: &WireWeapon| weapon.shot_speed / welcome.tick_hz as f32;
    // 무기가 마지막으로 쏜 포탄이 아직 날아가고 있으면 그 위치입니다.
    let shot = |from: Vec2, weapon: &WireWeapon, target: Option<WireTarget>, ready_at: u64| {
        let to = position_of(target?)?;
        let fired = ready_at.checked_sub(u64::from(weapon.cooldown_ticks))? as f32;
        let flight = (from.distance(to) / per_tick(weapon)).ceil().max(1.0);
        let progress = (tick - fired) / flight;
        (0.0..1.0)
            .contains(&progress)
            .then(|| lerp(from, to, progress))
    };

    let mut shots = Vec::new();
    let mut ships = Vec::new();
    for ship in &next.ships {
        let pos = ship_pos(ship);
        let mut ranges: Vec<f32> = Vec::new();
        for mount in &ship.mounts {
            let Some(weapon) = welcome.weapons.get(usize::from(mount.weapon)) else {
                continue;
            };
            if !ranges.contains(&weapon.range) {
                ranges.push(weapon.range);
            }
            if let Some(pos) = shot(pos, weapon, mount.target, mount.ready_at) {
                shots.push(ShotView {
                    pos,
                    team: ship.team,
                });
            }
        }
        ships.push(ShipView {
            id: ship.id,
            team: ship.team,
            pos,
            hp: ship.hp as f32 / welcome.ship_max_hp as f32,
            sunk: ship.sunk,
            mine: ship.id == welcome.ship,
            ranges,
        });
    }
    let mut towers = Vec::new();
    for tower in &next.towers {
        let pos = Vec2::new(tower.x, tower.y);
        let weapon = &welcome.tower_weapon;
        if let Some(pos) = shot(pos, weapon, tower.target, tower.ready_at) {
            shots.push(ShotView {
                pos,
                team: tower.team,
            });
        }
        towers.push(TowerView {
            id: tower.id,
            team: tower.team,
            pos,
            hp: tower.hp as f32 / welcome.tower_max_hp as f32,
            range: weapon.range,
        });
    }
    Frame {
        ships,
        towers,
        shots,
    }
}
