//! Tick마다 보내는 방의 상태입니다. 진영마다 따로 만들고, 받는 진영에 보이지 않는 적의 배와 NPC 배는 싣지 않습니다.
//!
//! 최근의 피해와 골드의 보상(`notes`)은 받는 진영의 배가 주었거나 받은 것만 싣습니다.
//!
//! 포탄은 싣지 않습니다. 클라이언트는 무기가 마지막으로 쏜 Tick(`ready_at`에서 쿨다운을 뺀 값)과 표적에서 날아가는 포탄을 그립니다.

use serde::{Deserialize, Serialize};
use warship_rules::{
    Creep, Item, Life, Note, NoteKind, Outcome, Refusal, Ship, Sim, Structure, StructureKind,
    Target, Team, Tuning,
};

use crate::messages::{WireTeam, WireVec2};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// 서버의 Tick 번호입니다. 대전이 끝나면 늘지 않습니다.
    pub tick: u64,
    /// 번호가 작은 배부터입니다. 받는 진영의 배는 모두 있고, 적의 배는 이번 Tick에 보인 것만 있습니다.
    pub ships: Vec<WireShip>,
    /// 번호가 작은 NPC 배부터입니다. 받는 진영의 것은 모두 있고, 적의 것은 이번 Tick에 보인 것만 있습니다.
    pub creeps: Vec<WireCreep>,
    /// 번호가 작은 건물부터입니다. 건물은 두 진영에 모두 보이고, 파괴된 건물은 없습니다.
    pub structures: Vec<WireStructure>,
    /// 방에 있는 모든 배의 점수입니다. 번호가 작은 배부터이고, 보이지 않는 적의 배도 있습니다.
    pub scores: Vec<WireScore>,
    /// 최근 1초 동안 받는 진영의 배가 준 피해, 받은 피해, 받은 골드의 보상입니다. 오래된 것이 앞입니다.
    /// 화면이 숫자로 띄웁니다.
    pub notes: Vec<WireNote>,
    /// 대전의 결과입니다. 끝나기 전에는 없습니다.
    pub outcome: Option<WireOutcome>,
}

/// 피해나 골드의 보상 하나입니다.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireNote {
    /// 방 안에서 일어난 순서의 번호입니다. 같은 일을 스냅샷 사이에서 알아봅니다.
    pub id: u32,
    /// 일어난 Tick입니다.
    pub tick: u64,
    pub pos: WireVec2,
    pub amount: u32,
    pub kind: WireNoteKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireNoteKind {
    /// `target`이 피해를 받았습니다. `by`는 포탄을 쏜 플레이어의 배이고, NPC 배와 건물이 쏜 포탄에는 없습니다.
    Hit { by: Option<u32>, target: WireTarget },
    /// 이 번호의 배가 골드를 받았습니다.
    Gold { ship: u32 },
}

/// 배 하나의 점수입니다. 점수판에 보여 줍니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireScore {
    pub ship: u32,
    pub team: WireTeam,
    pub level: u8,
    /// 가라앉힌 적의 배의 수입니다.
    pub kills: u32,
    /// 가라앉은 횟수입니다.
    pub deaths: u32,
    /// 가라앉힌 NPC 배의 수입니다.
    pub creep_kills: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireShip {
    pub id: u32,
    pub team: WireTeam,
    pub x: f32,
    pub y: f32,
    /// `Welcome::hulls`의 자리입니다.
    pub hull: u16,
    pub hp: u32,
    /// 선체의 최대 HP에 장비가 올린 HP를 더한 값입니다.
    pub max_hp: u32,
    pub level: u8,
    /// 가라앉은 배가 다시 나타나는 Tick입니다. 떠 있는 배에는 없습니다.
    pub respawn_at: Option<u64>,
    /// 적의 감속 스킬의 효과를 받고 있는지입니다.
    pub slowed: bool,
    /// 은신하고 있는지입니다. 은신한 배는 자기 진영과, 그 배를 본 건물의 진영에만 보냅니다.
    pub cloaked: bool,
    /// 불이 붙어 있는지입니다.
    pub burning: bool,
    /// 적에게 보이지 않는 정보입니다. 받는 진영의 배에만 있습니다.
    pub own: Option<WireOwn>,
    /// 아이템이 달린 칸만 있습니다.
    pub mounts: Vec<WireMount>,
    /// 서버가 마지막으로 처리한 이 배의 입력 순서 번호입니다.
    pub last_input_seq: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireOwn {
    pub gold: u32,
    pub xp: u32,
    /// 아직 스킬에 넣지 않은 스킬 포인트의 수입니다.
    pub skill_points: u8,
    /// `Welcome::skills`와 같은 순서입니다.
    pub skills: Vec<WireSkill>,
    /// 이 Tick부터 소모품을 다시 쓸 수 있습니다.
    pub supply_ready_at: u64,
    /// 서버가 마지막으로 처리한 이 배의 명령 순서 번호입니다.
    pub last_command_seq: u32,
    /// 그 명령을 거절한 이유입니다. 실행했으면 없습니다.
    pub refusal: Option<WireRefusal>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireSkill {
    /// 넣은 스킬 포인트의 수입니다.
    pub rank: u8,
    /// 이 Tick부터 다시 쓸 수 있습니다.
    pub ready_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireRefusal {
    Sunk,
    NoShopInRange,
    UnknownItem,
    NotEnoughGold,
    NoFreeMount,
    EmptyMount,
    MountsInUse,
    SameHull,
    NoSkillPoint,
    MaxRank,
    NotLearned,
    OnCooldown,
    NotOnThisHull,
    NotForSale,
    MissingParts,
    NoBaseInRange,
    NotUsable,
    NotATrader,
    CarryingFreight,
    NoHarbor,
    NotSoldHere,
}

impl From<Refusal> for WireRefusal {
    fn from(refusal: Refusal) -> WireRefusal {
        match refusal {
            Refusal::Sunk => WireRefusal::Sunk,
            Refusal::NoShopInRange => WireRefusal::NoShopInRange,
            Refusal::UnknownItem => WireRefusal::UnknownItem,
            Refusal::NotEnoughGold => WireRefusal::NotEnoughGold,
            Refusal::NoFreeMount => WireRefusal::NoFreeMount,
            Refusal::EmptyMount => WireRefusal::EmptyMount,
            Refusal::MountsInUse => WireRefusal::MountsInUse,
            Refusal::SameHull => WireRefusal::SameHull,
            Refusal::NoSkillPoint => WireRefusal::NoSkillPoint,
            Refusal::MaxRank => WireRefusal::MaxRank,
            Refusal::NotLearned => WireRefusal::NotLearned,
            Refusal::OnCooldown => WireRefusal::OnCooldown,
            Refusal::NotOnThisHull => WireRefusal::NotOnThisHull,
            Refusal::NotForSale => WireRefusal::NotForSale,
            Refusal::MissingParts => WireRefusal::MissingParts,
            Refusal::NoBaseInRange => WireRefusal::NoBaseInRange,
            Refusal::NotUsable => WireRefusal::NotUsable,
            Refusal::NotATrader => WireRefusal::NotATrader,
            Refusal::CarryingFreight => WireRefusal::CarryingFreight,
            Refusal::NoHarbor => WireRefusal::NoHarbor,
            Refusal::NotSoldHere => WireRefusal::NotSoldHere,
        }
    }
}

/// 칸에 달린 아이템입니다. 번호는 `Welcome::weapons`, `Welcome::gear`, `Welcome::supplies`, `Welcome::trades`의 자리입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireItem {
    Weapon(u16),
    Gear(u16),
    /// 한 번 쓰면 사라지는 소모품입니다.
    Supply(u16),
    /// 교역의 계약입니다.
    Contract(u16),
    /// 교역항에서 실은 화물입니다.
    Cargo(u16),
}

impl WireItem {
    /// 무기이면 `Welcome::weapons`의 자리입니다.
    pub fn weapon(self) -> Option<u16> {
        match self {
            WireItem::Weapon(weapon) => Some(weapon),
            WireItem::Gear(_)
            | WireItem::Supply(_)
            | WireItem::Contract(_)
            | WireItem::Cargo(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireMount {
    /// 아이템이 달린 칸의 번호입니다(0부터).
    pub slot: u8,
    pub item: WireItem,
    /// 무기가 마지막으로 쏜 표적입니다.
    pub target: Option<WireTarget>,
    /// 마지막으로 쏠 때 표적이 있던 자리입니다. 폭탄은 이 자리에 떨어집니다.
    pub aim: Option<WireVec2>,
    /// 이 Tick부터 다시 쏠 수 있습니다. 표적이 있으면 마지막 사격은 이 값에서 쿨다운을 뺀 Tick에 있었습니다.
    pub ready_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireTarget {
    Ship(u32),
    Creep(u32),
    Structure(u32),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireCreep {
    pub id: u32,
    pub team: WireTeam,
    /// `Welcome::creeps`의 자리입니다.
    pub kind: u8,
    pub x: f32,
    pub y: f32,
    pub hp: u32,
    pub target: Option<WireTarget>,
    pub ready_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireStructureKind {
    Tower,
    Harbor,
    Base,
}

impl From<StructureKind> for WireStructureKind {
    fn from(kind: StructureKind) -> WireStructureKind {
        match kind {
            StructureKind::Tower => WireStructureKind::Tower,
            StructureKind::Harbor => WireStructureKind::Harbor,
            StructureKind::Base => WireStructureKind::Base,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireStructure {
    pub id: u32,
    pub kind: WireStructureKind,
    pub team: WireTeam,
    pub x: f32,
    pub y: f32,
    pub hp: u32,
    pub target: Option<WireTarget>,
    pub ready_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireOutcome {
    Victory(WireTeam),
    Draw,
}

impl Snapshot {
    /// `team`에 보이는 방의 상태입니다. 게임 상태를 전송 형식으로 바꾸는 곳은 이 함수 하나입니다.
    pub fn from_sim(sim: &Sim, team: Team) -> Snapshot {
        let ship_shown = |ship: &&Ship| ship.team == team || sim.sees(team, Target::Ship(ship.id));
        let creep_shown =
            |creep: &&Creep| creep.team == team || sim.sees(team, Target::Creep(creep.id));
        Snapshot {
            tick: sim.tick(),
            ships: sim
                .ships()
                .filter(ship_shown)
                .map(|ship| wire_ship(ship, team, sim.tuning(), sim.tick()))
                .collect(),
            creeps: sim.creeps().filter(creep_shown).map(wire_creep).collect(),
            structures: sim.structures().map(wire_structure).collect(),
            scores: sim
                .ships()
                .map(|ship| WireScore {
                    ship: ship.id.0,
                    team: ship.team.into(),
                    level: ship.level(sim.tuning()) as u8,
                    kills: ship.kills,
                    deaths: ship.deaths,
                    creep_kills: ship.creep_kills,
                })
                .collect(),
            notes: sim
                .notes()
                .iter()
                .filter_map(|note| wire_note(sim, team, note))
                .collect(),
            outcome: sim.outcome().map(|outcome| match outcome {
                Outcome::Victory(winner) => WireOutcome::Victory(winner.into()),
                Outcome::Draw => WireOutcome::Draw,
            }),
        }
    }
}

/// `team`의 배가 주었거나 받은 피해, 받은 골드의 보상만 전송 형식으로 바꿉니다.
/// 준 피해는 맞은 순간에 그 개체가 이 진영에 보이고 있었을 때만 싣습니다. 보이지 않는 적의 자리를 알려 주지 않기 위해서입니다.
fn wire_note(sim: &Sim, team: Team, note: &Note) -> Option<WireNote> {
    let ours = |ship| sim.ship(ship).is_some_and(|ship| ship.team == team);
    let kind = match note.kind {
        NoteKind::Hit { by, target, seen } => {
            let taken = matches!(target, Target::Ship(ship) if ours(ship));
            let dealt = by.is_some_and(ours) && seen[team.index()];
            (taken || dealt).then_some(WireNoteKind::Hit {
                by: by.map(|ship| ship.0),
                target: wire_target(target),
            })?
        }
        NoteKind::Gold { ship } => ours(ship).then_some(WireNoteKind::Gold { ship: ship.0 })?,
    };
    Some(WireNote {
        id: note.id,
        tick: note.tick,
        pos: WireVec2::new(note.pos.x, note.pos.y),
        amount: note.amount,
        kind,
    })
}

fn wire_ship(ship: &Ship, viewer: Team, tuning: &Tuning, tick: u64) -> WireShip {
    let mounts = ship.mounts.iter().enumerate().filter_map(|(slot, mount)| {
        let mount = mount.as_ref()?;
        Some(WireMount {
            slot: slot as u8,
            item: match mount.item {
                Item::Weapon(weapon) => WireItem::Weapon(weapon.0),
                Item::Gear(gear) => WireItem::Gear(gear.0),
                Item::Supply(supply) => WireItem::Supply(supply.0),
                Item::Contract(trade) => WireItem::Contract(trade.0),
                Item::Cargo(trade) => WireItem::Cargo(trade.0),
            },
            target: mount.target.map(wire_target),
            aim: mount.aim.map(|aim| WireVec2::new(aim.x, aim.y)),
            ready_at: mount.ready_at,
        })
    });
    WireShip {
        id: ship.id.0,
        team: ship.team.into(),
        x: ship.pos.x,
        y: ship.pos.y,
        hull: ship.hull.0,
        hp: ship.hp,
        max_hp: ship.max_hp(tuning),
        level: ship.level(tuning) as u8,
        respawn_at: match ship.life {
            Life::Afloat => None,
            Life::Sunk { respawn_at } => Some(respawn_at),
        },
        slowed: ship.slowed(tick),
        cloaked: ship.cloaked(tick),
        burning: ship.burning(tick),
        own: (ship.team == viewer).then(|| WireOwn {
            gold: ship.gold,
            xp: ship.xp,
            skill_points: ship.skill_points(tuning) as u8,
            skills: ship
                .skills
                .iter()
                .map(|skill| WireSkill {
                    rank: skill.rank,
                    ready_at: skill.ready_at,
                })
                .collect(),
            supply_ready_at: ship.supply_ready_at,
            last_command_seq: ship.last_command_seq,
            refusal: ship.refusal.map(WireRefusal::from),
        }),
        mounts: mounts.collect(),
        last_input_seq: ship.last_input_seq,
    }
}

fn wire_creep(creep: &Creep) -> WireCreep {
    WireCreep {
        id: creep.id.0,
        team: creep.team.into(),
        kind: creep.kind as u8,
        x: creep.pos.x,
        y: creep.pos.y,
        hp: creep.hp,
        target: creep.target.map(wire_target),
        ready_at: creep.ready_at,
    }
}

fn wire_structure(structure: &Structure) -> WireStructure {
    WireStructure {
        id: structure.id.0,
        kind: structure.kind.into(),
        team: structure.team.into(),
        x: structure.pos.x,
        y: structure.pos.y,
        hp: structure.hp,
        target: structure.target.map(wire_target),
        ready_at: structure.ready_at,
    }
}

fn wire_target(target: Target) -> WireTarget {
    match target {
        Target::Ship(id) => WireTarget::Ship(id.0),
        Target::Creep(id) => WireTarget::Creep(id.0),
        Target::Structure(id) => WireTarget::Structure(id.0),
    }
}

#[cfg(test)]
mod tests {
    use warship_rules::{CreepId, Vec2};

    use super::*;

    #[test]
    fn a_team_gets_only_the_notes_of_its_ships_and_no_hit_on_an_enemy_it_did_not_see() {
        let mut sim = Sim::new(Tuning::default());
        let red = sim.join().unwrap();
        let blue = sim.join().unwrap();
        let team = |ship| sim.ship(ship).unwrap().team;
        assert_eq!((team(red), team(blue)), (Team::RED, Team::BLUE));
        let note = |kind| Note {
            id: 7,
            tick: 3,
            pos: Vec2::new(1.0, 2.0),
            amount: 40,
            kind,
        };
        let hit = |target, seen| {
            note(NoteKind::Hit {
                by: Some(red),
                target,
                seen,
            })
        };
        let sent = |team, note: &Note| wire_note(&sim, team, note).map(|note| note.kind);

        let on_ship = hit(Target::Ship(blue), [true, false]);
        let wire = WireNoteKind::Hit {
            by: Some(red.0),
            target: WireTarget::Ship(blue.0),
        };
        assert_eq!(sent(Team::RED, &on_ship), Some(wire));
        assert_eq!(
            sent(Team::BLUE, &on_ship),
            Some(wire),
            "the hit ship's team"
        );

        let unseen = hit(Target::Creep(CreepId(4)), [false, false]);
        assert_eq!(
            sent(Team::RED, &unseen),
            None,
            "the target was out of sight"
        );
        assert_eq!(sent(Team::BLUE, &unseen), None);

        let gold = note(NoteKind::Gold { ship: red });
        assert_eq!(
            sent(Team::RED, &gold),
            Some(WireNoteKind::Gold { ship: red.0 })
        );
        assert_eq!(sent(Team::BLUE, &gold), None);
    }
}
