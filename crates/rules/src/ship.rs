//! 플레이어가 조종하는 배와 그 이동입니다.

use std::collections::BTreeMap;

use crate::command::Refusal;
use crate::ids::{GearId, HullId, ShipId, SkillId, SupplyId, Team, TradeId, WeaponId};
use crate::input::PlayerInputs;
use crate::math::Vec2;
use crate::nav;
use crate::structures;
use crate::tuning::{ARMOR_CAP_PERCENT, GearEffect, Tuning};
use crate::weapon::Target;
use crate::world::World;

/// 가장 큰 선체의 아이템 칸의 수입니다. 선체마다 쓰는 칸의 수는 `HullSpec::mounts`입니다.
pub const MOUNTS: usize = 6;

/// 플레이어의 배 하나가 다른 배에 준 피해입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wound {
    pub by: ShipId,
    pub damage: u32,
    /// 마지막으로 맞힌 Tick입니다.
    pub last: u64,
}

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
    /// 적의 스킬이 속도를 줄이는 효과입니다. 효과가 끝난 뒤에도 값은 남아 있습니다.
    pub snare: Option<Boost>,
    /// 이 Tick까지 은신합니다. 은신하지 않은 배는 0입니다.
    pub cloaked_until: u64,
    /// 이 Tick까지 불이 붙어 있습니다. 불이 붙은 적이 없는 배는 0입니다.
    pub burning_until: u64,
    /// 최근에 이 배를 맞힌 플레이어의 배마다 준 피해입니다. 이 배가 가라앉으면 도움의 보상을 나누는 데 쓰고 비웁니다.
    pub wounds: Vec<Wound>,
    /// 이 Tick부터 소모품을 다시 쓸 수 있습니다.
    pub supply_ready_at: u64,
    /// 배가 차례로 지나갈 지점입니다. 마지막 지점이 목적지이고, 비어 있으면 멈춰 있습니다.
    pub route: Vec<Vec2>,
    /// 아이템을 다는 칸입니다. 무기와 장비가 같은 칸을 나누어 씁니다. 빈 칸은 `None`입니다.
    pub mounts: [Option<Mount>; MOUNTS],
    /// 이 배의 포탄이 가라앉힌 적의 배의 수입니다.
    pub kills: u32,
    /// 이 배가 가라앉은 횟수입니다.
    pub deaths: u32,
    /// 이 배의 포탄이 가라앉힌 NPC 배의 수입니다.
    pub creep_kills: u32,
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

/// 칸에 다는 아이템입니다. 무기는 스스로 쏘고, 장비는 달려 있는 동안 배의 수치를 올리고, 소모품은 쓸 때 한 번 효과를 냅니다.
/// 교역의 계약과 화물도 칸 하나를 차지합니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Weapon(WeaponId),
    Gear(GearId),
    /// 소모품입니다. 한 번 쓰면 사라집니다.
    Supply(SupplyId),
    /// 교역의 계약입니다. 교역항에 가면 화물로 바뀝니다.
    Contract(TradeId),
    /// 교역항에서 실은 화물입니다. 자기 진영의 본진에 가져가면 보상으로 바뀝니다.
    Cargo(TradeId),
}

/// 칸 하나에 달린 아이템입니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mount {
    pub item: Item,
    /// 무기가 이 Tick부터 다시 쏠 수 있습니다. 장비에서는 쓰지 않습니다.
    pub ready_at: u64,
    /// 무기가 마지막으로 쏜 표적입니다. 쏠 수 있는데 쏠 적이 없으면 `None`이 됩니다.
    pub target: Option<Target>,
    /// 마지막으로 쏠 때 표적이 있던 자리입니다. 폭탄은 이 자리에 떨어집니다.
    pub aim: Option<Vec2>,
}

impl Mount {
    /// 바로 쓸 수 있는 새 아이템입니다.
    pub(crate) fn new(item: Item) -> Mount {
        Mount {
            item,
            ready_at: 0,
            target: None,
            aim: None,
        }
    }

    /// 이 칸의 아이템이 무기이면 그 무기입니다.
    pub fn weapon(&self) -> Option<WeaponId> {
        match self.item {
            Item::Weapon(weapon) => Some(weapon),
            Item::Gear(_) | Item::Supply(_) | Item::Contract(_) | Item::Cargo(_) => None,
        }
    }

    /// 이 칸의 아이템이 장비이면 그 장비입니다.
    pub fn gear(&self) -> Option<GearId> {
        match self.item {
            Item::Gear(gear) => Some(gear),
            Item::Weapon(_) | Item::Supply(_) | Item::Contract(_) | Item::Cargo(_) => None,
        }
    }
}

impl Ship {
    pub(crate) fn new(id: ShipId, team: Team, pos: Vec2, tuning: &Tuning) -> Ship {
        let mut mounts = [None; MOUNTS];
        for (mount, &weapon) in mounts.iter_mut().zip(&tuning.ship.start_weapons) {
            *mount = Some(Mount::new(Item::Weapon(weapon)));
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
            snare: None,
            cloaked_until: 0,
            burning_until: 0,
            wounds: Vec::new(),
            supply_ready_at: 0,
            route: Vec::new(),
            mounts,
            kills: 0,
            deaths: 0,
            creep_kills: 0,
            last_input_seq: 0,
            last_command_seq: 0,
            refusal: None,
        }
    }

    /// 배의 레벨입니다. 1부터 시작합니다.
    /// `by`가 이 배에 준 피해를 적습니다. `assist_ticks`보다 오래 맞히지 않은 배의 피해는 잊습니다.
    pub(crate) fn wound(&mut self, by: ShipId, damage: u32, tuning: &Tuning, tick: u64) {
        let fresh = |wound: &Wound| wound.last + u64::from(tuning.ship.assist_ticks) > tick;
        self.wounds.retain(fresh);
        match self.wounds.iter_mut().find(|wound| wound.by == by) {
            Some(wound) => {
                wound.damage += damage;
                wound.last = tick;
            }
            None => self.wounds.push(Wound {
                by,
                damage,
                last: tick,
            }),
        }
    }

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

    /// 달고 있는 장비의 효과입니다.
    fn gear_effects<'a>(&'a self, tuning: &'a Tuning) -> impl Iterator<Item = GearEffect> + 'a {
        let gear = self.mounts.iter().flatten().filter_map(Mount::gear);
        let specs = gear.filter_map(|id| tuning.gear_of(id));
        specs.map(|gear| gear.effect)
    }

    /// 선체의 최대 HP에 장비가 올린 HP를 더한 값입니다.
    pub fn max_hp(&self, tuning: &Tuning) -> u32 {
        let planking = self.gear_effects(tuning).map(|effect| match effect {
            GearEffect::Planking { max_hp } => max_hp,
            _ => 0,
        });
        tuning.hull(self.hull).max_hp + planking.sum::<u32>()
    }

    /// `tick`에 이 배가 1초에 가는 거리입니다. 선체의 속도에 돛 장비와 속도 스킬의 백분율을 더해서 올리고, 적의 감속 스킬의 백분율만큼 줄입니다.
    pub fn speed(&self, tuning: &Tuning, tick: u64) -> f32 {
        let sails = self.gear_effects(tuning).map(|effect| match effect {
            GearEffect::Sails { speed_percent } => speed_percent,
            _ => 0,
        });
        let percent = 100 + sails.sum::<u32>() + Boost::percent_at(self.sprint, tick);
        let kept = 100 - Boost::percent_at(self.snare, tick).min(100);
        tuning.hull(self.hull).speed * (percent * kept) as f32 / 10_000.0
    }

    /// 선체와 선원 장비가 1초마다 회복하는 HP입니다.
    fn crew_repair(&self, tuning: &Tuning) -> u32 {
        let crew = self.gear_effects(tuning).map(|effect| match effect {
            GearEffect::Crew { repair_per_sec } => repair_per_sec,
            _ => 0,
        });
        tuning.hull(self.hull).repair_per_sec + crew.sum::<u32>()
    }

    /// `tick`에 이 배에 닿은 `amount`의 피해 가운데 실제로 HP에서 빠지는 양입니다.
    /// 선체의 장갑, 장갑 장비, 피해 감소 스킬의 백분율을 더하고, 합은 `ARMOR_CAP_PERCENT`를 넘지 않습니다.
    pub(crate) fn braced(&self, tuning: &Tuning, amount: u32, tick: u64) -> u32 {
        let plating = self.gear_effects(tuning).map(|effect| match effect {
            GearEffect::Plating { armor_percent } => armor_percent,
            _ => 0,
        });
        let hull = tuning.hull(self.hull).armor_percent;
        let armor = hull + plating.sum::<u32>() + Boost::percent_at(self.brace, tick);
        let kept = 100 - armor.min(ARMOR_CAP_PERCENT);
        (u64::from(amount) * u64::from(kept) / 100) as u32
    }

    /// `tick`에 은신하고 있는지입니다. 은신한 배는 적의 배와 NPC 배에게 보이지 않습니다.
    pub fn cloaked(&self, tick: u64) -> bool {
        self.is_afloat() && tick <= self.cloaked_until
    }

    /// `tick`에 불이 붙어 있는지입니다. 불이 붙은 배는 1초마다 피해를 받습니다.
    pub fn burning(&self, tick: u64) -> bool {
        self.is_afloat() && tick < self.burning_until
    }

    /// `tick`에 적의 감속 스킬의 효과를 받고 있는지입니다.
    pub fn slowed(&self, tick: u64) -> bool {
        Boost::percent_at(self.snare, tick) > 0
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
            ship.hp = ship.max_hp(tuning);
            ship.pos = tuning.map.spawns[ship.team.index()];
            ship.route.clear();
        }
    }
}

/// 선원 장비를 단 떠 있는 배가 1초에 한 번 HP를 회복합니다.
pub(crate) fn crew_repairs(world: &mut World, tuning: &Tuning, tick: u64) {
    if !tick.is_multiple_of(u64::from(tuning.tick_hz)) {
        return;
    }
    for ship in world.ships.values_mut().filter(|ship| ship.is_afloat()) {
        ship.hp = (ship.hp + ship.crew_repair(tuning)).min(ship.max_hp(tuning));
    }
}

/// 입력을 받은 배의 목적지를 바꾸고, 떠 있는 배를 길을 따라 한 Tick만큼 움직입니다.
/// 목적지가 바뀌면 섬을 피해서 가는 길을 다시 찾습니다. 맵 밖이나 섬 위의 목적지는 배가 설 수 있는 가까운 점으로 옮깁니다.
/// 한 Tick에 갈 수 있는 거리 안에 목적지가 있으면 목적지에 정확히 섭니다.
pub(crate) fn move_ships(world: &mut World, inputs: &PlayerInputs, tuning: &Tuning, tick: u64) {
    // 건물 위의 목적지를 그 건물의 둘레로 옮깁니다. 배를 고치는 동안에는 건물을 읽을 수 없으므로 먼저 계산합니다.
    let asked = world.ships.values().filter_map(|ship| {
        let point = inputs.get(&ship.id)?.destination?.get();
        let open = nav::settle(&tuning.map, point);
        let beside = structures::beside(world, open, ship.pos);
        Some((ship.id, nav::settle(&tuning.map, beside)))
    });
    let asked: BTreeMap<ShipId, Vec2> = asked.collect();
    for ship in world.ships.values_mut() {
        if let Some(input) = inputs.get(&ship.id) {
            ship.last_input_seq = input.seq;
            match input.destination {
                None => ship.route.clear(),
                Some(_) => {
                    if let Some(&destination) = asked.get(&ship.id)
                        && ship.route.last() != Some(&destination)
                    {
                        ship.route = nav::route(&tuning.map, ship.pos, destination);
                    }
                }
            }
        }
        if !ship.is_afloat() {
            continue;
        }
        let mut step = ship.speed(tuning, tick) / tuning.tick_hz as f32;
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
