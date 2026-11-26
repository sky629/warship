//! 게임의 모든 수치입니다. 규칙의 다른 모듈은 숫자를 직접 쓰지 않고 이 구조체에서 읽습니다.
//!
//! 거리의 단위는 타일이고, 시간의 단위는 수치마다 적혀 있습니다(초 또는 Tick).
//! 기본 수치(`Tuning::default`)는 프로토타입용 값이고, 배와 무기의 이름과 맵의 배치는 이 게임에서 새로 지은 것입니다.

use crate::ids::{Team, WeaponId};
use crate::math::Vec2;
use crate::ship::MOUNTS;
use crate::structures::StructureKind;

#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    /// 서버가 1초에 실행하는 Tick의 수입니다. Tick 속도는 이 값 하나로만 정합니다.
    pub tick_hz: u32,
    /// 한 진영에 들어올 수 있는 배의 수입니다.
    pub team_size: usize,
    pub ship: ShipTuning,
    /// 무기 표입니다. `WeaponId`가 이 표의 행 번호입니다.
    pub weapons: Vec<WeaponSpec>,
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
    pub max_hp: u32,
    /// 1초에 움직이는 거리입니다.
    pub speed: f32,
    /// 이 거리 안의 적이 배의 진영에 보입니다.
    pub vision: f32,
    /// 배가 처음 가지는 무기입니다. 앞 칸부터 차례로 답니다.
    pub start_weapons: Vec<WeaponId>,
    /// 가라앉은 배가 시작 자리에 다시 나타날 때까지의 Tick 수입니다.
    pub respawn_ticks: u32,
    /// 이 배를 가라앉힌 배가 받는 골드입니다.
    pub bounty: u32,
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StructureSite {
    pub kind: StructureKind,
    pub team: Team,
    pub pos: Vec2,
    /// 항구가 맡은 라인의 번호입니다. 항구에만 있습니다.
    pub lane: Option<usize>,
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
        if !is_positive(self.ship.speed) {
            return Err("ship speed must be positive".to_string());
        }
        let kinds = [
            StructureKind::Tower,
            StructureKind::Harbor,
            StructureKind::Base,
        ];
        let structures = kinds.map(|kind| self.structures.of(kind));
        let mut max_hps = vec![self.ship.max_hp];
        let mut visions = vec![self.ship.vision];
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
        if self.ship.start_weapons.len() > MOUNTS {
            return Err(format!("a ship carries at most {MOUNTS} weapons"));
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
            .all(|p| self.contains(p))
        {
            return Err("spawns, lanes and structures must be inside the map".to_string());
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
        Ok(())
    }
}

impl Default for Tuning {
    /// 64 x 48 맵입니다. 두 진영의 본진이 동서 끝에 있고, 북쪽과 남쪽에 라인이 하나씩 있습니다.
    /// 라인마다 진영별로 항구 하나와 타워 하나가 있습니다.
    fn default() -> Tuning {
        let gun = |name, damage, range, cooldown_ticks, shot_speed, hits| WeaponSpec {
            name,
            damage,
            range,
            cooldown_ticks,
            shot_speed,
            hits,
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
                max_hp: 400,
                speed: 4.0,
                vision: 9.0,
                start_weapons: vec![WeaponId(0)],
                respawn_ticks: 200,
                bounty: 200,
            },
            weapons: vec![
                gun("Deck Gun", 20, 5.5, 30, 14.0, both),
                gun("Swivel Gun", 4, 3.5, 3, 18.0, both),
                gun("Long Nine", 30, 12.0, 25, 12.0, Hits::ShipsOnly),
            ],
            structures: StructureTable {
                tower: StructureSpec {
                    max_hp: 3000,
                    vision: 9.0,
                    weapon: Some(gun("Tower Battery", 30, 8.0, 30, 14.0, Hits::ShipsOnly)),
                    bounty: 300,
                },
                harbor: StructureSpec {
                    max_hp: 5000,
                    vision: 9.0,
                    weapon: None,
                    bounty: 500,
                },
                base: StructureSpec {
                    max_hp: 8000,
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
            },
        }
    }
}
