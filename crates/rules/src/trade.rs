//! 교역입니다. 자기 진영의 본진에서 계약을 사고, 교역항에서 화물을 싣고, 본진에 돌아오면 보상을 받습니다.
//! 계약과 화물은 무기나 장비와 같은 칸을 차지하고, 배가 가라앉으면 사라집니다.

use crate::command::Refusal;
use crate::ids::{ShipId, TradeId};
use crate::note::NoteKind;
use crate::ship::{Item, Mount, Ship};
use crate::shop;
use crate::tuning::Tuning;
use crate::world::World;

/// 계약을 사서 첫 빈 칸에 답니다. 본진 가까이에 떠 있는 교역선만 삽니다.
pub(crate) fn contract(
    world: &mut World,
    tuning: &Tuning,
    id: ShipId,
    trade: TradeId,
) -> Result<(), Refusal> {
    let spec = tuning.trade_of(trade).ok_or(Refusal::UnknownItem)?;
    shop::customer(world, tuning, id)?;
    let docked = world
        .ships
        .get(&id)
        .is_some_and(|ship| shop::at_dock(world, tuning, ship));
    if !docked {
        return Err(Refusal::NoBaseInRange);
    }
    let ship = world.ships.get_mut(&id).ok_or(Refusal::Sunk)?;
    if !tuning.hull(ship.hull).trader {
        return Err(Refusal::NotATrader);
    }
    shop::mount_bought(ship, tuning, Item::Contract(trade), spec.price)
}

/// 교역항 가까이에 떠 있는 배의 계약을 화물로 바꾸고, 본진 가까이에 떠 있는 배의 화물을 보상으로 바꿉니다.
pub(crate) fn haul(world: &mut World, tuning: &Tuning, tick: u64) {
    let carriers = world.ships.values();
    let carriers = carriers.filter(|ship| ship.is_afloat() && carries_freight(ship));
    let carriers: Vec<(ShipId, bool)> = carriers
        .map(|ship| (ship.id, shop::at_dock(world, tuning, ship)))
        .collect();
    let mut paid = Vec::new();
    for (id, docked) in carriers {
        let Some(ship) = world.ships.get_mut(&id) else {
            continue;
        };
        let pos = ship.pos;
        for slot in &mut ship.mounts {
            let item = slot.map(|mount| mount.item);
            match item {
                Some(Item::Contract(trade)) => {
                    let port = tuning
                        .trade_of(trade)
                        .map(|spec| tuning.map.ports[spec.port].pos);
                    if port.is_some_and(|port| port.distance(pos) <= tuning.shop.range) {
                        *slot = Some(Mount::new(Item::Cargo(trade)));
                    }
                }
                Some(Item::Cargo(trade)) if docked => {
                    let reward = tuning.trade_of(trade).map_or(0, |spec| spec.reward);
                    ship.gold += reward;
                    paid.push((id, pos, reward));
                    *slot = None;
                }
                _ => {}
            }
        }
    }
    for (ship, pos, reward) in paid {
        world.note(tick, pos, reward, NoteKind::Gold { ship });
    }
}

/// 배가 계약이나 화물을 달고 있는지입니다.
pub(crate) fn carries_freight(ship: &Ship) -> bool {
    let freight = |mount: &Mount| matches!(mount.item, Item::Contract(_) | Item::Cargo(_));
    ship.mounts.iter().flatten().any(freight)
}

/// 가라앉는 배의 계약과 화물을 없앱니다.
pub(crate) fn lose_freight(ship: &mut Ship) {
    for slot in &mut ship.mounts {
        if matches!(slot, Some(mount) if matches!(mount.item, Item::Contract(_) | Item::Cargo(_))) {
            *slot = None;
        }
    }
}
