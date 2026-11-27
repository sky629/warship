//! 스냅샷을 보관하고, 조금 이전의 방 모습을 보간해서 화면에 그릴 `Frame`으로 만듭니다.
//!
//! 스냅샷은 Tick마다 오지만 화면은 그보다 자주 그립니다. 가장 최근 스냅샷을 그대로 그리면 배가 끊겨서 움직이므로,
//! `DELAY_SECS` 이전의 시각을 감싸는 두 스냅샷 사이를 보간합니다.
//!
//! 포탄은 스냅샷에 없습니다. 무기가 마지막으로 쏜 Tick과 표적에서, 지금 날아가고 있는 포탄의 위치를 계산합니다.

use std::collections::VecDeque;

use warship_protocol::{
    Snapshot, Welcome, WireItem, WireOutcome, WireRefusal, WireStructureKind, WireTarget, WireTeam,
    WireWeapon,
};
use warship_rules::Vec2;

/// 화면이 보여 주는 방의 모습은 이 시간만큼 이전의 것입니다.
const DELAY_SECS: f64 = 0.1;
/// 이 시간보다 오래된 스냅샷은 버립니다.
const KEEP_SECS: f64 = 1.0;

/// 화면에 그릴 한 순간의 방입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub ships: Vec<ShipView>,
    pub creeps: Vec<CreepView>,
    pub structures: Vec<StructureView>,
    /// 날아가고 있는 포탄입니다.
    pub shots: Vec<ShotView>,
    /// 이 접속이 조종하는 배의 상태입니다.
    pub me: Option<MyShip>,
    pub outcome: Option<WireOutcome>,
}

/// 이 접속이 조종하는 배에 대해 화면이 글자로 보여 주는 것입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct MyShip {
    pub gold: u32,
    /// 가라앉아 있으면, 다시 나타날 때까지 남은 시간(초)입니다.
    pub respawn_secs: Option<f32>,
    /// `Welcome::hulls`의 자리입니다.
    pub hull: u16,
    pub level: u8,
    pub xp: u32,
    pub skill_points: u8,
    /// 선체의 칸마다 달린 아이템입니다. 빈 칸은 `None`입니다.
    pub mounts: Vec<Option<WireItem>>,
    /// `Welcome::skills`와 같은 순서입니다.
    pub skills: Vec<SkillView>,
    /// 마지막 명령이 거절되었으면 그 이유입니다.
    pub refusal: Option<WireRefusal>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkillView {
    pub rank: u8,
    /// 다시 쓸 수 있을 때까지 남은 시간(초)입니다. 쓸 수 있으면 0입니다.
    pub ready_secs: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShipView {
    pub id: u32,
    pub team: WireTeam,
    pub pos: Vec2,
    /// `Welcome::hulls`의 자리입니다.
    pub hull: u16,
    /// 최대 HP에 대한 비율입니다(0 이상 1 이하).
    pub hp: f32,
    pub sunk: bool,
    /// 적의 감속 스킬의 효과를 받고 있는지입니다.
    pub slowed: bool,
    /// 이 접속이 조종하는 배입니다.
    pub mine: bool,
    /// 달린 무기의 사거리입니다. 같은 사거리는 한 번만 있습니다.
    pub ranges: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreepView {
    pub id: u32,
    pub team: WireTeam,
    pub pos: Vec2,
    /// 최대 HP에 대한 비율입니다(0 이상 1 이하).
    pub hp: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructureView {
    pub id: u32,
    pub kind: WireStructureKind,
    pub team: WireTeam,
    pub pos: Vec2,
    /// 최대 HP에 대한 비율입니다(0 이상 1 이하).
    pub hp: f32,
    /// 건물의 포의 사거리입니다. 쏘지 않는 건물에는 없습니다.
    pub range: Option<f32>,
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

/// `next`의 자리와, 같은 번호의 개체가 `prev`에 있으면 그 자리 사이의 `alpha` 지점입니다.
/// `next`에만 있는 개체(막 보이기 시작한 적)는 `next`의 자리입니다.
fn between(prev: Option<(f32, f32)>, next: (f32, f32), alpha: f32) -> Vec2 {
    let to = Vec2::new(next.0, next.1);
    prev.map_or(to, |(x, y)| lerp(Vec2::new(x, y), to, alpha))
}

/// 두 스냅샷 사이의 `alpha` 지점(0은 `prev`, 1은 `next`)의 방입니다. 위치만 보간하고, 나머지는 `next`의 값입니다.
fn build(welcome: &Welcome, prev: &Snapshot, next: &Snapshot, alpha: f32) -> Frame {
    let tick = prev.tick as f32 + (next.tick - prev.tick) as f32 * alpha;
    let ship_pos = |id: u32| {
        let ship = next.ships.iter().find(|ship| ship.id == id)?;
        let old = prev.ships.iter().find(|old| old.id == id);
        Some(between(
            old.map(|old| (old.x, old.y)),
            (ship.x, ship.y),
            alpha,
        ))
    };
    let creep_pos = |id: u32| {
        let creep = next.creeps.iter().find(|creep| creep.id == id)?;
        let old = prev.creeps.iter().find(|old| old.id == id);
        Some(between(
            old.map(|old| (old.x, old.y)),
            (creep.x, creep.y),
            alpha,
        ))
    };
    let position_of = |target: WireTarget| match target {
        WireTarget::Ship(id) => ship_pos(id),
        WireTarget::Creep(id) => creep_pos(id),
        WireTarget::Structure(id) => {
            let site = next.structures.iter().find(|site| site.id == id)?;
            Some(Vec2::new(site.x, site.y))
        }
    };
    // 무기가 마지막으로 쏜 포탄이 아직 날아가고 있으면 그 위치입니다.
    let shot = |from: Vec2, weapon: &WireWeapon, target: Option<WireTarget>, ready_at: u64| {
        let to = position_of(target?)?;
        let fired = ready_at.checked_sub(u64::from(weapon.cooldown_ticks))? as f32;
        let per_tick = weapon.shot_speed / welcome.tick_hz as f32;
        let flight = (from.distance(to) / per_tick).ceil().max(1.0);
        let progress = (tick - fired) / flight;
        (0.0..1.0)
            .contains(&progress)
            .then(|| lerp(from, to, progress))
    };

    let mut shots = Vec::new();
    let mut ships = Vec::new();
    for ship in &next.ships {
        let pos = ship_pos(ship.id).unwrap_or(Vec2::new(ship.x, ship.y));
        let mut ranges: Vec<f32> = Vec::new();
        for mount in &ship.mounts {
            // 장비는 쏘지 않으므로 사거리와 포탄이 없습니다.
            let weapon = mount.item.weapon();
            let Some(weapon) = weapon.and_then(|id| welcome.weapons.get(usize::from(id))) else {
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
            hull: ship.hull,
            hp: ship.hp as f32 / ship.max_hp.max(1) as f32,
            sunk: ship.respawn_at.is_some(),
            slowed: ship.slowed,
            mine: ship.id == welcome.ship,
            ranges,
        });
    }
    let mut creeps = Vec::new();
    for creep in &next.creeps {
        let Some(spec) = welcome.creeps.get(usize::from(creep.kind)) else {
            continue;
        };
        let pos = creep_pos(creep.id).unwrap_or(Vec2::new(creep.x, creep.y));
        if let Some(pos) = shot(pos, &spec.weapon, creep.target, creep.ready_at) {
            shots.push(ShotView {
                pos,
                team: creep.team,
            });
        }
        creeps.push(CreepView {
            id: creep.id,
            team: creep.team,
            pos,
            hp: creep.hp as f32 / spec.max_hp as f32,
        });
    }
    let mut structures = Vec::new();
    for site in &next.structures {
        let pos = Vec2::new(site.x, site.y);
        let spec = welcome.structure(site.kind);
        if let Some(weapon) = &spec.weapon
            && let Some(pos) = shot(pos, weapon, site.target, site.ready_at)
        {
            shots.push(ShotView {
                pos,
                team: site.team,
            });
        }
        structures.push(StructureView {
            id: site.id,
            kind: site.kind,
            team: site.team,
            pos,
            hp: site.hp as f32 / spec.max_hp as f32,
            range: spec.weapon.as_ref().map(|weapon| weapon.range),
        });
    }
    let secs_until = |at: u64| (at as f32 - tick).max(0.0) / welcome.tick_hz as f32;
    let mine = next.ships.iter().find(|ship| ship.id == welcome.ship);
    let me = mine.and_then(|ship| {
        let own = ship.own.as_ref()?;
        let slots = welcome.hulls.get(usize::from(ship.hull))?.mounts;
        let item_in = |slot: u8| {
            let mount = ship.mounts.iter().find(|mount| mount.slot == slot);
            mount.map(|mount| mount.item)
        };
        Some(MyShip {
            gold: own.gold,
            respawn_secs: ship.respawn_at.map(secs_until),
            hull: ship.hull,
            level: ship.level,
            xp: own.xp,
            skill_points: own.skill_points,
            mounts: (0..slots).map(item_in).collect(),
            skills: own
                .skills
                .iter()
                .map(|skill| SkillView {
                    rank: skill.rank,
                    ready_secs: secs_until(skill.ready_at),
                })
                .collect(),
            refusal: own.refusal,
        })
    });
    Frame {
        ships,
        creeps,
        structures,
        shots,
        me,
        outcome: next.outcome,
    }
}
