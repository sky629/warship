//! 상점 창에 보여 줄 탭과 줄입니다. 탭 하나가 상점 하나의 한 묶음입니다.
//!
//! 자기 진영의 상점(본진과 항구)은 탭이 셋입니다: 무기와 장비(`Armory`), 선체(`Shipyard`), 교역(`Trade`).
//! 어느 진영의 것도 아닌 항구는 항구마다 탭이 하나입니다. 줄마다 아이콘, 이름, 짧은 설명, 값이 있습니다.

use warship_protocol::{Welcome, WireCommand, WireShop, WireShot, WireStructureKind};

use crate::assets::{Icon, IconKind};
use crate::interp::Frame;

/// 상점 창의 줄 하나입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct ShopRow {
    pub icon: Icon,
    pub name: String,
    /// 이름 아래에 적는 짧은 설명입니다.
    pub note: String,
    /// 이 줄에 커서를 올리면 창의 아래에 보여 주는 설명입니다. 없으면 빈 글입니다.
    pub detail: String,
    /// 줄에 적는 값입니다.
    pub price: u32,
    /// 이 줄의 명령에 드는 골드입니다. 선체는 지금 선체의 값을 일부 돌려받으므로 여기에 적지 않습니다.
    pub cost: Option<u32>,
    /// 이 줄을 누르면 보내는 명령입니다.
    pub order: WireCommand,
}

/// 상점 창의 탭 하나입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct ShopTab {
    pub title: String,
    /// 이 탭의 것을 파는 상점입니다. `Welcome::shops`의 자리입니다.
    pub shop: usize,
    pub rows: Vec<ShopRow>,
}

/// 상점 창의 탭을 왼쪽부터 차례로 만듭니다. 줄이 없는 탭은 만들지 않습니다.
pub fn shop_tabs(welcome: &Welcome) -> Vec<ShopTab> {
    let weapon_name = |id: u16| {
        let weapon = welcome.weapons.get(usize::from(id));
        weapon.map_or("?", |weapon| weapon.name.as_str())
    };
    let reload = |ticks: u32| ticks as f32 / welcome.tick_hz.max(1) as f32;
    // 무기의 피해, 사거리, 다시 쏘기까지의 시간, 공격 방식입니다.
    let weapon_detail = |id: u16| {
        let weapon = welcome.weapons.get(usize::from(id));
        weapon.map_or(String::new(), |weapon| {
            let shot = match weapon.shot {
                WireShot::Shell => String::new(),
                WireShot::Scatter { radius } => format!(", scatters {radius} around the target"),
                WireShot::Bomb { radius } => {
                    format!(", bomb {radius} wide, lands where the target was")
                }
                WireShot::Fire {
                    damage_per_sec,
                    secs,
                } => format!(", burns {damage_per_sec} a second for {secs}s"),
            };
            format!(
                "damage {}, range {}, reload {}s{shot}",
                weapon.damage,
                weapon.range,
                reload(weapon.cooldown_ticks)
            )
        })
    };
    let hired = welcome.creeps.get(usize::from(welcome.hire.creep));
    let hired = hired.map_or("?", |creep| creep.name.as_str());
    // 상점이 파는 것을 무기, 장비, 소모품, 조합, 용병의 순서로 놓습니다.
    let goods = |shop: &WireShop| {
        let mut rows = Vec::new();
        for &weapon in &shop.weapons {
            let Some(spec) = welcome.weapons.get(usize::from(weapon)) else {
                continue;
            };
            rows.push(ShopRow {
                icon: Icon::new(IconKind::Weapon, &spec.name),
                name: spec.name.clone(),
                note: format!(
                    "{} dmg, range {}, {}s",
                    spec.damage,
                    spec.range,
                    reload(spec.cooldown_ticks)
                ),
                detail: weapon_detail(weapon),
                price: spec.price,
                cost: Some(spec.price),
                order: WireCommand::Buy { weapon },
            });
        }
        for &index in &shop.gear {
            let Some(gear) = welcome.gear.get(usize::from(index)) else {
                continue;
            };
            rows.push(ShopRow {
                icon: Icon::new(IconKind::Gear, &gear.name),
                name: gear.name.clone(),
                note: gear.effect.clone(),
                detail: String::new(),
                price: gear.price,
                cost: Some(gear.price),
                order: WireCommand::BuyGear { gear: index },
            });
        }
        for &index in &shop.supplies {
            let Some(supply) = welcome.supplies.get(usize::from(index)) else {
                continue;
            };
            rows.push(ShopRow {
                icon: Icon::new(IconKind::Supply, &supply.name),
                name: supply.name.clone(),
                note: format!("{}, one use", supply.effect),
                detail: String::new(),
                price: supply.price,
                cost: Some(supply.price),
                order: WireCommand::BuySupply { supply: index },
            });
        }
        for &index in &shop.recipes {
            let Some(recipe) = welcome.recipes.get(usize::from(index)) else {
                continue;
            };
            // 같은 재료가 여럿이면 수를 앞에 적어서 줄을 짧게 합니다.
            let mut parts: Vec<(usize, u16)> = Vec::new();
            for &part in &recipe.parts {
                match parts.iter_mut().find(|(_, id)| *id == part) {
                    Some((count, _)) => *count += 1,
                    None => parts.push((1, part)),
                }
            }
            let parts = parts.iter().map(|&(count, part)| match count {
                1 => weapon_name(part).to_string(),
                count => format!("{count} x {}", weapon_name(part)),
            });
            let result = weapon_name(recipe.result);
            rows.push(ShopRow {
                icon: Icon::new(IconKind::Weapon, result),
                name: result.to_string(),
                note: format!("combine: {}", parts.collect::<Vec<_>>().join(" + ")),
                detail: weapon_detail(recipe.result),
                price: recipe.price,
                cost: Some(recipe.price),
                order: WireCommand::Combine { recipe: index },
            });
        }
        let lanes = welcome.lanes.iter().enumerate();
        for (index, lane) in lanes.filter(|_| shop.hire) {
            // 라인의 이름은 맵에서 라인이 있는 자리(북쪽, 가운데, 남쪽)입니다.
            let y = lane.iter().map(|point| point.y).sum::<f32>() / lane.len().max(1) as f32;
            let place = match y / welcome.map_height {
                share if share > 2.0 / 3.0 => "North",
                share if share < 1.0 / 3.0 => "South",
                _ => "Middle",
            };
            rows.push(ShopRow {
                icon: Icon::new(IconKind::Hire, hired),
                name: format!("Hire: {place} lane"),
                note: format!("{} x {hired}", welcome.hire.count),
                detail: "the creeps come out of your harbor on that lane".to_string(),
                price: welcome.hire.price,
                cost: Some(welcome.hire.price),
                order: WireCommand::Hire { lane: index as u8 },
            });
        }
        rows
    };
    let hulls = || {
        let hulls = welcome.hulls.iter().enumerate();
        let rows = hulls.map(|(index, hull)| {
            let traits = match hull.traits.as_str() {
                "" => String::new(),
                traits => format!(", {traits}"),
            };
            let skills = hull.skills.iter().map(|skill| {
                let skill = welcome.skills.get(usize::from(*skill));
                skill.map_or("?", |skill| skill.name.as_str())
            });
            ShopRow {
                icon: Icon::new(IconKind::Hull, &hull.name),
                name: hull.name.clone(),
                note: format!("{} HP, {} mounts{traits}", hull.max_hp, hull.mounts),
                detail: format!("skills: {}", skills.collect::<Vec<_>>().join(", ")),
                price: hull.price,
                cost: None,
                order: WireCommand::ChangeHull { hull: index as u16 },
            }
        });
        rows.collect::<Vec<_>>()
    };
    let trades = || {
        let trades = welcome.trades.iter().enumerate();
        let rows = trades.map(|(index, trade)| {
            let port = welcome.shops.get(usize::from(trade.port) + 1);
            let port = port.map_or("?", |port| port.name.as_str());
            ShopRow {
                icon: Icon::new(IconKind::Trade, &trade.name),
                name: trade.name.clone(),
                note: format!("to {port}, pays {}g", trade.reward),
                detail: "a trader hull buys this at your base and unloads it at the port"
                    .to_string(),
                price: trade.price,
                cost: Some(trade.price),
                order: WireCommand::Contract {
                    trade: index as u16,
                },
            }
        });
        rows.collect::<Vec<_>>()
    };
    let mut tabs = Vec::new();
    for (index, shop) in welcome.shops.iter().enumerate() {
        let tab = |title: &str, rows| ShopTab {
            title: title.to_string(),
            shop: index,
            rows,
        };
        // 첫 번째 상점은 자기 진영의 본진과 항구입니다. 파는 것이 많아서 탭을 나눕니다.
        if index == 0 {
            tabs.push(tab("Armory", goods(shop)));
            if shop.hulls {
                tabs.push(tab("Shipyard", hulls()));
            }
            tabs.push(tab("Trade", trades()));
        } else {
            let mut rows = goods(shop);
            if shop.hulls {
                rows.extend(hulls());
            }
            tabs.push(tab(&shop.name, rows));
        }
    }
    tabs.retain(|tab| !tab.rows.is_empty());
    tabs
}

/// 내 배가 쓸 수 있는 상점입니다. `Welcome::shops`의 자리입니다.
/// 떠 있는 배는 상점 거리 안에 있는 상점을 쓰고, 가라앉아 있는 배는 본진의 상점을 씁니다.
pub fn shops_near(welcome: &Welcome, frame: &Frame) -> Vec<usize> {
    let Some(me) = frame.ships.iter().find(|ship| ship.mine) else {
        return Vec::new();
    };
    if me.sunk {
        return vec![0];
    }
    let near = |x: f32, y: f32| (me.pos.x - x).hypot(me.pos.y - y) <= welcome.shop_range;
    let home = frame.structures.iter().any(|site| {
        site.team == welcome.team
            && site.kind != WireStructureKind::Tower
            && near(site.pos.x, site.pos.y)
    });
    let ports = welcome.ports.iter().enumerate();
    let ports = ports.filter(|(_, port)| near(port.x, port.y));
    home.then_some(0)
        .into_iter()
        .chain(ports.map(|(index, _)| index + 1))
        .collect()
}

/// 상점 창을 열 때 보여 줄 탭입니다. 내 배가 있는 상점의 첫 탭이고, 상점에 있지 않으면 첫 탭입니다.
pub fn first_tab(tabs: &[ShopTab], near: &[usize]) -> usize {
    let here = tabs.iter().position(|tab| near.contains(&tab.shop));
    here.unwrap_or(0)
}
