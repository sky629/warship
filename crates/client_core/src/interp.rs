//! 스냅샷을 보관하고, 조금 이전의 방 모습을 보간해서 화면에 그릴 `Frame`으로 만듭니다.
//!
//! 스냅샷은 Tick마다 오지만 화면은 그보다 자주 그립니다. 가장 최근 스냅샷을 그대로 그리면 배가 끊겨서 움직이므로,
//! `DELAY_SECS` 이전의 시각을 감싸는 두 스냅샷 사이를 보간합니다.
//!
//! 포탄은 스냅샷에 없습니다. 무기가 마지막으로 쏜 Tick과 표적에서, 지금 날아가고 있는 포탄의 위치를 계산합니다.

use std::collections::VecDeque;

use warship_protocol::{
    Snapshot, Welcome, WireItem, WireOutcome, WireRefusal, WireScore, WireShot, WireStructureKind,
    WireTarget, WireTeam, WireVec2, WireWeapon,
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
    /// 방금 터진 산탄과 폭탄의 범위입니다.
    pub blasts: Vec<BlastView>,
    /// 방에 있는 모든 배의 점수입니다.
    pub scores: Vec<WireScore>,
    /// 대전이 시작한 뒤 지난 시간(초)입니다.
    pub secs: f32,
    /// 이 접속이 조종하는 배의 상태입니다.
    pub me: Option<MyShip>,
    pub outcome: Option<WireOutcome>,
}

/// 이 접속이 조종하는 배에 대해 화면이 글자로 보여 주는 것입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct MyShip {
    pub gold: u32,
    pub hp: u32,
    pub max_hp: u32,
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
    /// 소모품을 다시 쓸 수 있을 때까지 남은 시간(초)입니다. 쓸 수 있으면 0입니다.
    pub supply_secs: f32,
    /// 서버가 마지막으로 처리한 명령의 순서 번호입니다.
    pub command_seq: u32,
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
    /// 모양의 크기의 배율입니다. 최대 HP가 큰 선체가 큽니다.
    pub size: f32,
    /// `Welcome::hulls`의 자리입니다.
    pub hull: u16,
    /// 최대 HP에 대한 비율입니다(0 이상 1 이하).
    pub hp: f32,
    pub sunk: bool,
    /// 적의 감속 스킬의 효과를 받고 있는지입니다.
    pub slowed: bool,
    /// 은신하고 있는지입니다.
    pub cloaked: bool,
    /// 불이 붙어 있는지입니다.
    pub burning: bool,
    /// 이 접속이 조종하는 배입니다.
    pub mine: bool,
    /// 달린 무기의 사거리입니다. 같은 사거리는 한 번만 있습니다.
    pub ranges: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreepView {
    pub id: u32,
    /// `Welcome::creeps`의 자리입니다.
    pub kind: u16,
    pub team: WireTeam,
    pub pos: Vec2,
    /// 모양의 크기의 배율입니다. 최대 HP가 큰 종류가 큽니다.
    pub size: f32,
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

/// 포탄 한 발이 누구의 어느 칸에서 언제 나왔는지입니다. 같은 포탄을 프레임 사이에서 알아보는 데 씁니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Volley {
    pub shooter: WireTarget,
    /// 배의 칸의 번호입니다. NPC 배와 건물은 0입니다.
    pub slot: u8,
    /// 쏜 Tick입니다.
    pub fired: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShotView {
    pub volley: Volley,
    /// 쏜 무기의 이름입니다. 화면이 무기마다 다른 모형과 발사음을 고르는 데 씁니다.
    pub weapon: String,
    pub pos: Vec2,
    pub team: WireTeam,
    /// 포탄의 종류입니다. 화면이 종류마다 다르게 그립니다.
    pub kind: WireShot,
    /// 수면 위의 높이입니다. 폭탄은 포물선을 그리며 날아갑니다.
    pub lift: f32,
    /// 포탄의 크기의 배율입니다. 피해가 큰 무기의 포탄이 큽니다.
    pub size: f32,
}

/// 산탄이나 폭탄이 터진 범위입니다. 터진 뒤 잠깐 동안 보입니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlastView {
    pub volley: Volley,
    pub pos: Vec2,
    pub team: WireTeam,
    pub radius: f32,
    /// 터진 뒤 지난 시간의 비율입니다. 0은 터진 순간이고, 1에 가까울수록 사라질 때입니다.
    pub age: f32,
}

/// 포탄 하나가 화면에 보이는 모습입니다. 날아가는 동안은 포탄이고, 터진 뒤에는 범위입니다.
enum Seen {
    Shot(ShotView),
    Blast(BlastView),
}

/// 터진 범위가 보이는 Tick 수입니다.
const BLAST_TICKS: f32 = 8.0;
/// 폭탄이 가장 높이 올라가는 높이입니다.
const BOMB_ARC: f32 = 2.5;
/// 포탄이 날아가는 높이입니다.
const SHOT_HEIGHT: f32 = 0.5;

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
    // 무기가 마지막으로 쏜 포탄이 아직 날아가고 있으면 그 포탄입니다.
    // 폭탄은 쏠 때 표적이 있던 자리(`aim`)로 포물선을 그리며 날아가고, 나머지는 표적을 따라갑니다.
    let shot = |shooter: WireTarget,
                slot: u8,
                from: Vec2,
                team: WireTeam,
                weapon: &WireWeapon,
                target: Option<WireTarget>,
                aim: Option<WireVec2>,
                ready_at: u64| {
        let falls = matches!(weapon.shot, WireShot::Bomb { .. });
        let to = match aim.filter(|_| falls) {
            Some(aim) => Vec2::new(aim.x, aim.y),
            None => position_of(target?)?,
        };
        let fired = ready_at.checked_sub(u64::from(weapon.cooldown_ticks))?;
        let volley = Volley {
            shooter,
            slot,
            fired,
        };
        let fired = fired as f32;
        let per_tick = weapon.shot_speed / welcome.tick_hz as f32;
        let flight = (from.distance(to) / per_tick).ceil().max(1.0);
        let progress = (tick - fired) / flight;
        let arc = if falls {
            (progress * std::f32::consts::PI).sin() * BOMB_ARC
        } else {
            0.0
        };
        if (0.0..1.0).contains(&progress) {
            return Some(Seen::Shot(ShotView {
                volley,
                weapon: weapon.name.clone(),
                pos: lerp(from, to, progress),
                team,
                kind: weapon.shot,
                lift: SHOT_HEIGHT + arc,
                size: (0.6 + weapon.damage as f32 / 40.0).min(2.5),
            }));
        }
        let radius = match weapon.shot {
            WireShot::Scatter { radius } | WireShot::Bomb { radius } => radius,
            WireShot::Shell | WireShot::Fire { .. } => return None,
        };
        let age = (tick - fired - flight) / BLAST_TICKS;
        (0.0..1.0).contains(&age).then_some(Seen::Blast(BlastView {
            volley,
            pos: to,
            team,
            radius,
            age,
        }))
    };

    let mut seen = Vec::new();
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
            seen.extend(shot(
                WireTarget::Ship(ship.id),
                mount.slot,
                pos,
                ship.team,
                weapon,
                mount.target,
                mount.aim,
                mount.ready_at,
            ));
        }
        // 배는 선체의 최대 HP가 클수록 크게 그립니다. 첫 번째 선체의 크기가 1입니다.
        let hull_hp = |hull: u16| welcome.hulls.get(usize::from(hull)).map(|hull| hull.max_hp);
        let ratio = hull_hp(ship.hull).zip(hull_hp(0));
        let size = ratio.map_or(1.0, |(hp, first)| (hp as f32 / first.max(1) as f32).sqrt());
        ships.push(ShipView {
            id: ship.id,
            team: ship.team,
            pos,
            size,
            hull: ship.hull,
            hp: ship.hp as f32 / ship.max_hp.max(1) as f32,
            sunk: ship.respawn_at.is_some(),
            slowed: ship.slowed,
            cloaked: ship.cloaked,
            burning: ship.burning,
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
        let weapon = &spec.weapon;
        seen.extend(shot(
            WireTarget::Creep(creep.id),
            0,
            pos,
            creep.team,
            weapon,
            creep.target,
            None,
            creep.ready_at,
        ));
        // NPC 배는 최대 HP가 클수록 크게 그립니다. 첫 번째 종류의 크기가 1입니다.
        let smallest = welcome
            .creeps
            .first()
            .map_or(spec.max_hp, |first| first.max_hp);
        creeps.push(CreepView {
            id: creep.id,
            kind: u16::from(creep.kind),
            team: creep.team,
            pos,
            size: (spec.max_hp as f32 / smallest as f32).sqrt(),
            hp: creep.hp as f32 / spec.max_hp as f32,
        });
    }
    let mut structures = Vec::new();
    for site in &next.structures {
        let pos = Vec2::new(site.x, site.y);
        let spec = welcome.structure(site.kind);
        if let Some(weapon) = &spec.weapon {
            seen.extend(shot(
                WireTarget::Structure(site.id),
                0,
                pos,
                site.team,
                weapon,
                site.target,
                None,
                site.ready_at,
            ));
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
            hp: ship.hp,
            max_hp: ship.max_hp,
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
            supply_secs: secs_until(own.supply_ready_at),
            command_seq: own.last_command_seq,
            refusal: own.refusal,
        })
    });
    let (mut shots, mut blasts) = (Vec::new(), Vec::new());
    for seen in seen {
        match seen {
            Seen::Shot(shot) => shots.push(shot),
            Seen::Blast(blast) => blasts.push(blast),
        }
    }
    Frame {
        ships,
        creeps,
        structures,
        shots,
        blasts,
        scores: next.scores.clone(),
        secs: tick / welcome.tick_hz.max(1) as f32,
        me,
        outcome: next.outcome,
    }
}
