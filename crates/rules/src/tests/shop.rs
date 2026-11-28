use super::support::{BIG_HULL, GUN, LONG_GUN, Scenario};
use crate::command::{Command, Refusal};
use crate::ids::{HullId, ShipId, Team, WeaponId};
use crate::structures::StructureKind;

/// 붉은 진영의 항구에서 3타일 떨어진 곳에 떠 있는, 골드 1000을 가진 배입니다.
fn customer() -> (Scenario, ShipId) {
    let mut s = Scenario::open_sea();
    s.structure_at(StructureKind::Harbor, Team::RED, 10.0, 10.0);
    let ship = s.ship_at(Team::RED, 13.0, 10.0);
    s.set_gold(ship, 1000);
    (s, ship)
}

fn weapons(s: &Scenario, ship: ShipId) -> Vec<Option<WeaponId>> {
    let mounts = s.ship(ship).mounts.iter();
    mounts
        .map(|mount| mount.and_then(|mount| mount.weapon()))
        .collect()
}

#[test]
fn a_bought_weapon_goes_into_the_first_free_mount_and_costs_its_price() {
    let (mut s, ship) = customer();

    assert_eq!(s.command(ship, Command::Buy(LONG_GUN)), None);
    assert_eq!(s.command(ship, Command::Buy(GUN)), None);

    assert_eq!(weapons(&s, ship)[..2], [Some(LONG_GUN), Some(GUN)]);
    assert_eq!(s.ship(ship).gold, 800);
}

#[test]
fn a_purchase_is_refused_without_gold_a_free_mount_a_shop_or_a_known_weapon() {
    let (mut s, ship) = customer();
    assert_eq!(
        s.command(ship, Command::Buy(WeaponId(9))),
        Some(Refusal::UnknownItem)
    );

    s.set_gold(ship, 99);
    assert_eq!(
        s.command(ship, Command::Buy(GUN)),
        Some(Refusal::NotEnoughGold)
    );

    s.set_gold(ship, 1000);
    s.command(ship, Command::Buy(GUN));
    s.command(ship, Command::Buy(GUN));
    assert_eq!(
        s.command(ship, Command::Buy(GUN)),
        Some(Refusal::NoFreeMount),
        "the small hull has two mounts"
    );

    let far = s.ship_at(Team::RED, 15.1, 10.0);
    s.set_gold(far, 1000);
    assert_eq!(
        s.command(far, Command::Buy(GUN)),
        Some(Refusal::NoShopInRange)
    );
    assert_eq!((s.ship(ship).gold, s.ship(far).gold), (800, 1000));
}

#[test]
fn only_a_standing_harbor_or_base_of_the_own_team_is_a_shop() {
    let mut s = Scenario::open_sea();
    s.tower_at(Team::RED, 10.0, 10.0);
    s.structure_at(StructureKind::Base, Team::BLUE, 10.0, 12.0);
    let ship = s.ship_at(Team::RED, 10.0, 11.0);
    s.set_gold(ship, 1000);
    assert_eq!(
        s.command(ship, Command::Buy(GUN)),
        Some(Refusal::NoShopInRange)
    );

    s.structure_at(StructureKind::Base, Team::RED, 10.0, 14.0);
    assert_eq!(s.command(ship, Command::Buy(GUN)), None);
}

#[test]
fn a_sunk_ship_uses_the_home_shop_wherever_it_sank() {
    let (mut s, ship) = customer();
    let enemy = s.ship_at(Team::BLUE, 14.0, 10.0);
    s.arm(enemy, GUN);
    s.set_ship_hp(ship, 1);
    s.run_ticks(3);
    assert!(!s.ship(ship).is_afloat());

    assert_eq!(s.command(ship, Command::Buy(GUN)), None);
    assert_eq!(s.ship(ship).hp, 0, "a purchase does not raise a sunk ship");

    // 본진에서 먼 자리에서 가라앉아도 본진의 상점을 씁니다.
    let far = s.ship_at(Team::RED, 60.0, 40.0);
    s.set_gold(far, 1000);
    assert_eq!(
        s.command(far, Command::Buy(GUN)),
        Some(Refusal::NoShopInRange)
    );
    s.sim
        .world_mut()
        .damage(crate::weapon::Target::Ship(far), 100_000, u64::MAX);
    assert_eq!(s.command(far, Command::Buy(GUN)), None);
}

#[test]
fn selling_a_weapon_empties_the_mount_and_returns_half_the_price() {
    let (mut s, ship) = customer();
    s.command(ship, Command::Buy(GUN));
    s.command(ship, Command::Buy(LONG_GUN));

    assert_eq!(s.command(ship, Command::Sell { slot: 0 }), None);

    assert_eq!(weapons(&s, ship)[..2], [None, Some(LONG_GUN)]);
    assert_eq!(s.ship(ship).gold, 850);
    assert_eq!(
        s.command(ship, Command::Sell { slot: 0 }),
        Some(Refusal::EmptyMount)
    );
    assert_eq!(
        s.command(ship, Command::Sell { slot: 200 }),
        Some(Refusal::EmptyMount)
    );
}

#[test]
fn a_new_hull_costs_its_price_and_keeps_the_share_of_hp() {
    let (mut s, ship) = customer();
    s.set_ship_hp(ship, 250);

    assert_eq!(s.command(ship, Command::ChangeHull(BIG_HULL)), None);

    let after = s.ship(ship);
    assert_eq!((after.hull, after.hp, after.gold), (BIG_HULL, 500, 600));
    for _ in 0..4 {
        assert_eq!(s.command(ship, Command::Buy(GUN)), None);
    }
    assert_eq!(s.ship(ship).gold, 200);
}

#[test]
fn going_back_to_a_smaller_hull_returns_half_the_price_and_needs_its_extra_mounts_empty() {
    let (mut s, ship) = customer();
    s.command(ship, Command::ChangeHull(BIG_HULL));
    s.command(ship, Command::Buy(GUN));
    s.command(ship, Command::Buy(GUN));
    s.command(ship, Command::Buy(GUN));
    let small = Command::ChangeHull(HullId(0));

    assert_eq!(s.command(ship, small), Some(Refusal::MountsInUse));
    s.command(ship, Command::Sell { slot: 2 });
    assert_eq!(s.command(ship, small), None);

    // 1000 - 400(선체) - 300(무기 3개) + 50(무기 판매) + 200(선체 환급)
    assert_eq!((s.ship(ship).hull, s.ship(ship).gold), (HullId(0), 550));
    assert_eq!(s.ship(ship).hp, 1000);
}

#[test]
fn a_hull_change_is_refused_for_the_same_hull_an_unknown_hull_and_too_little_gold() {
    let (mut s, ship) = customer();
    let change = |s: &mut Scenario, hull| s.command(ship, Command::ChangeHull(HullId(hull)));
    assert_eq!(change(&mut s, 0), Some(Refusal::SameHull));
    assert_eq!(change(&mut s, 7), Some(Refusal::UnknownItem));
    s.set_gold(ship, 399);
    assert_eq!(change(&mut s, 1), Some(Refusal::NotEnoughGold));
    assert_eq!((s.ship(ship).hull, s.ship(ship).gold), (HullId(0), 399));
}

#[test]
fn a_command_with_a_sequence_number_already_handled_is_ignored() {
    let (mut s, ship) = customer();
    s.sim.command(ship, 5, Command::Buy(GUN));

    s.sim.command(ship, 5, Command::Buy(GUN));
    s.sim.command(ship, 4, Command::Buy(GUN));
    s.sim.command(ShipId(99), 6, Command::Buy(GUN));

    assert_eq!(weapons(&s, ship)[..2], [Some(GUN), None]);
    assert_eq!(s.ship(ship).last_command_seq, 5);
}

#[test]
fn a_ship_at_its_base_is_repaired_every_second_up_to_its_hull_hp_but_not_at_a_harbor() {
    let (mut s, at_harbor) = customer();
    s.structure_at(StructureKind::Base, Team::RED, 30.0, 30.0);
    let docked = s.ship_at(Team::RED, 31.0, 30.0);
    s.set_ship_hp(docked, 985);
    s.set_ship_hp(at_harbor, 985);
    assert!(s.sim.at_home(at_harbor) && !s.sim.at_dock(at_harbor));
    assert!(s.sim.at_home(docked) && s.sim.at_dock(docked));

    // 1초에 최대 HP 1000의 1%를 회복합니다.
    s.run_ticks(20);
    assert_eq!((s.ship(docked).hp, s.ship(at_harbor).hp), (995, 985));
    s.run_ticks(20);
    assert_eq!(s.ship(docked).hp, 1000);
}
