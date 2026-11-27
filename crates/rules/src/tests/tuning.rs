//! 기본 수치가 검사를 통과하는지와, 규칙의 전제를 어기는 수치가 거절되는지 확인합니다.

use crate::ids::WeaponId;
use crate::math::Vec2;
use crate::sim::Sim;
use crate::tuning::Tuning;

#[test]
fn the_default_tuning_is_valid_and_starts_a_room() {
    let tuning = Tuning::default();
    assert_eq!(tuning.validate(), Ok(()));

    let sim = Sim::new(tuning);
    // 진영마다 본진 하나, 라인 둘에 항구와 타워가 하나씩입니다.
    assert_eq!(sim.structures().count(), 10);
}

#[test]
fn a_new_ship_carries_the_start_weapons() {
    let mut sim = Sim::new(Tuning::default());
    let id = sim.add_ship(crate::ids::Team::BLUE);

    let ship = sim.ship(id).unwrap();
    assert_eq!(ship.pos, Vec2::new(57.0, 24.0));
    assert_eq!(ship.hp, 400);
    assert_eq!(ship.mounts[0].unwrap().weapon, WeaponId(0));
    assert!(ship.mounts[1].is_none());
}

#[test]
fn tunings_that_break_the_rules_are_refused() {
    let broken: [fn(&mut Tuning); 27] = [
        |t| t.tick_hz = 0,
        |t| t.team_size = 0,
        |t| t.weapons[0].cooldown_ticks = 0,
        |t| t.weapons[1].range = f32::NAN,
        |t| t.ship.start_weapons = vec![WeaponId(99)],
        // 시작 선체의 칸은 3개입니다.
        |t| t.ship.start_weapons = vec![WeaponId(0); 4],
        |t| t.map.spawns[0] = Vec2::new(-1.0, 0.0),
        |t| t.creeps[0].speed = 0.0,
        |t| t.waves[0].creep = 9,
        |t| t.waves[0].period_ticks = 0,
        |t| t.map.lanes[0].truncate(1),
        // 본진이 둘인 진영과, 없는 라인을 맡은 항구입니다.
        |t| {
            let base = t.map.structures[0];
            t.map.structures.push(base);
        },
        |t| t.map.structures[1].lane = Some(7),
        |t| t.hulls.clear(),
        |t| t.hulls[1].mounts = 7,
        |t| t.hulls[1].max_hp = 0,
        |t| t.hulls[2].speed = 0.0,
        |t| t.shop.refund_percent = 101,
        |t| t.shop.range = 0.0,
        |t| t.growth.level_xp = vec![100, 100],
        |t| t.skills[0].max_rank = 0,
        |t| t.skills[1].cooldown_ticks = 0,
        |t| t.map.islands[0].radius = 0.0,
        // 맵의 가장자리에 붙은 섬, 다른 섬에 붙은 섬, 시작 자리를 덮은 섬, 라인을 막은 섬입니다.
        |t| t.map.islands[0].center = Vec2::new(5.5, 24.0),
        |t| t.map.islands[1].center = Vec2::new(24.5, 24.0),
        |t| t.map.islands[1].center = Vec2::new(9.0, 22.0),
        |t| t.map.islands[1].center = Vec2::new(30.0, 34.0),
    ];
    for adjust in broken {
        let mut tuning = Tuning::default();
        adjust(&mut tuning);
        assert!(tuning.validate().is_err());
    }
}
