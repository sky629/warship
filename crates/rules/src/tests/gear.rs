//! 장비와 무기의 조합을 확인합니다. 무기와 장비는 같은 칸을 나누어 씁니다. 배는 자기 진영의 본진 옆에 2000골드를 가지고 있습니다.

use super::support::{
    BIG_GUN, BIG_HULL, BRACE, CREW, GUN, LONG_GUN, PLANKING, PLATING, SAILS, Scenario, TWIN_GUNS,
    assert_close,
};
use crate::command::{Command, Refusal};
use crate::ids::{GearId, HullId, RecipeId, ShipId, Team, WeaponId};
use crate::ship::Item;
use crate::structures::StructureKind;

fn customer() -> (Scenario, ShipId) {
    let mut s = Scenario::open_sea();
    s.structure_at(StructureKind::Base, Team::RED, 10.0, 10.0);
    let ship = s.ship_at(Team::RED, 11.0, 10.0);
    s.set_gold(ship, 2000);
    (s, ship)
}

fn weapons(s: &Scenario, ship: ShipId) -> Vec<Option<WeaponId>> {
    let mounts = s.ship(ship).mounts.iter();
    mounts
        .map(|mount| mount.and_then(|mount| mount.weapon()))
        .collect()
}

#[test]
fn gear_takes_a_mount_like_a_weapon_and_sells_for_half() {
    let (mut s, ship) = customer();
    assert_eq!(s.command(ship, Command::Buy(GUN)), None);
    assert_eq!(s.command(ship, Command::BuyGear(SAILS)), None);
    assert_eq!(
        s.command(ship, Command::BuyGear(CREW)),
        Some(Refusal::NoFreeMount),
        "the small hull has two mounts for weapons and gear together"
    );
    assert_eq!(
        s.command(ship, Command::BuyGear(GearId(9))),
        Some(Refusal::UnknownItem)
    );
    let items = |s: &Scenario| -> Vec<Option<Item>> {
        let mounts = s.ship(ship).mounts.iter();
        mounts.map(|mount| mount.map(|mount| mount.item)).collect()
    };
    assert_eq!(
        items(&s)[..2],
        [Some(Item::Weapon(GUN)), Some(Item::Gear(SAILS))]
    );
    assert_eq!(s.ship(ship).gold, 1700);

    assert_eq!(s.command(ship, Command::Sell { slot: 1 }), None);
    assert_eq!(items(&s)[..2], [Some(Item::Weapon(GUN)), None]);
    assert_eq!(s.ship(ship).gold, 1800);
    assert_eq!(
        s.command(ship, Command::Sell { slot: 1 }),
        Some(Refusal::EmptyMount)
    );
}

#[test]
fn gear_does_not_shoot() {
    let (mut s, ship) = customer();
    s.ship_at(Team::BLUE, 13.0, 10.0);
    s.command(ship, Command::BuyGear(PLATING));

    s.run_ticks(40);

    // 무기는 쏘면 표적과 다음에 쏠 Tick을 적습니다. 장비의 칸에는 아무것도 적히지 않습니다.
    let mount = s.ship(ship).mounts[0].expect("the gear is mounted");
    assert_eq!((mount.target, mount.ready_at), (None, 0));
}

#[test]
fn gear_is_refused_without_gold_and_away_from_a_shop() {
    let (mut s, ship) = customer();
    s.set_gold(ship, 199);
    assert_eq!(
        s.command(ship, Command::BuyGear(SAILS)),
        Some(Refusal::NotEnoughGold)
    );
    let far = s.ship_at(Team::RED, 30.0, 30.0);
    s.set_gold(far, 2000);
    assert_eq!(
        s.command(far, Command::BuyGear(SAILS)),
        Some(Refusal::NoShopInRange)
    );
}

#[test]
fn planking_keeps_the_share_of_hp_when_bought_and_sold() {
    let (mut s, ship) = customer();
    s.set_ship_hp(ship, 500);

    s.command(ship, Command::BuyGear(PLANKING));
    assert_eq!(s.ship(ship).max_hp(s.sim.tuning()), 1300);
    assert_eq!(s.ship(ship).hp, 650);

    s.command(ship, Command::Sell { slot: 0 });
    assert_eq!(s.ship(ship).hp, 500, "buying and selling does not repair");
}

#[test]
fn buying_and_selling_planking_never_adds_hp() {
    for hp in [1, 2, 3, 333, 999, 1000] {
        let (mut s, ship) = customer();
        s.set_ship_hp(ship, hp);

        s.command(ship, Command::BuyGear(PLANKING));
        s.command(ship, Command::Sell { slot: 0 });

        assert_eq!(s.ship(ship).hp, hp);
    }
}

#[test]
fn sails_make_the_ship_faster() {
    let (mut s, ship) = customer();
    s.command(ship, Command::BuyGear(SAILS));
    s.sail_to(ship, 31.0, 10.0);

    s.run_ticks(20);

    // 1초에 4타일의 125%를 갑니다.
    assert_close(s.ship(ship).pos.x, 16.0);
}

#[test]
fn a_crew_repairs_the_ship_every_second_anywhere_on_the_sea() {
    let (mut s, ship) = customer();
    s.command(ship, Command::BuyGear(CREW));
    s.sim.world_mut().ships.get_mut(&ship).unwrap().pos = crate::math::Vec2::new(30.0, 30.0);
    s.set_ship_hp(ship, 500);

    s.run_ticks(40);

    assert_eq!(s.ship(ship).hp, 510);
}

#[test]
fn plating_reduces_damage_and_adds_to_a_brace_up_to_the_cap() {
    let (mut s, ship) = customer();
    s.command(ship, Command::ChangeHull(BIG_HULL));
    s.command(ship, Command::BuyGear(PLATING));
    s.command(ship, Command::BuyGear(PLATING));
    s.set_xp(ship, 100);
    s.command(ship, Command::Learn(BRACE));
    s.command(ship, Command::Learn(BRACE));
    let tuning = s.sim.tuning().clone();
    let tick = s.sim.tick();

    // 장갑 두 개는 40%를 줄입니다. 2단계의 피해 감소 스킬(80%)을 더하면 120%이지만 80%까지만 줄입니다.
    assert_eq!(s.ship(ship).braced(&tuning, 100, tick), 60);
    s.command(ship, Command::Cast(BRACE));
    assert_eq!(s.ship(ship).braced(&tuning, 100, tick), 20);
}

#[test]
fn a_smaller_hull_needs_room_for_the_gear_and_keeps_the_share_of_hp() {
    let (mut s, ship) = customer();
    s.command(ship, Command::ChangeHull(BIG_HULL));
    s.command(ship, Command::BuyGear(PLANKING));
    s.command(ship, Command::BuyGear(SAILS));
    s.command(ship, Command::Buy(GUN));

    let small = Command::ChangeHull(HullId(0));
    assert_eq!(s.command(ship, small), Some(Refusal::MountsInUse));
    s.command(ship, Command::Sell { slot: 2 });
    assert_eq!(s.command(ship, small), None);

    // 최대 HP는 2300에서 1300이 되고, HP는 같은 비율을 유지합니다.
    assert_eq!(s.ship(ship).hp, 1300);
}

#[test]
fn two_guns_and_the_recipe_price_become_one_big_gun_in_the_first_of_their_mounts() {
    let (mut s, ship) = customer();
    s.command(ship, Command::ChangeHull(BIG_HULL));
    s.command(ship, Command::Buy(LONG_GUN));
    s.command(ship, Command::Buy(GUN));
    s.command(ship, Command::Buy(GUN));
    let gold = s.ship(ship).gold;

    assert_eq!(s.command(ship, Command::Combine(TWIN_GUNS)), None);

    assert_eq!(
        weapons(&s, ship)[..4],
        [Some(LONG_GUN), Some(BIG_GUN), None, None]
    );
    assert_eq!(s.ship(ship).gold, gold - 50);
    // 조합한 무기는 값(250)의 반에 팔립니다.
    s.command(ship, Command::Sell { slot: 1 });
    assert_eq!(s.ship(ship).gold, gold - 50 + 125);
}

#[test]
fn a_recipe_is_refused_without_its_parts_or_its_price_and_its_result_is_not_for_sale() {
    let (mut s, ship) = customer();
    s.command(ship, Command::Buy(GUN));
    assert_eq!(
        s.command(ship, Command::Combine(TWIN_GUNS)),
        Some(Refusal::MissingParts),
        "one gun is not two"
    );
    assert_eq!(
        s.command(ship, Command::Combine(RecipeId(9))),
        Some(Refusal::UnknownItem)
    );
    assert_eq!(
        s.command(ship, Command::Buy(BIG_GUN)),
        Some(Refusal::NotForSale)
    );

    s.command(ship, Command::Buy(GUN));
    s.set_gold(ship, 49);
    assert_eq!(
        s.command(ship, Command::Combine(TWIN_GUNS)),
        Some(Refusal::NotEnoughGold)
    );
    assert_eq!(weapons(&s, ship)[..2], [Some(GUN), Some(GUN)]);
}
