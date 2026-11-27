//! 게임의 모든 수치입니다. 규칙의 다른 모듈은 숫자를 직접 쓰지 않고 이 구조체에서 읽습니다.
//!
//! 거리의 단위는 타일이고, 시간의 단위는 수치마다 적혀 있습니다(초 또는 Tick).
//! 기본 수치(`Tuning::default`)는 프로토타입용 값이고, 배와 무기의 이름과 맵의 배치는 이 게임에서 새로 지은 것입니다.

use crate::ids::{GearId, HullId, SkillId, Team, TradeId, WeaponId};
use crate::math::Vec2;
use crate::nav;
use crate::ship::MOUNTS;
use crate::structures::StructureKind;

#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    /// 서버가 1초에 실행하는 Tick의 수입니다. Tick 속도는 이 값 하나로만 정합니다.
    pub tick_hz: u32,
    /// 한 진영에 들어올 수 있는 배의 수입니다.
    pub team_size: usize,
    pub ship: ShipTuning,
    /// 선체 표입니다. `HullId`가 이 표의 행 번호이고, 새 배는 0번 선체로 시작합니다.
    pub hulls: Vec<HullSpec>,
    /// 무기 표입니다. `WeaponId`가 이 표의 행 번호입니다. 상점은 이 표의 무기를 모두 팝니다.
    pub weapons: Vec<WeaponSpec>,
    /// 스킬 표입니다. `SkillId`가 이 표의 행 번호이고, 모든 배가 같은 스킬을 배울 수 있습니다.
    pub skills: Vec<SkillSpec>,
    /// 장비 표입니다. 장비는 무기와 같은 칸에 달고, 쏘지 않고, 달려 있는 동안 배의 수치를 올립니다. 상점이 이 장비를 모두 팝니다.
    pub gear: Vec<GearSpec>,
    /// 조합 표입니다. 재료 무기를 모두 단 배가 값을 내면 재료가 결과 무기 하나로 바뀝니다.
    pub recipes: Vec<Recipe>,
    /// 교역 표입니다. `TradeId`가 이 표의 행 번호입니다. 본진이 이 교역의 계약을 모두 팝니다.
    pub trades: Vec<TradeSpec>,
    pub shop: ShopTuning,
    pub growth: GrowthTuning,
    pub structures: StructureTable,
    /// NPC 배의 종류입니다. `Wave::creep`이 이 표의 행 번호입니다.
    pub creeps: Vec<CreepSpec>,
    /// 항구가 NPC 배를 내보내는 일정입니다. 모든 항구가 같은 일정을 따릅니다.
    pub waves: Vec<Wave>,
    pub economy: EconomyTuning,
    pub map: MapSpec,
}

/// 플레이어의 배에 관한 수치입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct ShipTuning {
    /// 배가 처음 가지는 무기입니다. 앞 칸부터 차례로 답니다.
    pub start_weapons: Vec<WeaponId>,
    /// 가라앉은 배가 시작 자리에 다시 나타날 때까지의 Tick 수입니다.
    pub respawn_ticks: u32,
    /// 이 배를 가라앉힌 배가 받는 골드입니다.
    pub bounty: u32,
}

/// 선체 한 종류의 수치입니다. 플레이어는 상점에서 선체를 바꿉니다.
#[derive(Clone, Debug, PartialEq)]
pub struct HullSpec {
    pub name: &'static str,
    pub price: u32,
    pub max_hp: u32,
    /// 1초에 움직이는 거리입니다.
    pub speed: f32,
    /// 이 거리 안의 적이 배의 진영에 보입니다.
    pub vision: f32,
    /// 아이템(무기와 장비)을 달 수 있는 칸의 수입니다. 앞에서부터 이 수만큼의 칸을 씁니다.
    pub mounts: usize,
    /// 이 선체의 배가 배우고 쓸 수 있는 스킬입니다. 클라이언트가 이 순서로 단축키에 놓습니다.
    pub skills: Vec<SkillId>,
}

/// 상점에 관한 수치입니다. 자기 진영의 서 있는 본진과 항구가 상점입니다. 수리는 본진에서만 받습니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShopTuning {
    /// 상점에서 이 거리 안에 있는 배가 사고팔 수 있습니다.
    pub range: f32,
    /// 무기를 팔거나 선체를 바꿀 때 돌려받는, 가격에 대한 백분율입니다.
    pub refund_percent: u32,
    /// 자기 진영의 본진 가까이에 떠 있는 배가 1초마다 회복하는, 최대 HP에 대한 백분율입니다.
    pub repair_percent_per_sec: u32,
}

/// 레벨에 관한 수치입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct GrowthTuning {
    /// 2레벨부터 차례로, 그 레벨이 되는 데 필요한 누적 경험치입니다. 배는 레벨마다 스킬 포인트 하나를 받습니다.
    pub level_xp: Vec<u32>,
    /// 적이 가라앉거나 파괴된 자리에서 이 거리 안에 떠 있는 배가 경험치를 나누어 받습니다.
    pub share_range: f32,
}

/// 스킬 한 종류의 수치입니다. 스킬마다 다른 점은 모두 이 구조체의 값입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct SkillSpec {
    pub name: &'static str,
    /// 한 번 쓴 뒤 다시 쓸 수 있을 때까지의 Tick 수입니다.
    pub cooldown_ticks: u32,
    /// 스킬 포인트를 넣을 수 있는 횟수입니다.
    pub max_rank: u8,
    pub effect: SkillEffect,
}

/// 스킬이 하는 일입니다. 수치는 스킬의 단계(넣은 스킬 포인트의 수)에 비례합니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SkillEffect {
    /// 자기 배의 HP를 최대 HP까지 회복합니다.
    Repair { hp_per_rank: u32 },
    /// `radius` 안의 모든 적의 배와 NPC 배에 피해를 줍니다. 건물에는 피해가 없습니다.
    Salvo { damage_per_rank: u32, radius: f32 },
    /// `ticks` 동안 자기 배의 속도를 백분율만큼 올립니다.
    Sprint { percent_per_rank: u32, ticks: u32 },
    /// 자기 배의 모든 무기를 바로 쏠 수 있게 합니다. 단계가 올라도 효과는 같습니다.
    Reload,
    /// `ticks` 동안 자기 배가 받는 피해를 백분율만큼 줄입니다.
    Brace { percent_per_rank: u32, ticks: u32 },
}

/// 장비 한 종류의 수치입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct GearSpec {
    pub name: &'static str,
    pub price: u32,
    pub effect: GearEffect,
}

/// 장비가 달려 있는 동안 배에 주는 효과입니다. 같은 장비를 여러 개 달면 효과가 더해집니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GearEffect {
    /// 최대 HP를 올립니다.
    Planking { max_hp: u32 },
    /// 속도를 백분율만큼 올립니다.
    Sails { speed_percent: u32 },
    /// 떠 있는 동안 1초마다 HP를 회복합니다.
    Crew { repair_per_sec: u32 },
    /// 받는 피해를 백분율만큼 줄입니다. 스킬의 피해 감소와 더해지고, 합은 `ARMOR_CAP_PERCENT`를 넘지 않습니다.
    Plating { armor_percent: u32 },
}

/// 받는 피해를 줄이는 효과를 모두 더한 값의 상한(백분율)입니다.
pub const ARMOR_CAP_PERCENT: u32 = 80;

/// 무기의 조합 하나입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    /// 재료 무기입니다. 같은 무기가 두 번 있으면 그 무기를 두 개 달고 있어야 합니다.
    pub parts: Vec<WeaponId>,
    pub result: WeaponId,
    /// 조합할 때 내는 골드입니다.
    pub price: u32,
}

/// 교역 하나입니다. 본진에서 계약을 사고, 교역항에서 화물을 싣고, 본진에 돌아오면 보상을 받습니다.
/// 계약과 화물은 칸 하나를 차지하고, 배가 가라앉으면 사라집니다.
#[derive(Clone, Debug, PartialEq)]
pub struct TradeSpec {
    pub name: &'static str,
    /// 계약의 값입니다.
    pub price: u32,
    /// 화물을 본진에 가져오면 받는 골드입니다. 계약의 값보다 큽니다.
    pub reward: u32,
    /// 화물을 싣는 교역항입니다. `MapSpec::ports`의 자리입니다.
    pub port: usize,
}

/// 무기 한 종류의 수치입니다. 무기마다 다른 점은 모두 이 구조체의 값입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct WeaponSpec {
    pub name: &'static str,
    /// 포탄 하나가 주는 피해입니다.
    pub damage: u32,
    /// 이 거리 안에 있고 진영에 보이는 적을 쏩니다.
    pub range: f32,
    /// 한 번 쏜 뒤 다시 쏠 수 있을 때까지의 Tick 수입니다.
    pub cooldown_ticks: u32,
    /// 포탄이 1초에 날아가는 거리입니다. 피해는 포탄이 표적까지 날아가는 시간이 지난 뒤에 들어갑니다.
    pub shot_speed: f32,
    pub hits: Hits,
    /// 무기의 값입니다. 상점에서 사는 가격이고, 팔 때 이 값의 일부를 돌려받습니다. 건물과 NPC 배의 포에서는 쓰지 않습니다.
    pub price: u32,
    /// 상점에서 바로 살 수 있는지입니다. 조합으로만 얻는 무기는 거짓입니다.
    pub buyable: bool,
}

/// 무기가 쏠 수 있는 표적의 종류입니다. 배에는 플레이어의 배와 NPC 배가 모두 들어갑니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hits {
    ShipsOnly,
    ShipsAndStructures,
}

/// 건물 종류별 수치입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct StructureTable {
    pub tower: StructureSpec,
    pub harbor: StructureSpec,
    pub base: StructureSpec,
}

impl StructureTable {
    pub fn of(&self, kind: StructureKind) -> &StructureSpec {
        match kind {
            StructureKind::Tower => &self.tower,
            StructureKind::Harbor => &self.harbor,
            StructureKind::Base => &self.base,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructureSpec {
    pub max_hp: u32,
    /// 이 거리 안의 적이 건물의 진영에 보입니다.
    pub vision: f32,
    /// 건물의 포입니다. 배의 무기와 같은 규칙으로 쏩니다. 없으면 쏘지 않습니다.
    pub weapon: Option<WeaponSpec>,
    /// 이 건물을 파괴한 배가 받는 골드입니다.
    pub bounty: u32,
}

/// NPC 배 한 종류의 수치입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct CreepSpec {
    pub name: &'static str,
    pub max_hp: u32,
    /// 1초에 움직이는 거리입니다.
    pub speed: f32,
    pub vision: f32,
    pub weapon: WeaponSpec,
    /// 이 NPC 배를 가라앉힌 배가 받는 골드입니다.
    pub bounty: u32,
}

/// 항구가 NPC 배를 되풀이해서 내보내는 일정 하나입니다.
/// `first_tick`에 첫 무리가 나오고, 그 뒤 `period_ticks`마다 다음 무리가 나옵니다. 무리 안의 배는 `gap_ticks` 간격으로 한 척씩 나옵니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wave {
    /// `Tuning::creeps`의 행 번호입니다.
    pub creep: usize,
    /// 무리 하나의 배의 수입니다.
    pub count: u32,
    pub first_tick: u64,
    pub period_ticks: u32,
    pub gap_ticks: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EconomyTuning {
    /// 모든 배가 1초마다 받는 골드입니다.
    pub income_per_sec: u32,
}

/// 맵의 크기와 처음부터 놓여 있는 것입니다. 맵은 (0, 0)에서 (`width`, `height`)까지입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct MapSpec {
    pub width: f32,
    pub height: f32,
    /// 진영마다 배가 처음 놓이는 자리입니다. 순서는 `Team::index`입니다.
    pub spawns: [Vec2; 2],
    /// 라인마다 NPC 배가 지나가는 지점입니다. `Team::RED` 쪽에서 `Team::BLUE` 쪽으로 가는 순서이고, `Team::BLUE`의 NPC 배는 거꾸로 갑니다.
    pub lanes: Vec<Vec<Vec2>>,
    pub structures: Vec<StructureSite>,
    /// 배가 지나가지 못하는 섬입니다.
    pub islands: Vec<Island>,
    /// 교역항의 자리입니다. 어느 진영의 것도 아니고, 상점 거리 안에 온 배의 계약을 화물로 바꿉니다.
    pub ports: Vec<Vec2>,
}

/// 원 모양의 섬입니다.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Island {
    pub center: Vec2,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StructureSite {
    pub kind: StructureKind,
    pub team: Team,
    pub pos: Vec2,
    /// 항구가 맡은 라인의 번호입니다. 항구에만 있습니다.
    pub lane: Option<usize>,
}

impl Tuning {
    pub fn hull(&self, id: HullId) -> &HullSpec {
        &self.hulls[usize::from(id.0)]
    }

    /// 교역 표에 있는 번호이면 그 교역입니다.
    pub fn trade_of(&self, id: TradeId) -> Option<&TradeSpec> {
        self.trades.get(usize::from(id.0))
    }

    /// 장비 표에 있는 번호이면 그 장비입니다.
    pub fn gear_of(&self, id: GearId) -> Option<&GearSpec> {
        self.gear.get(usize::from(id.0))
    }
}

impl MapSpec {
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= 0.0 && p.x <= self.width && p.y >= 0.0 && p.y <= self.height
    }

    /// 맵 안에서 `p`와 가장 가까운 점입니다.
    pub fn clamp(&self, p: Vec2) -> Vec2 {
        Vec2::new(p.x.clamp(0.0, self.width), p.y.clamp(0.0, self.height))
    }

    /// `team`의 본진이 놓이는 자리입니다. 본진이 없는 맵이면 없습니다.
    pub fn base_of(&self, team: Team) -> Option<Vec2> {
        let is_base = |site: &&StructureSite| site.kind == StructureKind::Base && site.team == team;
        self.structures.iter().find(is_base).map(|site| site.pos)
    }

    /// `team`의 NPC 배가 `lane`에서 `leg`번째로 향하는 지점입니다. 라인의 지점을 모두 지나면 적의 본진으로 향하고, 그 뒤에는 없습니다.
    pub fn path_point(&self, team: Team, lane: usize, leg: usize) -> Option<Vec2> {
        let points = self.lanes.get(lane)?;
        if leg == points.len() {
            return self.base_of(team.enemy());
        }
        let index = if team == Team::RED {
            leg
        } else {
            points.len().checked_sub(leg + 1)?
        };
        points.get(index).copied()
    }
}

impl WeaponSpec {
    fn validate(&self) -> Result<(), String> {
        let name = self.name;
        if self.damage == 0 {
            return Err(format!("weapon {name}: damage must be at least 1"));
        }
        if self.cooldown_ticks == 0 {
            return Err(format!("weapon {name}: cooldown_ticks must be at least 1"));
        }
        if !is_positive(self.range) {
            return Err(format!("weapon {name}: range must be positive"));
        }
        if !is_positive(self.shot_speed) {
            return Err(format!("weapon {name}: shot_speed must be positive"));
        }
        Ok(())
    }
}

fn is_positive(value: f32) -> bool {
    value.is_finite() && value > 0.0
}

impl Tuning {
    /// 규칙이 전제하는 조건을 수치가 지키는지 확인합니다. `Sim::new`에는 이 검사를 통과한 수치만 넘깁니다.
    pub fn validate(&self) -> Result<(), String> {
        if self.tick_hz == 0 {
            return Err("tick_hz must be at least 1".to_string());
        }
        if self.team_size == 0 {
            return Err("team_size must be at least 1".to_string());
        }
        let Some(starter) = self.hulls.first() else {
            return Err("the hull table needs at least one hull".to_string());
        };
        for hull in &self.hulls {
            if !is_positive(hull.speed) {
                return Err(format!("hull {}: speed must be positive", hull.name));
            }
            if hull.mounts == 0 || hull.mounts > MOUNTS {
                return Err(format!("hull {}: mounts must be 1 to {MOUNTS}", hull.name));
            }
            let known = |skill: &SkillId| usize::from(skill.0) < self.skills.len();
            let repeated = |(nth, skill): (usize, &SkillId)| hull.skills[..nth].contains(skill);
            if !hull.skills.iter().all(known) || hull.skills.iter().enumerate().any(repeated) {
                return Err(format!(
                    "hull {}: skills must be distinct rows of the skill table",
                    hull.name
                ));
            }
        }
        let known = |weapon: &WeaponId| usize::from(weapon.0) < self.weapons.len();
        for recipe in &self.recipes {
            if recipe.parts.len() < 2 || !recipe.parts.iter().all(known) || !known(&recipe.result) {
                return Err(
                    "a recipe needs two or more parts, and its parts and result must be weapons of the table"
                        .to_string(),
                );
            }
        }
        for trade in &self.trades {
            if trade.port >= self.map.ports.len() || trade.reward <= trade.price {
                return Err(format!(
                    "trade {}: it names a port of the map and pays more than its price",
                    trade.name
                ));
            }
        }
        for gear in &self.gear {
            if matches!(gear.effect, GearEffect::Plating { armor_percent } if armor_percent > ARMOR_CAP_PERCENT)
            {
                return Err(format!(
                    "gear {}: plating takes at most {ARMOR_CAP_PERCENT} percent",
                    gear.name
                ));
            }
        }
        let percents = [self.shop.refund_percent, self.shop.repair_percent_per_sec];
        if !is_positive(self.shop.range) || percents.iter().any(|percent| *percent > 100) {
            return Err(
                "the shop needs a positive range, and a refund and a repair of at most 100 percent"
                    .to_string(),
            );
        }
        if !is_positive(self.growth.share_range) {
            return Err("the experience share range must be positive".to_string());
        }
        if !self.growth.level_xp.is_sorted_by(|a, b| a < b) {
            return Err("level_xp must increase with every level".to_string());
        }
        for skill in &self.skills {
            let effect_fits = match skill.effect {
                SkillEffect::Salvo { radius, .. } => is_positive(radius),
                // 가장 높은 단계에서도 피해가 음수가 되지 않아야 합니다.
                SkillEffect::Brace {
                    percent_per_rank, ..
                } => percent_per_rank * u32::from(skill.max_rank) <= 100,
                SkillEffect::Repair { .. } | SkillEffect::Sprint { .. } | SkillEffect::Reload => {
                    true
                }
            };
            if skill.cooldown_ticks == 0 || skill.max_rank == 0 || !effect_fits {
                return Err(format!(
                    "skill {}: cooldown_ticks, max_rank and radius must be positive, and a brace takes at most 100 percent",
                    skill.name
                ));
            }
        }
        let kinds = [
            StructureKind::Tower,
            StructureKind::Harbor,
            StructureKind::Base,
        ];
        let structures = kinds.map(|kind| self.structures.of(kind));
        let mut max_hps: Vec<u32> = self.hulls.iter().map(|hull| hull.max_hp).collect();
        let mut visions: Vec<f32> = self.hulls.iter().map(|hull| hull.vision).collect();
        let mut weapons: Vec<&WeaponSpec> = self.weapons.iter().collect();
        for spec in structures {
            max_hps.push(spec.max_hp);
            visions.push(spec.vision);
            weapons.extend(&spec.weapon);
        }
        for creep in &self.creeps {
            max_hps.push(creep.max_hp);
            visions.push(creep.vision);
            weapons.push(&creep.weapon);
            if !is_positive(creep.speed) {
                return Err(format!("creep {}: speed must be positive", creep.name));
            }
        }
        if max_hps.contains(&0) {
            return Err("max_hp must be at least 1".to_string());
        }
        if !visions.into_iter().all(is_positive) {
            return Err("vision must be positive".to_string());
        }
        for weapon in weapons {
            weapon.validate()?;
        }
        if self.ship.start_weapons.len() > starter.mounts {
            return Err(format!(
                "the starting hull carries at most {} weapons",
                starter.mounts
            ));
        }
        for id in &self.ship.start_weapons {
            if usize::from(id.0) >= self.weapons.len() {
                return Err(format!("start weapon {} is not in the weapon table", id.0));
            }
        }
        for wave in &self.waves {
            if wave.creep >= self.creeps.len() {
                return Err(format!(
                    "wave creep {} is not in the creep table",
                    wave.creep
                ));
            }
            if wave.count == 0 || wave.period_ticks == 0 {
                return Err(
                    "a wave needs at least one creep and a period of at least one tick".to_string(),
                );
            }
        }
        self.map.validate()
    }
}

impl MapSpec {
    fn validate(&self) -> Result<(), String> {
        if !is_positive(self.width) || !is_positive(self.height) {
            return Err("the map must have a positive size".to_string());
        }
        let sites = self.structures.iter().map(|site| site.pos);
        let lane_points = self.lanes.iter().flatten().copied();
        if !sites
            .chain(self.spawns)
            .chain(lane_points)
            .chain(self.ports.iter().copied())
            .all(|p| self.contains(p))
        {
            return Err("spawns, lanes, structures and ports must be inside the map".to_string());
        }
        if self.lanes.iter().any(|lane| lane.len() < 2) {
            return Err("a lane needs at least two points".to_string());
        }
        for team in Team::ALL {
            let is_base =
                |site: &&StructureSite| site.kind == StructureKind::Base && site.team == team;
            if self.structures.iter().filter(is_base).count() > 1 {
                return Err("a team has at most one base".to_string());
            }
        }
        for site in &self.structures {
            let lane_fits = match (site.kind, site.lane) {
                (StructureKind::Harbor, Some(lane)) => lane < self.lanes.len(),
                (StructureKind::Harbor, None) => false,
                (_, lane) => lane.is_none(),
            };
            if !lane_fits {
                return Err(
                    "a harbor names one of the lanes, and other structures name none".to_string(),
                );
            }
        }
        for (index, island) in self.islands.iter().enumerate() {
            let (center, reach) = (island.center, island.radius + nav::CHANNEL);
            let inside = center.x >= reach
                && center.x <= self.width - reach
                && center.y >= reach
                && center.y <= self.height - reach;
            if !is_positive(island.radius) || !inside {
                return Err(
                    "an island has a positive radius and leaves a channel to the map edge"
                        .to_string(),
                );
            }
            let apart = |other: &Island| center.distance(other.center) >= reach + other.radius;
            if !self.islands[..index].iter().all(apart) {
                return Err("islands must leave a channel between them".to_string());
            }
        }
        let sites = self.structures.iter().map(|site| site.pos);
        let ports = self.ports.iter().copied();
        if !sites
            .chain(self.spawns)
            .chain(ports)
            .all(|p| nav::is_open(self, p))
        {
            return Err("spawns, structures and ports must be off the islands".to_string());
        }
        // NPC 배는 길을 찾지 않으므로, 항구에서 적의 본진까지 가는 길의 모든 구간이 섬을 지나지 않아야 합니다.
        for site in &self.structures {
            let Some(lane) = site.lane else { continue };
            let mut from = site.pos;
            let points = (0..).map_while(|leg| self.path_point(site.team, lane, leg));
            for point in points {
                if !nav::clear(self, from, point) {
                    return Err("a lane must not cross an island".to_string());
                }
                from = point;
            }
        }
        Ok(())
    }
}

impl Default for Tuning {
    /// 64 x 48 맵입니다. 두 진영의 본진이 동서 끝에 있고, 북쪽과 남쪽에 라인이 하나씩 있습니다.
    /// 라인마다 진영별로 항구 하나와 타워 하나가 있습니다. 두 라인 사이에 섬 셋이 있고, 라인의 바깥쪽 한가운데에 섬이 하나씩 있습니다.
    fn default() -> Tuning {
        let gun = |name, damage, range, cooldown_ticks, shot_speed, hits| WeaponSpec {
            name,
            damage,
            range,
            cooldown_ticks,
            shot_speed,
            hits,
            price: 0,
            buyable: false,
        };
        // 상점에서 파는 무기와, 조합으로만 얻는 무기입니다. 조합한 무기의 값은 재료의 값과 조합 값의 합입니다.
        let sold = |price, weapon: WeaponSpec| WeaponSpec {
            price,
            buyable: true,
            ..weapon
        };
        let made = |price, weapon: WeaponSpec| WeaponSpec { price, ..weapon };
        let recipe = |parts: [u16; 2], result, price| Recipe {
            parts: parts.map(WeaponId).to_vec(),
            result: WeaponId(result),
            price,
        };
        let trade = |name, price, reward, port| TradeSpec {
            name,
            price,
            reward,
            port,
        };
        let gear = |name, price, effect| GearSpec {
            name,
            price,
            effect,
        };
        // 모든 선체가 0번과 1번 스킬을 쓰고, 선체마다 고유한 스킬이 있습니다.
        let hull = |name, price, max_hp, speed, mounts, own_skills: &[u16]| {
            let own = own_skills.iter().map(|skill| SkillId(*skill));
            HullSpec {
                name,
                price,
                max_hp,
                speed,
                vision: 9.0 + max_hp as f32 / 1500.0,
                mounts,
                skills: [SkillId(0), SkillId(1)].into_iter().chain(own).collect(),
            }
        };
        let skill = |name, cooldown_ticks, effect| SkillSpec {
            name,
            cooldown_ticks,
            max_rank: 3,
            effect,
        };
        let both = Hits::ShipsAndStructures;
        let (north, south) = (36.0, 12.0);
        let mut structures = Vec::new();
        for (team, base_x, harbor_x, tower_x) in
            [(Team::RED, 4.0, 10.0, 22.0), (Team::BLUE, 60.0, 54.0, 42.0)]
        {
            let site = |kind, x, y, lane| StructureSite {
                kind,
                team,
                pos: Vec2::new(x, y),
                lane,
            };
            structures.push(site(StructureKind::Base, base_x, 24.0, None));
            for (lane, y) in [north, south].into_iter().enumerate() {
                structures.push(site(StructureKind::Harbor, harbor_x, y, Some(lane)));
                structures.push(site(StructureKind::Tower, tower_x, y, None));
            }
        }
        Tuning {
            tick_hz: 20,
            team_size: 5,
            ship: ShipTuning {
                start_weapons: vec![WeaponId(0)],
                respawn_ticks: 200,
                bounty: 200,
            },
            hulls: vec![
                hull("Sloop", 0, 400, 4.0, 3, &[2]),
                hull("Brig", 500, 650, 3.8, 4, &[2]),
                hull("Frigate", 1100, 950, 3.6, 5, &[3]),
                hull("Galleon", 2000, 1500, 3.2, 6, &[4]),
                hull("Man-of-War", 3500, 2300, 3.0, 6, &[3, 4]),
            ],
            weapons: vec![
                sold(100, gun("Deck Gun", 20, 5.5, 30, 14.0, both)),
                sold(150, gun("Swivel Gun", 4, 3.5, 3, 18.0, both)),
                sold(300, gun("Long Nine", 30, 12.0, 25, 12.0, Hits::ShipsOnly)),
                sold(250, gun("Carronade", 45, 3.5, 40, 14.0, both)),
                sold(400, gun("Mortar", 50, 9.0, 60, 7.0, both)),
                made(350, gun("Twin Deck Guns", 48, 5.5, 30, 14.0, both)),
                made(500, gun("Chain Gun", 6, 4.0, 2, 18.0, both)),
                made(600, gun("Heavy Carronade", 80, 4.0, 36, 14.0, both)),
                made(1100, gun("Siege Mortar", 90, 11.0, 55, 8.0, both)),
            ],
            recipes: vec![
                recipe([0, 0], 5, 150),
                recipe([1, 1], 6, 200),
                recipe([3, 0], 7, 250),
                recipe([4, 2], 8, 400),
            ],
            trades: vec![
                trade("Timber", 60, 90, 0),
                trade("Salt", 60, 90, 1),
                trade("Spice", 300, 360, 0),
                trade("Silk", 300, 360, 1),
            ],
            gear: vec![
                gear("Oak Planking", 200, GearEffect::Planking { max_hp: 250 }),
                gear("Extra Sails", 250, GearEffect::Sails { speed_percent: 12 }),
                gear("Shipwrights", 300, GearEffect::Crew { repair_per_sec: 6 }),
                gear(
                    "Copper Sheathing",
                    350,
                    GearEffect::Plating { armor_percent: 12 },
                ),
            ],
            skills: vec![
                skill("Patch Up", 600, SkillEffect::Repair { hp_per_rank: 80 }),
                skill(
                    "Grapeshot",
                    300,
                    SkillEffect::Salvo {
                        damage_per_rank: 40,
                        radius: 4.0,
                    },
                ),
                skill(
                    "Full Sail",
                    400,
                    SkillEffect::Sprint {
                        percent_per_rank: 20,
                        ticks: 80,
                    },
                ),
                skill("Broadside", 500, SkillEffect::Reload),
                skill(
                    "Iron Hull",
                    600,
                    SkillEffect::Brace {
                        percent_per_rank: 20,
                        ticks: 100,
                    },
                ),
            ],
            shop: ShopTuning {
                range: 5.0,
                refund_percent: 50,
                repair_percent_per_sec: 5,
            },
            growth: GrowthTuning {
                level_xp: vec![100, 250, 450, 700, 1000, 1400, 1900, 2500, 3200],
                share_range: 10.0,
            },
            structures: StructureTable {
                tower: StructureSpec {
                    max_hp: 3000,
                    vision: 9.0,
                    weapon: Some(gun("Tower Battery", 30, 8.0, 30, 14.0, Hits::ShipsOnly)),
                    bounty: 300,
                },
                harbor: StructureSpec {
                    max_hp: 7000,
                    vision: 9.0,
                    weapon: None,
                    bounty: 500,
                },
                base: StructureSpec {
                    max_hp: 12000,
                    vision: 11.0,
                    weapon: Some(gun("Citadel Battery", 45, 9.0, 20, 16.0, Hits::ShipsOnly)),
                    bounty: 0,
                },
            },
            creeps: vec![
                CreepSpec {
                    name: "Cutter",
                    max_hp: 120,
                    speed: 2.5,
                    vision: 7.0,
                    weapon: gun("Cutter Gun", 6, 4.0, 20, 14.0, both),
                    bounty: 20,
                },
                CreepSpec {
                    name: "Brig",
                    max_hp: 400,
                    speed: 2.0,
                    vision: 8.0,
                    weapon: gun("Brig Gun", 16, 5.5, 30, 14.0, both),
                    bounty: 50,
                },
            ],
            waves: vec![
                Wave {
                    creep: 0,
                    count: 3,
                    first_tick: 40,
                    period_ticks: 600,
                    gap_ticks: 12,
                },
                Wave {
                    creep: 1,
                    count: 1,
                    first_tick: 1240,
                    period_ticks: 1200,
                    gap_ticks: 0,
                },
            ],
            economy: EconomyTuning { income_per_sec: 2 },
            map: MapSpec {
                width: 64.0,
                height: 48.0,
                spawns: [Vec2::new(7.0, 24.0), Vec2::new(57.0, 24.0)],
                lanes: [north, south]
                    .into_iter()
                    .map(|y| vec![Vec2::new(12.0, y), Vec2::new(52.0, y)])
                    .collect(),
                structures,
                islands: [
                    (32.0, 24.0, 5.0),
                    (20.0, 24.0, 3.0),
                    (44.0, 24.0, 3.0),
                    (32.0, 43.5, 2.5),
                    (32.0, 4.5, 2.5),
                ]
                .into_iter()
                .map(|(x, y, radius)| Island {
                    center: Vec2::new(x, y),
                    radius,
                })
                .collect(),
                // 두 진영의 본진에서 같은 거리에 있는, 가운데 섬의 북쪽과 남쪽입니다.
                ports: vec![Vec2::new(32.0, 31.5), Vec2::new(32.0, 16.5)],
            },
        }
    }
}
