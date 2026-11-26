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
    let broken: [fn(&mut Tuning); 13] = [
        |t| t.tick_hz = 0,
        |t| t.team_size = 0,
        |t| t.weapons[0].cooldown_ticks = 0,
        |t| t.weapons[1].range = f32::NAN,
        |t| t.ship.start_weapons = vec![WeaponId(99)],
        |t| t.ship.start_weapons = vec![WeaponId(0); 7],
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
    ];
    for adjust in broken {
        let mut tuning = Tuning::default();
        adjust(&mut tuning);
        assert!(tuning.validate().is_err());
    }
}
