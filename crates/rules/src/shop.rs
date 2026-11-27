//! 상점입니다. 자기 진영의 서 있는 본진과 항구 가까이에 떠 있는 배가 무기를 사고팔고 선체를 바꾸고, 수리를 받습니다.

use crate::command::Refusal;
use crate::ids::{HullId, ShipId, WeaponId};
use crate::ship::{Mount, Ship};
use crate::structures::StructureKind;
use crate::tuning::Tuning;
use crate::world::World;

/// 배가 자기 진영의 서 있는 본진이나 항구에서 상점 거리 안에 있는지입니다.
pub(crate) fn at_shop(world: &World, tuning: &Tuning, ship: &Ship) -> bool {
    world.structures.values().any(|site| {
        site.kind != StructureKind::Tower
            && site.hp > 0
            && site.team == ship.team
            && site.pos.distance(ship.pos) <= tuning.shop.range
    })
}

/// 상점에 떠 있는 배를 1초에 한 번 수리합니다.
pub(crate) fn repair(world: &mut World, tuning: &Tuning, tick: u64) {
    if !tick.is_multiple_of(u64::from(tuning.tick_hz)) {
        return;
    }
    let docked = |ship: &&Ship| ship.is_afloat() && at_shop(world, tuning, ship);
    let docked: Vec<ShipId> = world
        .ships
        .values()
        .filter(docked)
        .map(|ship| ship.id)
        .collect();
    for id in docked {
        if let Some(ship) = world.ships.get_mut(&id) {
            let max_hp = tuning.hull(ship.hull).max_hp;
            ship.hp = (ship.hp + tuning.shop.repair_per_sec).min(max_hp);
        }
    }
}

/// 상점을 쓸 수 있는 배를 돌려줍니다. 가라앉은 배와 상점에서 먼 배는 거절합니다.
fn customer<'a>(
    world: &'a mut World,
    tuning: &Tuning,
    id: ShipId,
) -> Result<&'a mut Ship, Refusal> {
    let ship = world.ships.get(&id).ok_or(Refusal::Sunk)?;
    if !ship.is_afloat() {
        return Err(Refusal::Sunk);
    }
    if !at_shop(world, tuning, ship) {
        return Err(Refusal::NoShopInRange);
    }
    world.ships.get_mut(&id).ok_or(Refusal::Sunk)
}

fn refund(price: u32, tuning: &Tuning) -> u32 {
    (u64::from(price) * u64::from(tuning.shop.refund_percent) / 100) as u32
}

pub(crate) fn buy(
    world: &mut World,
    tuning: &Tuning,
    id: ShipId,
    weapon: WeaponId,
) -> Result<(), Refusal> {
    let ship = customer(world, tuning, id)?;
    let spec = tuning.weapons.get(usize::from(weapon.0));
    let spec = spec.ok_or(Refusal::UnknownItem)?;
    let usable = &mut ship.mounts[..tuning.hull(ship.hull).mounts];
    let free = usable.iter_mut().find(|mount| mount.is_none());
    let free = free.ok_or(Refusal::NoFreeMount)?;
    if ship.gold < spec.price {
        return Err(Refusal::NotEnoughGold);
    }
    ship.gold -= spec.price;
    *free = Some(Mount {
        weapon,
        ready_at: 0,
        target: None,
    });
    Ok(())
}

pub(crate) fn sell(
    world: &mut World,
    tuning: &Tuning,
    id: ShipId,
    slot: usize,
) -> Result<(), Refusal> {
    let ship = customer(world, tuning, id)?;
    let mount = ship.mounts.get_mut(slot).and_then(Option::take);
    let mount = mount.ok_or(Refusal::EmptyMount)?;
    let price = tuning.weapons[usize::from(mount.weapon.0)].price;
    ship.gold += refund(price, tuning);
    Ok(())
}

/// 선체를 바꿉니다. 지금 선체의 가격의 일부를 돌려받고 새 선체의 가격을 냅니다. HP는 최대 HP에 대한 비율을 유지합니다.
pub(crate) fn change_hull(
    world: &mut World,
    tuning: &Tuning,
    id: ShipId,
    hull: HullId,
) -> Result<(), Refusal> {
    let ship = customer(world, tuning, id)?;
    let new = tuning.hulls.get(usize::from(hull.0));
    let new = new.ok_or(Refusal::UnknownItem)?;
    if hull == ship.hull {
        return Err(Refusal::SameHull);
    }
    if ship.mounts[new.mounts..].iter().any(Option::is_some) {
        return Err(Refusal::MountsInUse);
    }
    let old = tuning.hull(ship.hull);
    let funds = ship.gold + refund(old.price, tuning);
    if funds < new.price {
        return Err(Refusal::NotEnoughGold);
    }
    ship.gold = funds - new.price;
    ship.hp = (u64::from(ship.hp) * u64::from(new.max_hp)).div_ceil(u64::from(old.max_hp)) as u32;
    ship.hull = hull;
    Ok(())
}
