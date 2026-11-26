//! 게임의 모든 수치입니다. 규칙의 다른 모듈은 숫자를 직접 쓰지 않고 이 구조체에서 읽습니다.
//!
//! 거리의 단위는 타일이고, 시간의 단위는 수치마다 적혀 있습니다(초 또는 Tick).
//! 기본 수치(`Tuning::default`)는 프로토타입용 값이고, 배와 무기의 이름은 이 게임에서 새로 지은 것입니다.

use crate::ids::{Team, WeaponId};
use crate::math::Vec2;
use crate::ship::MOUNTS;

#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    /// 서버가 1초에 실행하는 Tick의 수입니다. Tick 속도는 이 값 하나로만 정합니다.
    pub tick_hz: u32,
    /// 한 진영에 들어올 수 있는 배의 수입니다.
    pub team_size: usize,
    pub ship: ShipTuning,
    /// 무기 표입니다. `WeaponId`가 이 표의 행 번호입니다.
    pub weapons: Vec<WeaponSpec>,
    pub tower: TowerTuning,
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

/// 무기가 쏠 수 있는 표적의 종류입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hits {
    ShipsOnly,
    ShipsAndStructures,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TowerTuning {
    pub max_hp: u32,
    /// 이 거리 안의 적이 타워의 진영에 보입니다.
    pub vision: f32,
    /// 타워의 포입니다. 배의 무기와 같은 규칙으로 쏩니다.
    pub weapon: WeaponSpec,
}

/// 맵의 크기와 처음부터 놓여 있는 것입니다. 맵은 (0, 0)에서 (`width`, `height`)까지입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct MapSpec {
    pub width: f32,
    pub height: f32,
    /// 진영마다 배가 처음 놓이는 자리입니다. 순서는 `Team::index`입니다.
    pub spawns: [Vec2; 2],
    pub towers: Vec<TowerSite>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TowerSite {
    pub team: Team,
    pub pos: Vec2,
}

impl MapSpec {
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= 0.0 && p.x <= self.width && p.y >= 0.0 && p.y <= self.height
    }

    /// 맵 안에서 `p`와 가장 가까운 점입니다.
    pub fn clamp(&self, p: Vec2) -> Vec2 {
        Vec2::new(p.x.clamp(0.0, self.width), p.y.clamp(0.0, self.height))
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
        if self.ship.max_hp == 0 || self.tower.max_hp == 0 {
            return Err("max_hp must be at least 1".to_string());
        }
        if !is_positive(self.ship.speed) {
            return Err("ship speed must be positive".to_string());
        }
        if !is_positive(self.ship.vision) || !is_positive(self.tower.vision) {
            return Err("vision must be positive".to_string());
        }
        for weapon in self.weapons.iter().chain([&self.tower.weapon]) {
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
        if !is_positive(self.map.width) || !is_positive(self.map.height) {
            return Err("the map must have a positive size".to_string());
        }
        let sites = self.map.towers.iter().map(|site| site.pos);
        if !sites.chain(self.map.spawns).all(|p| self.map.contains(p)) {
            return Err("spawns and towers must be inside the map".to_string());
        }
        Ok(())
    }
}

impl Default for Tuning {
    fn default() -> Tuning {
        Tuning {
            tick_hz: 20,
            team_size: 5,
            ship: ShipTuning {
                max_hp: 400,
                speed: 4.0,
                vision: 9.0,
                start_weapons: vec![WeaponId(0)],
            },
            weapons: vec![
                WeaponSpec {
                    name: "Deck Gun",
                    damage: 20,
                    range: 5.5,
                    cooldown_ticks: 30,
                    shot_speed: 14.0,
                    hits: Hits::ShipsAndStructures,
                },
                WeaponSpec {
                    name: "Swivel Gun",
                    damage: 4,
                    range: 3.5,
                    cooldown_ticks: 3,
                    shot_speed: 18.0,
                    hits: Hits::ShipsAndStructures,
                },
                WeaponSpec {
                    name: "Long Nine",
                    damage: 30,
                    range: 12.0,
                    cooldown_ticks: 25,
                    shot_speed: 12.0,
                    hits: Hits::ShipsOnly,
                },
            ],
            tower: TowerTuning {
                max_hp: 3000,
                vision: 9.0,
                weapon: WeaponSpec {
                    name: "Tower Battery",
                    damage: 30,
                    range: 8.0,
                    cooldown_ticks: 30,
                    shot_speed: 14.0,
                    hits: Hits::ShipsOnly,
                },
            },
            map: MapSpec {
                width: 64.0,
                height: 48.0,
                spawns: [Vec2::new(6.0, 24.0), Vec2::new(58.0, 24.0)],
                towers: vec![
                    TowerSite {
                        team: Team::RED,
                        pos: Vec2::new(20.0, 24.0),
                    },
                    TowerSite {
                        team: Team::BLUE,
                        pos: Vec2::new(44.0, 24.0),
                    },
                ],
            },
        }
    }
}
