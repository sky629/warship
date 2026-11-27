//! 테스트에서만 쓰는 상황 구성 도구입니다. 개체를 필요한 자리에 놓은 뒤,
//! 서버가 하는 것과 같은 방식(누르고 있는 입력과 `step`)으로 게임을 진행합니다.

use crate::command::{Command, Refusal};
use crate::creep::Creep;
use crate::ids::{
    CreepId, GearId, HullId, RecipeId, ShipId, SkillId, StructureId, Team, TradeId, WeaponId,
};
use crate::input::{PlayerInput, PlayerInputs, Waypoint};
use crate::math::Vec2;
use crate::ship::{Item, Mount, Ship};
use crate::sim::Sim;
use crate::structures::{Structure, StructureKind};
use crate::tuning::{
    CreepSpec, EconomyTuning, GearEffect, GearSpec, GrowthTuning, Hits, HullSpec, MapSpec, Recipe,
    ShipTuning, ShopTuning, SkillEffect, SkillSpec, StructureSpec, StructureTable, TradeSpec,
    Tuning, WeaponSpec,
};

/// 사거리 5, 피해 10, 쿨다운 10Tick이고 배와 건물을 쏘는 포입니다. 포탄은 한 Tick에 1타일을 납니다.
pub(crate) const GUN: WeaponId = WeaponId(0);
/// 사거리 12, 피해 10, 쿨다운 10Tick이고 배만 쏘는 포입니다. 사거리가 배의 시야(9)보다 깁니다.
pub(crate) const LONG_GUN: WeaponId = WeaponId(1);
/// `GUN` 두 개와 50골드로 조합하는 포입니다. 피해가 30이고 상점에서 팔지 않습니다.
pub(crate) const BIG_GUN: WeaponId = WeaponId(2);
pub(crate) const TWIN_GUNS: RecipeId = RecipeId(0);
pub(crate) const PLANKING: GearId = GearId(0);
pub(crate) const SAILS: GearId = GearId(1);
pub(crate) const CREW: GearId = GearId(2);
pub(crate) const PLATING: GearId = GearId(3);
/// 값이 100이고 (30, 10)의 교역항에서 화물을 실어 오면 250골드를 받는 교역입니다.
pub(crate) const TRADE: TradeId = TradeId(0);
pub(crate) const BIG_HULL: HullId = HullId(1);
pub(crate) const REPAIR: SkillId = SkillId(0);
pub(crate) const SALVO: SkillId = SkillId(1);
pub(crate) const SPRINT: SkillId = SkillId(2);
pub(crate) const RELOAD: SkillId = SkillId(3);
pub(crate) const BRACE: SkillId = SkillId(4);
pub(crate) const RALLY: SkillId = SkillId(5);
pub(crate) const SNARE: SkillId = SkillId(6);

pub(crate) struct Scenario {
    pub sim: Sim,
    held: PlayerInputs,
    /// 마지막으로 보낸 명령의 순서 번호입니다.
    command_seq: u32,
}

impl Scenario {
    /// 건물도 라인도 섬도 없는 40 x 40 맵입니다. 배는 HP가 1000이고 무기 없이 놓이며, 무기는 테스트가 `arm`으로 답니다.
    /// 1초는 20Tick이고 배는 1초에 4타일을 갑니다. 배, NPC 배, 건물의 시야는 9입니다.
    /// 타워와 본진의 포는 사거리 6, 피해 15, 쿨다운 10Tick이고 배만 쏩니다. 항구는 쏘지 않습니다. 건물의 HP는 1000입니다.
    /// NPC 배는 HP 100, 1초에 2타일, 포는 `GUN`과 같습니다. NPC 배는 저절로 나오지 않습니다(일정이 없습니다).
    /// 수입은 1초에 2골드이고, 보상은 배 200, NPC 배 20, 타워 300, 항구 500입니다. 가라앉은 배는 100Tick 뒤에 다시 나타납니다.
    /// 무기의 가격은 100이고, 팔거나 선체를 바꾸면 가격의 반을 돌려받습니다. 본진과 항구에서 5타일 안이 상점이고, 본진에서 5타일 안에 있는 배는 1초에 최대 HP의 1%를 회복합니다.
    /// 선체는 HP 1000에 칸 2개인 0번(가격 0)과 HP 2000에 칸 4개인 1번(가격 400)입니다. 무기와 장비가 같은 칸을 씁니다.
    /// 장비의 가격은 200이고, 0번은 최대 HP 300, 1번은 속도 25%, 2번은 1초에 HP 5 회복, 3번은 받는 피해 20% 감소입니다.
    /// `GUN` 두 개와 50골드를 `BIG_GUN`(피해 30, 값 250) 하나로 조합할 수 있습니다.
    /// 교역(`TRADE`)은 계약의 값이 100이고, (30, 10)의 교역항에서 5타일 안에 가면 화물을 싣고, 본진에 돌아오면 250골드를 받습니다.
    /// 누적 경험치 100에 2레벨, 300에 3레벨이 되고, 경험치는 10타일 안의 배가 나누어 받습니다.
    /// 스킬은 쿨다운 100Tick에 2단계까지이고, 0번은 단계마다 HP 100 회복, 1번(`SALVO`)은 4타일 안의 적에 단계마다 피해 30,
    /// 2번(`SPRINT`)은 20Tick 동안 단계마다 속도 50% 증가, 3번(`RELOAD`)은 모든 무기의 쿨다운 제거,
    /// 4번(`BRACE`)은 20Tick 동안 단계마다 받는 피해 40% 감소, 5번(`RALLY`)은 4타일 안의 같은 진영의 배가 단계마다 HP 60 회복,
    /// 6번(`SNARE`)은 4타일 안의 적의 배가 20Tick 동안 단계마다 속도 40% 감소입니다.
    /// 0번 선체는 모든 스킬을 쓰고 교역을 합니다. 1번 선체는 `REPAIR`, `SALVO`, `RELOAD`, `BRACE`만 쓰고 교역을 하지 못합니다.
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
            command_seq: 0,
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
        *free = Some(Mount::new(Item::Weapon(weapon)));
    }

    pub fn tower_at(&mut self, team: Team, x: f32, y: f32) -> StructureId {
        self.structure_at(StructureKind::Tower, team, x, y)
    }

    /// 건물을 최대 HP로 놓습니다. 항구는 0번 라인을 맡습니다.
    pub fn structure_at(&mut self, kind: StructureKind, team: Team, x: f32, y: f32) -> StructureId {
        let max_hp = self.sim.tuning().structures.of(kind).max_hp;
        let structures = &mut self.sim.world_mut().structures;
        let id = StructureId(structures.len() as u32 + 1);
        let structure = Structure {
            id,
            kind,
            team,
            pos: Vec2::new(x, y),
            hp: max_hp,
            lane: (kind == StructureKind::Harbor).then_some(0),
            ready_at: 0,
            target: None,
        };
        structures.insert(id, structure);
        id
    }

    /// 0번 종류의 NPC 배를 0번 라인에 놓습니다. 길의 첫 지점으로 향합니다.
    pub fn creep_at(&mut self, team: Team, x: f32, y: f32) -> CreepId {
        let max_hp = self.sim.tuning().creeps[0].max_hp;
        let creeps = &mut self.sim.world_mut().creeps;
        let id = CreepId(1000 + creeps.len() as u32);
        let creep = Creep {
            id,
            team,
            kind: 0,
            lane: 0,
            leg: 0,
            pos: Vec2::new(x, y),
            hp: max_hp,
            ready_at: 0,
            target: None,
        };
        creeps.insert(id, creep);
        id
    }

    pub fn set_gold(&mut self, ship: ShipId, gold: u32) {
        self.ship_mut(ship).gold = gold;
    }

    pub fn set_xp(&mut self, ship: ShipId, xp: u32) {
        self.ship_mut(ship).xp = xp;
    }

    /// 다음 순서 번호로 명령을 보내고, 거절되었으면 그 이유를 돌려줍니다.
    pub fn command(&mut self, ship: ShipId, command: Command) -> Option<Refusal> {
        self.command_seq += 1;
        self.sim.command(ship, self.command_seq, command);
        assert_eq!(self.ship(ship).last_command_seq, self.command_seq);
        self.ship(ship).refusal
    }

    pub fn set_ship_hp(&mut self, ship: ShipId, hp: u32) {
        self.ship_mut(ship).hp = hp;
    }

    pub fn set_structure_hp(&mut self, structure: StructureId, hp: u32) {
        let structure = self.sim.world_mut().structures.get_mut(&structure);
        structure.expect("the structure stands").hp = hp;
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

fn gun(name: &'static str, range: f32, hits: Hits) -> WeaponSpec {
    WeaponSpec {
        name,
        damage: 10,
        range,
        cooldown_ticks: 10,
        shot_speed: 20.0,
        hits,
        price: 100,
        buyable: true,
    }
}

fn hull(
    name: &'static str,
    price: u32,
    max_hp: u32,
    mounts: usize,
    skills: &[SkillId],
) -> HullSpec {
    HullSpec {
        skills: skills.to_vec(),
        name,
        price,
        max_hp,
        speed: 4.0,
        vision: 9.0,
        mounts,
        armor_percent: 0,
        repair_per_sec: 0,
        trader: false,
    }
}

fn gear(effect: GearEffect) -> GearSpec {
    GearSpec {
        name: "gear",
        price: 200,
        effect,
    }
}

fn skill(effect: SkillEffect) -> SkillSpec {
    SkillSpec {
        name: "skill",
        cooldown_ticks: 100,
        max_rank: 2,
        effect,
    }
}

fn open_sea_tuning() -> Tuning {
    let armed = |bounty| StructureSpec {
        max_hp: 1000,
        vision: 9.0,
        weapon: Some(WeaponSpec {
            damage: 15,
            ..gun("tower gun", 6.0, Hits::ShipsOnly)
        }),
        bounty,
    };
    Tuning {
        tick_hz: 20,
        team_size: 5,
        ship: ShipTuning {
            start_weapons: Vec::new(),
            respawn_ticks: 100,
            bounty: 200,
        },
        hulls: vec![
            HullSpec {
                trader: true,
                ..hull(
                    "small hull",
                    0,
                    1000,
                    2,
                    &[REPAIR, SALVO, SPRINT, RELOAD, BRACE, RALLY, SNARE],
                )
            },
            hull("big hull", 400, 2000, 4, &[REPAIR, SALVO, RELOAD, BRACE]),
        ],
        weapons: vec![
            gun("gun", 5.0, Hits::ShipsAndStructures),
            gun("long gun", 12.0, Hits::ShipsOnly),
            WeaponSpec {
                damage: 30,
                price: 250,
                buyable: false,
                ..gun("big gun", 5.0, Hits::ShipsAndStructures)
            },
        ],
        skills: vec![
            skill(SkillEffect::Repair { hp_per_rank: 100 }),
            skill(SkillEffect::Salvo {
                damage_per_rank: 30,
                radius: 4.0,
            }),
            skill(SkillEffect::Sprint {
                percent_per_rank: 50,
                ticks: 20,
            }),
            skill(SkillEffect::Reload),
            skill(SkillEffect::Brace {
                percent_per_rank: 40,
                ticks: 20,
            }),
            skill(SkillEffect::Rally {
                hp_per_rank: 60,
                radius: 4.0,
            }),
            skill(SkillEffect::Snare {
                percent_per_rank: 40,
                radius: 4.0,
                ticks: 20,
            }),
        ],
        gear: vec![
            gear(GearEffect::Planking { max_hp: 300 }),
            gear(GearEffect::Sails { speed_percent: 25 }),
            gear(GearEffect::Crew { repair_per_sec: 5 }),
            gear(GearEffect::Plating { armor_percent: 20 }),
        ],
        recipes: vec![Recipe {
            parts: vec![GUN, GUN],
            result: BIG_GUN,
            price: 50,
        }],
        trades: vec![TradeSpec {
            name: "trade",
            price: 100,
            reward: 250,
            port: 0,
        }],
        shop: ShopTuning {
            range: 5.0,
            refund_percent: 50,
            repair_percent_per_sec: 1,
        },
        growth: GrowthTuning {
            level_xp: vec![100, 300],
            share_range: 10.0,
        },
        structures: StructureTable {
            tower: armed(300),
            harbor: StructureSpec {
                weapon: None,
                ..armed(500)
            },
            base: armed(0),
        },
        creeps: vec![CreepSpec {
            name: "creep",
            max_hp: 100,
            speed: 2.0,
            vision: 9.0,
            weapon: gun("creep gun", 5.0, Hits::ShipsAndStructures),
            bounty: 20,
        }],
        waves: Vec::new(),
        economy: EconomyTuning { income_per_sec: 2 },
        map: MapSpec {
            width: 40.0,
            height: 40.0,
            spawns: [Vec2::new(1.0, 1.0), Vec2::new(39.0, 39.0)],
            lanes: Vec::new(),
            structures: Vec::new(),
            islands: Vec::new(),
            ports: vec![Vec2::new(30.0, 10.0)],
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
