//! 교역을 확인합니다. 본진에서 계약을 사고, 교역항에서 화물을 싣고, 본진에 돌아오면 보상을 받습니다.
//! 본진은 (10, 10)에, 교역항은 (30, 10)에 있습니다. 배는 본진 옆에 500골드를 가지고 있고, 수입은 없습니다.

use super::support::{GUN, Scenario, TRADE};
use crate::command::{Command, Refusal};
use crate::ids::{ShipId, Team, TradeId};
use crate::ship::{Item, Life};
use crate::structures::StructureKind;

fn trader() -> (Scenario, ShipId) {
    let mut s = Scenario::with_tuning(|tuning| tuning.economy.income_per_sec = 0);
    s.structure_at(StructureKind::Base, Team::RED, 10.0, 10.0);
    let ship = s.ship_at(Team::RED, 11.0, 10.0);
    s.set_gold(ship, 500);
    (s, ship)
}

fn items(s: &Scenario, ship: ShipId) -> Vec<Option<Item>> {
    let mounts = s.ship(ship).mounts.iter();
    mounts.map(|mount| mount.map(|mount| mount.item)).collect()
}

#[test]
fn a_contract_is_bought_at_the_base_and_takes_a_mount() {
    let (mut s, ship) = trader();

    assert_eq!(s.command(ship, Command::Contract(TRADE)), None);

    assert_eq!(items(&s, ship)[0], Some(Item::Contract(TRADE)));
    assert_eq!(s.ship(ship).gold, 400);
}

#[test]
fn a_contract_is_refused_without_a_base_gold_or_a_free_mount() {
    let (mut s, ship) = trader();
    assert_eq!(
        s.command(ship, Command::Contract(TradeId(9))),
        Some(Refusal::UnknownItem)
    );
    s.set_gold(ship, 99);
    assert_eq!(
        s.command(ship, Command::Contract(TRADE)),
        Some(Refusal::NotEnoughGold)
    );
    s.set_gold(ship, 500);
    s.command(ship, Command::Buy(GUN));
    s.command(ship, Command::Buy(GUN));
    assert_eq!(
        s.command(ship, Command::Contract(TRADE)),
        Some(Refusal::NoFreeMount)
    );

    // 항구는 상점이지만 계약을 팔지 않습니다.
    s.structure_at(StructureKind::Harbor, Team::RED, 30.0, 30.0);
    let away = s.ship_at(Team::RED, 31.0, 30.0);
    s.set_gold(away, 500);
    assert_eq!(s.command(away, Command::Buy(GUN)), None);
    assert_eq!(
        s.command(away, Command::Contract(TRADE)),
        Some(Refusal::NoBaseInRange)
    );
}

#[test]
fn the_port_loads_the_cargo_and_the_base_pays_for_it() {
    let (mut s, ship) = trader();
    // 교역항 옆의 항구는 화물을 받지 않습니다.
    s.structure_at(StructureKind::Harbor, Team::RED, 30.0, 13.0);
    s.command(ship, Command::Contract(TRADE));

    s.run_ticks(20);
    assert_eq!(
        items(&s, ship)[0],
        Some(Item::Contract(TRADE)),
        "the base does not pay for a contract"
    );

    s.sail_to(ship, 30.0, 10.0);
    s.run_ticks(100);
    assert_eq!(items(&s, ship)[0], Some(Item::Cargo(TRADE)));
    assert_eq!(s.ship(ship).gold, 400);

    s.sail_to(ship, 11.0, 10.0);
    s.run_ticks(100);
    assert_eq!(items(&s, ship)[0], None, "the mount is free again");
    assert_eq!(s.ship(ship).gold, 650);
}

#[test]
fn a_sunk_ship_loses_its_contracts_and_cargo_and_keeps_the_rest() {
    let (mut s, ship) = trader();
    s.command(ship, Command::Contract(TRADE));
    s.command(ship, Command::Buy(GUN));
    s.tower_at(Team::BLUE, 11.0, 14.0);
    s.set_ship_hp(ship, 1);

    s.run_ticks(30);

    assert!(matches!(s.ship(ship).life, Life::Sunk { .. }));
    assert_eq!(items(&s, ship)[..2], [None, Some(Item::Weapon(GUN))]);
}

#[test]
fn a_contract_sells_for_half() {
    let (mut s, ship) = trader();
    s.command(ship, Command::Contract(TRADE));

    assert_eq!(s.command(ship, Command::Sell { slot: 0 }), None);

    assert_eq!(items(&s, ship)[0], None);
    assert_eq!(s.ship(ship).gold, 450);
}
