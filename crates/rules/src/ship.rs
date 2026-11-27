//! 플레이어가 조종하는 배와 그 이동입니다.

use crate::command::Refusal;
use crate::ids::{HullId, ShipId, SkillId, Team, WeaponId};
use crate::input::PlayerInputs;
use crate::math::Vec2;
use crate::nav;
use crate::tuning::Tuning;
use crate::weapon::Target;
use crate::world::World;

/// 가장 큰 선체가 달 수 있는 무기의 수입니다. 선체마다 쓰는 칸의 수는 `HullSpec::mounts`입니다.
pub const MOUNTS: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub struct Ship {
    pub id: ShipId,
    pub team: Team,
    pub pos: Vec2,
    pub hull: HullId,
    pub hp: u32,
    pub life: Life,
    pub gold: u32,
    /// 지금까지 받은 경험치입니다.
    pub xp: u32,
    /// 스킬마다 넣은 스킬 포인트와 쿨다운입니다. 순서는 스킬 표의 순서입니다.
    pub skills: Vec<SkillState>,
    /// 속도를 올리는 스킬의 효과입니다. 효과가 끝난 뒤에도 값은 남아 있습니다.
    pub sprint: Option<Boost>,
    /// 받는 피해를 줄이는 스킬의 효과입니다. 효과가 끝난 뒤에도 값은 남아 있습니다.
    pub brace: Option<Boost>,
    /// 배가 차례로 지나갈 지점입니다. 마지막 지점이 목적지이고, 비어 있으면 멈춰 있습니다.
    pub route: Vec<Vec2>,
    /// 무기를 다는 칸입니다. 빈 칸은 `None`입니다.
    pub mounts: [Option<Mount>; MOUNTS],
    /// 마지막으로 처리한 입력의 순서 번호입니다.
    pub last_input_seq: u32,
    /// 마지막으로 처리한 명령의 순서 번호입니다.
    pub last_command_seq: u32,
    /// 마지막으로 처리한 명령을 거절한 이유입니다. 그 명령을 실행했으면 없습니다.
    pub refusal: Option<Refusal>,
}

/// 배가 스킬 하나에 대해 가진 상태입니다.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SkillState {
    /// 넣은 스킬 포인트의 수입니다. 0이면 배우지 않은 스킬입니다.
    pub rank: u8,
    /// 이 Tick부터 다시 쓸 수 있습니다.
    pub ready_at: u64,
}

/// 정해진 Tick까지 이어지는 스킬의 효과입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Boost {
    /// 이 Tick까지 효과가 있습니다.
    pub until: u64,
    pub percent: u32,
}

impl Boost {
    /// `tick`에 효과가 있으면 그 백분율이고, 없으면 0입니다.
    fn percent_at(boost: Option<Boost>, tick: u64) -> u32 {
        match boost {
            Some(boost) if tick <= boost.until => boost.percent,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Life {
    Afloat,
    /// HP가 0이 된 배입니다. 움직이지 않고, 쏘지 않고, 표적이 되지 않고, 시야를 주지 않습니다.
    /// `respawn_at` Tick에 시작 자리에 다시 나타납니다.
    Sunk {
        respawn_at: u64,
    },
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
            hull: HullId(0),
            hp: tuning.hulls[0].max_hp,
            life: Life::Afloat,
            gold: 0,
            xp: 0,
            skills: vec![SkillState::default(); tuning.skills.len()],
            sprint: None,
            brace: None,
            route: Vec::new(),
            mounts,
            last_input_seq: 0,
            last_command_seq: 0,
            refusal: None,
        }
    }

    /// 배의 레벨입니다. 1부터 시작합니다.
    pub fn level(&self, tuning: &Tuning) -> u32 {
        let reached = |needed: &&u32| **needed <= self.xp;
        1 + tuning.growth.level_xp.iter().take_while(reached).count() as u32
    }

    /// 아직 스킬에 넣지 않은 스킬 포인트의 수입니다.
    pub fn skill_points(&self, tuning: &Tuning) -> u32 {
        let spent: u32 = self.skills.iter().map(|skill| u32::from(skill.rank)).sum();
        self.level(tuning) - spent
    }

    /// 이 선체의 배가 이 스킬을 배우고 쓸 수 있는지입니다.
    pub fn has_skill(&self, tuning: &Tuning, skill: SkillId) -> bool {
        tuning.hull(self.hull).skills.contains(&skill)
    }

    /// `tick`에 이 배에 닿은 `amount`의 피해 가운데 실제로 HP에서 빠지는 양입니다.
    pub(crate) fn braced(&self, amount: u32, tick: u64) -> u32 {
        let kept = 100 - Boost::percent_at(self.brace, tick).min(100);
        (u64::from(amount) * u64::from(kept) / 100) as u32
    }

    pub fn is_afloat(&self) -> bool {
        self.life == Life::Afloat
    }
}

/// 다시 나타날 Tick이 된 배를 최대 HP로 그 진영의 시작 자리에 놓습니다. 목적지는 지워지고, 나머지는 그대로입니다.
pub(crate) fn respawn(world: &mut World, tuning: &Tuning, tick: u64) {
    for ship in world.ships.values_mut() {
        if matches!(ship.life, Life::Sunk { respawn_at } if respawn_at <= tick) {
            ship.life = Life::Afloat;
            ship.hp = tuning.hull(ship.hull).max_hp;
            ship.pos = tuning.map.spawns[ship.team.index()];
            ship.route.clear();
        }
    }
}

/// 입력을 받은 배의 목적지를 바꾸고, 떠 있는 배를 길을 따라 한 Tick만큼 움직입니다.
/// 목적지가 바뀌면 섬을 피해서 가는 길을 다시 찾습니다. 맵 밖이나 섬 위의 목적지는 배가 설 수 있는 가까운 점으로 옮깁니다.
/// 한 Tick에 갈 수 있는 거리 안에 목적지가 있으면 목적지에 정확히 섭니다.
pub(crate) fn move_ships(world: &mut World, inputs: &PlayerInputs, tuning: &Tuning, tick: u64) {
    for ship in world.ships.values_mut() {
        if let Some(input) = inputs.get(&ship.id) {
            ship.last_input_seq = input.seq;
            match input.destination {
                None => ship.route.clear(),
                Some(point) => {
                    let destination = nav::settle(&tuning.map, point.get());
                    if ship.route.last() != Some(&destination) {
                        ship.route = nav::route(&tuning.map, ship.pos, destination);
                    }
                }
            }
        }
        if !ship.is_afloat() {
            continue;
        }
        let boost = Boost::percent_at(ship.sprint, tick);
        let speed = tuning.hull(ship.hull).speed * (100 + boost) as f32 / 100.0;
        let mut step = speed / tuning.tick_hz as f32;
        while let Some(&next) = ship.route.first() {
            let offset = next - ship.pos;
            let distance = offset.length();
            if distance > step {
                ship.pos = ship.pos + offset * (step / distance);
                break;
            }
            ship.pos = next;
            step -= distance;
            ship.route.remove(0);
        }
    }
}
