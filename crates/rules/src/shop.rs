//! 상점입니다. 자기 진영의 서 있는 본진과 항구 가까이에 떠 있는 배가 무기와 장비를 사고팔고, 무기를 조합하고, 선체를 바꿉니다. 본진 가까이에서는 수리를 받습니다.

use crate::command::Refusal;
use crate::ids::{GearId, HullId, RecipeId, ShipId, SkillId, WeaponId};
use crate::ship::{Item, Mount, Ship};
use crate::structures::StructureKind;
use crate::trade;
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

/// 배가 자기 진영의 서 있는 본진에서 상점 거리 안에 있는지입니다. 여기에 떠 있는 배가 수리를 받습니다.
pub(crate) fn at_dock(world: &World, tuning: &Tuning, ship: &Ship) -> bool {
    world.structures.values().any(|site| {
        site.kind == StructureKind::Base
            && site.hp > 0
            && site.team == ship.team
            && site.pos.distance(ship.pos) <= tuning.shop.range
    })
}

/// 자기 진영의 본진에 떠 있는 배를 1초에 한 번 수리합니다. 최대 HP의 일정한 비율을 회복하므로, 큰 배도 같은 시간에 수리됩니다.
pub(crate) fn repair(world: &mut World, tuning: &Tuning, tick: u64) {
    if !tick.is_multiple_of(u64::from(tuning.tick_hz)) {
        return;
    }
    let docked = |ship: &&Ship| ship.is_afloat() && at_dock(world, tuning, ship);
    let docked: Vec<ShipId> = world
        .ships
        .values()
        .filter(docked)
        .map(|ship| ship.id)
        .collect();
    for id in docked {
        if let Some(ship) = world.ships.get_mut(&id) {
            let max_hp = ship.max_hp(tuning);
            let percent = u64::from(tuning.shop.repair_percent_per_sec);
            let repaired = (u64::from(max_hp) * percent).div_ceil(100) as u32;
            ship.hp = (ship.hp + repaired).min(max_hp);
        }
    }
}

/// 상점을 쓸 수 있는 배를 돌려줍니다. 가라앉은 배와 상점에서 먼 배는 거절합니다.
pub(crate) fn customer<'a>(
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
    let spec = tuning.weapons.get(usize::from(weapon.0));
    let spec = spec.ok_or(Refusal::UnknownItem)?;
    if !spec.buyable {
        return Err(Refusal::NotForSale);
    }
    let ship = customer(world, tuning, id)?;
    mount_bought(ship, tuning, Item::Weapon(weapon), spec.price)
}

/// 장비를 사서 첫 빈 칸에 답니다. HP는 최대 HP에 대한 비율을 유지합니다.
pub(crate) fn buy_gear(
    world: &mut World,
    tuning: &Tuning,
    id: ShipId,
    gear: GearId,
) -> Result<(), Refusal> {
    let spec = tuning.gear_of(gear).ok_or(Refusal::UnknownItem)?;
    let ship = customer(world, tuning, id)?;
    let before = ship.max_hp(tuning);
    mount_bought(ship, tuning, Item::Gear(gear), spec.price)?;
    keep_hp_share(ship, tuning, before);
    Ok(())
}

/// 최대 HP가 `before`에서 바뀐 뒤에 HP를 같은 비율로 맞춥니다.
/// 최대 HP가 늘 때는 올림하고 줄 때는 내림해서, 늘렸다가 되돌려도 HP가 늘지 않습니다.
fn keep_hp_share(ship: &mut Ship, tuning: &Tuning, before: u32) {
    let after = u64::from(ship.max_hp(tuning));
    let before = u64::from(before);
    let scaled = u64::from(ship.hp) * after;
    ship.hp = if after >= before {
        scaled.div_ceil(before) as u32
    } else {
        ((scaled / before) as u32).max(ship.hp.min(1))
    };
}

/// 값을 내고 아이템을 선체의 첫 빈 칸에 답니다. 빈 칸이 없거나 골드가 모자라면 거절합니다.
pub(crate) fn mount_bought(
    ship: &mut Ship,
    tuning: &Tuning,
    item: Item,
    price: u32,
) -> Result<(), Refusal> {
    let usable = &mut ship.mounts[..tuning.hull(ship.hull).mounts];
    let free = usable.iter_mut().find(|mount| mount.is_none());
    let free = free.ok_or(Refusal::NoFreeMount)?;
    if ship.gold < price {
        return Err(Refusal::NotEnoughGold);
    }
    ship.gold -= price;
    *free = Some(Mount::new(item));
    Ok(())
}

/// 칸의 아이템을 팝니다. HP는 최대 HP에 대한 비율을 유지합니다.
pub(crate) fn sell(
    world: &mut World,
    tuning: &Tuning,
    id: ShipId,
    slot: usize,
) -> Result<(), Refusal> {
    let ship = customer(world, tuning, id)?;
    let before = ship.max_hp(tuning);
    let mount = ship.mounts.get_mut(slot).and_then(Option::take);
    let mount = mount.ok_or(Refusal::EmptyMount)?;
    let price = match mount.item {
        Item::Weapon(weapon) => tuning.weapons[usize::from(weapon.0)].price,
        Item::Gear(gear) => tuning.gear_of(gear).map_or(0, |gear| gear.price),
        Item::Contract(trade) | Item::Cargo(trade) => {
            tuning.trade_of(trade).map_or(0, |trade| trade.price)
        }
    };
    ship.gold += refund(price, tuning);
    keep_hp_share(ship, tuning, before);
    Ok(())
}

/// 재료 무기를 결과 무기 하나로 바꿉니다. 결과 무기는 재료가 있던 칸 가운데 가장 앞의 칸에 달리고, 바로 쏠 수 있습니다.
pub(crate) fn combine(
    world: &mut World,
    tuning: &Tuning,
    id: ShipId,
    recipe: RecipeId,
) -> Result<(), Refusal> {
    let ship = customer(world, tuning, id)?;
    let recipe = tuning.recipes.get(usize::from(recipe.0));
    let recipe = recipe.ok_or(Refusal::UnknownItem)?;
    // 재료마다 아직 고르지 않은 칸 가운데 그 무기가 달린 칸을 고릅니다.
    let mut slots: Vec<usize> = Vec::new();
    for part in &recipe.parts {
        let holds = |slot: &usize| {
            let held = ship.mounts[*slot].and_then(|mount| mount.weapon());
            !slots.contains(slot) && held == Some(*part)
        };
        let slot = (0..ship.mounts.len()).find(holds);
        slots.push(slot.ok_or(Refusal::MissingParts)?);
    }
    if ship.gold < recipe.price {
        return Err(Refusal::NotEnoughGold);
    }
    ship.gold -= recipe.price;
    for slot in &slots {
        ship.mounts[*slot] = None;
    }
    if let Some(first) = slots.into_iter().min() {
        ship.mounts[first] = Some(Mount::new(Item::Weapon(recipe.result)));
    }
    Ok(())
}

/// 선체를 바꿉니다. 지금 선체의 가격의 일부를 돌려받고 새 선체의 가격을 냅니다. HP는 최대 HP에 대한 비율을 유지합니다.
/// 새 선체에 없는 스킬은 배우지 않은 스킬이 되고, 그 스킬 포인트는 다시 쓸 수 있습니다.
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
    if !new.trader && trade::carries_freight(ship) {
        return Err(Refusal::CarryingFreight);
    }
    let old = tuning.hull(ship.hull);
    let funds = ship.gold + refund(old.price, tuning);
    if funds < new.price {
        return Err(Refusal::NotEnoughGold);
    }
    ship.gold = funds - new.price;
    let before = ship.max_hp(tuning);
    ship.hull = hull;
    keep_hp_share(ship, tuning, before);
    // 새 선체에 없는 스킬에 넣은 스킬 포인트는 돌아옵니다.
    for (index, skill) in ship.skills.iter_mut().enumerate() {
        if !new.skills.contains(&SkillId(index as u16)) {
            skill.rank = 0;
        }
    }
    Ok(())
}
