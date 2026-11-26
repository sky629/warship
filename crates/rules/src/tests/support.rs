//! 테스트에서만 쓰는 상황 구성 도구입니다. 개체를 필요한 자리에 놓은 뒤,
//! 서버가 하는 것과 같은 방식(누르고 있는 입력과 `step`)으로 게임을 진행합니다.

use crate::ids::{ShipId, Team, TowerId, WeaponId};
use crate::input::{PlayerInput, PlayerInputs, Waypoint};
use crate::math::Vec2;
use crate::ship::{Mount, Ship};
use crate::sim::Sim;
use crate::structures::Tower;
use crate::tuning::{Hits, MapSpec, ShipTuning, TowerTuning, Tuning, WeaponSpec};

/// 사거리 5, 피해 10, 쿨다운 10Tick이고 배와 타워를 쏘는 포입니다. 포탄은 한 Tick에 1타일을 납니다.
pub(crate) const GUN: WeaponId = WeaponId(0);
/// 사거리 12, 피해 10, 쿨다운 10Tick이고 배만 쏘는 포입니다. 사거리가 배의 시야(9)보다 깁니다.
pub(crate) const LONG_GUN: WeaponId = WeaponId(1);

pub(crate) struct Scenario {
    pub sim: Sim,
    held: PlayerInputs,
}

impl Scenario {
    /// 타워가 없는 40 x 40 맵입니다. 배는 HP가 1000이고 무기 없이 놓이며, 무기는 테스트가 `arm`으로 답니다.
    /// 1초는 20Tick이고 배는 1초에 4타일을 갑니다. 배와 타워의 시야는 9이고, 타워의 포는 사거리 6, 피해 15, 쿨다운 10Tick입니다.
    pub fn open_sea() -> Scenario {
        Scenario::with_tuning(|_| {})
    }

    /// `open_sea`와 같지만 수치를 먼저 바꿉니다.
    pub fn with_tuning(adjust: impl FnOnce(&mut Tuning)) -> Scenario {
        let mut tuning = open_sea_tuning();
        adjust(&mut tuning);
        tuning.validate().expect("the scenario tuning is valid");
        Scenario {
            sim: Sim::new(tuning),
            held: PlayerInputs::new(),
        }
    }

    pub fn ship_at(&mut self, team: Team, x: f32, y: f32) -> ShipId {
        let id = self.sim.add_ship(team);
        self.ship_mut(id).pos = Vec2::new(x, y);
        id
    }

    /// 배의 첫 빈 칸에 무기를 답니다.
    pub fn arm(&mut self, ship: ShipId, weapon: WeaponId) {
        let free = self
            .ship_mut(ship)
            .mounts
            .iter_mut()
            .find(|mount| mount.is_none())
            .expect("the ship has a free mount");
        *free = Some(Mount {
            weapon,
            ready_at: 0,
            target: None,
        });
    }

    pub fn tower_at(&mut self, team: Team, x: f32, y: f32) -> TowerId {
        let max_hp = self.sim.tuning().tower.max_hp;
        let towers = &mut self.sim.world_mut().towers;
        let id = TowerId(towers.len() as u32 + 1);
        let tower = Tower {
            id,
            team,
            pos: Vec2::new(x, y),
            hp: max_hp,
            ready_at: 0,
            target: None,
        };
        towers.insert(id, tower);
        id
    }

    pub fn set_ship_hp(&mut self, ship: ShipId, hp: u32) {
        self.ship_mut(ship).hp = hp;
    }

    pub fn set_tower_hp(&mut self, tower: TowerId, hp: u32) {
        let tower = self.sim.world_mut().towers.get_mut(&tower);
        tower.expect("the tower stands").hp = hp;
    }

    /// 배가 이 지점으로 가라는 입력을 계속 보냅니다.
    pub fn sail_to(&mut self, ship: ShipId, x: f32, y: f32) {
        let input = PlayerInput {
            seq: 1,
            destination: Waypoint::new(x, y),
        };
        self.held.insert(ship, input);
    }

    /// 배가 멈추라는 입력을 계속 보냅니다.
    pub fn stop(&mut self, ship: ShipId) {
        self.held.insert(ship, PlayerInput::default());
    }

    pub fn run_ticks(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.sim.step(&self.held);
        }
    }

    pub fn ship(&self, id: ShipId) -> &Ship {
        self.sim.ship(id).expect("the ship is in the room")
    }

    fn ship_mut(&mut self, id: ShipId) -> &mut Ship {
        let ship = self.sim.world_mut().ships.get_mut(&id);
        ship.expect("the ship is in the room")
    }
}

fn open_sea_tuning() -> Tuning {
    let gun = |name, range, hits| WeaponSpec {
        name,
        damage: 10,
        range,
        cooldown_ticks: 10,
        shot_speed: 20.0,
        hits,
    };
    Tuning {
        tick_hz: 20,
        ship: ShipTuning {
            max_hp: 1000,
            speed: 4.0,
            vision: 9.0,
            start_weapons: Vec::new(),
        },
        weapons: vec![
            gun("gun", 5.0, Hits::ShipsAndStructures),
            gun("long gun", 12.0, Hits::ShipsOnly),
        ],
        tower: TowerTuning {
            max_hp: 1000,
            vision: 9.0,
            weapon: WeaponSpec {
                damage: 15,
                ..gun("tower gun", 6.0, Hits::ShipsOnly)
            },
        },
        map: MapSpec {
            width: 40.0,
            height: 40.0,
            spawns: [Vec2::new(1.0, 1.0), Vec2::new(39.0, 39.0)],
            towers: Vec::new(),
        },
    }
}

/// 두 값이 0.001 안에서 같은지 확인합니다. 한 Tick의 이동 거리(0.2)는 이진수로 정확히 나타낼 수 없기 때문입니다.
pub(crate) fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-3,
        "expected {expected}, got {actual}"
    );
}
