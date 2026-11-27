//! 소모품을 확인합니다. 소모품은 무기나 장비와 같은 칸을 차지하고, 한 번 쓰면 사라집니다.
//! 배는 자기 진영의 본진 옆에 2000골드를 가지고 있습니다.

use super::support::{GUN, KIT, SANDBAGS, SPARE_SAILS, Scenario, assert_close};
use crate::command::{Command, Refusal};
use crate::ids::{ShipId, SupplyId, Team};
use crate::ship::Item;
use crate::structures::StructureKind;

fn customer() -> (Scenario, ShipId) {
    let mut s = Scenario::open_sea();
    s.structure_at(StructureKind::Base, Team::RED, 10.0, 10.0);
    let ship = s.ship_at(Team::RED, 11.0, 10.0);
    s.set_gold(ship, 2000);
    (s, ship)
}

fn item(s: &Scenario, ship: ShipId, slot: usize) -> Option<Item> {
    s.ship(ship).mounts[slot].map(|mount| mount.item)
}

#[test]
fn a_supply_takes_a_mount_and_is_gone_after_one_use() {
    let (mut s, ship) = customer();

    assert_eq!(s.command(ship, Command::BuySupply(KIT)), None);
    assert_eq!(item(&s, ship, 0), Some(Item::Supply(KIT)));
    assert_eq!(s.ship(ship).gold, 1950);

    s.set_ship_hp(ship, 500);
    assert_eq!(s.command(ship, Command::Use { slot: 0 }), None);
    assert_eq!(s.ship(ship).hp, 700);
    assert_eq!(item(&s, ship, 0), None);

    s.command(ship, Command::BuySupply(KIT));
    s.set_ship_hp(ship, 900);
    s.command(ship, Command::Use { slot: 0 });
    assert_eq!(s.ship(ship).hp, 1000, "a repair stops at the highest HP");
}

#[test]
fn a_supply_is_used_anywhere_but_not_by_a_sunk_ship_and_only_a_supply_is_used() {
    let (mut s, ship) = customer();
    s.command(ship, Command::BuySupply(KIT));
    s.command(ship, Command::Buy(GUN));
    assert_eq!(
        s.command(ship, Command::BuySupply(SupplyId(9))),
        Some(Refusal::UnknownItem)
    );
    assert_eq!(
        s.command(ship, Command::Use { slot: 1 }),
        Some(Refusal::NotUsable)
    );
    assert_eq!(
        s.command(ship, Command::Use { slot: 5 }),
        Some(Refusal::EmptyMount)
    );

    // 상점에서 먼 바다에서도 씁니다.
    s.sail_to(ship, 30.0, 10.0);
    s.run_ticks(100);
    s.set_ship_hp(ship, 500);
    assert_eq!(s.command(ship, Command::Use { slot: 0 }), None);
    assert_eq!(s.ship(ship).hp, 700);
}

#[test]
fn a_sunk_ship_keeps_its_supplies_and_cannot_use_them() {
    let (mut s, ship) = customer();
    s.command(ship, Command::BuySupply(KIT));
    s.tower_at(Team::BLUE, 11.0, 14.0);
    s.set_ship_hp(ship, 1);
    s.run_ticks(30);
    assert!(!s.ship(ship).is_afloat());

    assert_eq!(
        s.command(ship, Command::Use { slot: 0 }),
        Some(Refusal::Sunk)
    );
    assert_eq!(item(&s, ship, 0), Some(Item::Supply(KIT)));
}

#[test]
fn spare_sails_speed_the_ship_and_sandbags_cut_the_damage_for_a_while() {
    let (mut s, ship) = customer();
    s.command(ship, Command::BuySupply(SPARE_SAILS));
    s.command(ship, Command::BuySupply(SANDBAGS));
    let tuning = s.sim.tuning().clone();

    s.command(ship, Command::Use { slot: 1 });
    assert_eq!(s.ship(ship).braced(&tuning, 100, s.sim.tick()), 50);

    s.sail_to(ship, 31.0, 10.0);
    s.command(ship, Command::Use { slot: 0 });
    s.run_ticks(20);
    // 20Tick 동안 1초에 4타일의 150%를 갑니다.
    assert_close(s.ship(ship).pos.x, 17.0);
    assert_eq!(s.ship(ship).braced(&tuning, 100, s.sim.tick() + 1), 100);

    s.run_ticks(20);
    assert_close(s.ship(ship).pos.x, 21.0);
}

#[test]
fn a_supply_sells_for_half() {
    let (mut s, ship) = customer();
    s.command(ship, Command::BuySupply(KIT));

    assert_eq!(s.command(ship, Command::Sell { slot: 0 }), None);

    assert_eq!(s.ship(ship).gold, 1975);
}
