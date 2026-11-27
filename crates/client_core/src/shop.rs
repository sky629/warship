//! 상점 창에 보여 줄 줄과, 줄에 커서를 올리면 보여 줄 설명입니다. 묶음마다 제목 줄이 있습니다.
//! 첫 열은 무기, 장비, 소모품, 조합이고, 둘째 열은 선체와 교역입니다.

use warship_protocol::{Welcome, WireCommand, WireShop, WireShot};

/// 상점 창의 줄 하나입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct ShopRow {
    pub label: String,
    /// 이 줄을 누르면 보내는 명령입니다. 제목 줄에는 없습니다.
    pub order: Option<WireCommand>,
    /// 이 줄에 커서를 올리면 보여 주는 설명입니다. 없으면 빈 글입니다.
    pub detail: String,
    /// 이 줄의 명령에 드는 골드입니다. 선체는 지금 선체의 값을 일부 돌려받으므로 여기에 적지 않습니다.
    pub cost: Option<u32>,
}

impl ShopRow {
    fn heading(label: &str) -> ShopRow {
        ShopRow {
            label: label.to_string(),
            detail: String::new(),
            order: None,
            cost: None,
        }
    }

    fn item(label: String, detail: String, order: WireCommand, cost: Option<u32>) -> ShopRow {
        ShopRow {
            label: format!("  {label}"),
            detail,
            order: Some(order),
            cost,
        }
    }
}

/// 상점 창의 목록입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct ShopList {
    pub rows: Vec<ShopRow>,
    /// 둘째 열이 시작하는 줄의 번호입니다.
    pub second_column: usize,
}

impl ShopList {
    /// 열마다 있는 줄의 수입니다.
    pub fn columns(&self) -> [usize; 2] {
        [self.second_column, self.rows.len() - self.second_column]
    }
}

/// 상점 창의 줄을 위에서부터 차례로 만듭니다.
pub fn shop_list(welcome: &Welcome) -> ShopList {
    let weapon_name = |id: u16| {
        let weapon = welcome.weapons.get(usize::from(id));
        weapon.map_or("?", |weapon| weapon.name.as_str())
    };
    // 무기의 피해, 사거리, 다시 쏘기까지의 시간, 공격 방식입니다.
    let weapon_detail = |id: u16| {
        let weapon = welcome.weapons.get(usize::from(id));
        weapon.map_or(String::new(), |weapon| {
            let reload = weapon.cooldown_ticks as f32 / welcome.tick_hz.max(1) as f32;
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
                "damage {}, range {}, reload {reload}s{shot}",
                weapon.damage, weapon.range
            )
        })
    };
    let hired = welcome.creeps.get(usize::from(welcome.hire.creep));
    let hired = hired.map_or("?", |creep| creep.name.as_str());
    // 상점마다 그 상점이 파는 것을 무기, 장비, 소모품, 조합, 선체, 용병의 순서로 놓습니다.
    let stock_rows = |shop: &WireShop, rows: &mut Vec<ShopRow>| {
        rows.push(ShopRow::heading(&shop.name));
        for &weapon in &shop.weapons {
            let price = welcome.weapons.get(usize::from(weapon));
            let price = price.map_or(0, |weapon| weapon.price);
            let label = format!("{}  {price}g", weapon_name(weapon));
            let order = WireCommand::Buy { weapon };
            rows.push(ShopRow::item(
                label,
                weapon_detail(weapon),
                order,
                Some(price),
            ));
        }
        for &index in &shop.gear {
            let Some(gear) = welcome.gear.get(usize::from(index)) else {
                continue;
            };
            let (name, price, effect) = (&gear.name, gear.price, &gear.effect);
            let label = format!("{name}  {price}g  ({effect})");
            let order = WireCommand::BuyGear { gear: index };
            rows.push(ShopRow::item(label, String::new(), order, Some(price)));
        }
        for &index in &shop.supplies {
            let Some(supply) = welcome.supplies.get(usize::from(index)) else {
                continue;
            };
            let (name, price, effect) = (&supply.name, supply.price, &supply.effect);
            let label = format!("{name}  {price}g  ({effect}, one use)");
            let order = WireCommand::BuySupply { supply: index };
            rows.push(ShopRow::item(label, String::new(), order, Some(price)));
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
            let parts = parts.collect::<Vec<_>>().join(" + ");
            let result = weapon_name(recipe.result);
            let label = format!("{result} = {parts} + {}g", recipe.price);
            let order = WireCommand::Combine { recipe: index };
            let detail = weapon_detail(recipe.result);
            rows.push(ShopRow::item(label, detail, order, Some(recipe.price)));
        }
        let hulls = welcome.hulls.iter().enumerate();
        for (index, hull) in hulls.filter(|_| shop.hulls) {
            let traits = match hull.traits.as_str() {
                "" => String::new(),
                traits => format!(", {traits}"),
            };
            let label = format!(
                "{}  {}g  ({} HP, {} mounts{traits})",
                hull.name, hull.price, hull.max_hp, hull.mounts
            );
            let order = WireCommand::ChangeHull { hull: index as u16 };
            let skills = hull.skills.iter().map(|skill| {
                let skill = welcome.skills.get(usize::from(*skill));
                skill.map_or("?", |skill| skill.name.as_str())
            });
            let detail = format!("skills: {}", skills.collect::<Vec<_>>().join(", "));
            rows.push(ShopRow::item(label, detail, order, None));
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
            let label = format!(
                "Hire for the {place} lane  {}g  ({} x {hired})",
                welcome.hire.price, welcome.hire.count
            );
            let order = WireCommand::Hire { lane: index as u8 };
            let detail = "the creeps come out of your harbor on that lane".to_string();
            rows.push(ShopRow::item(
                label,
                detail,
                order,
                Some(welcome.hire.price),
            ));
        }
    };
    // 첫째 열은 자기 진영의 상점이고, 둘째 열은 교역과 어느 진영의 것도 아닌 항구입니다.
    let mut rows = Vec::new();
    let mut shops = welcome.shops.iter();
    if let Some(home) = shops.next() {
        stock_rows(home, &mut rows);
    }
    let second_column = rows.len();
    rows.push(ShopRow::heading("Trade (a trader hull at your base)"));
    for (index, trade) in welcome.trades.iter().enumerate() {
        let port = welcome.shops.get(usize::from(trade.port) + 1);
        let port = port.map_or("?", |port| port.name.as_str());
        let label = format!(
            "{}  {}g  ({port}, pays {}g)",
            trade.name, trade.price, trade.reward
        );
        let order = WireCommand::Contract {
            trade: index as u16,
        };
        rows.push(ShopRow::item(
            label,
            String::new(),
            order,
            Some(trade.price),
        ));
    }
    for shop in shops {
        stock_rows(shop, &mut rows);
    }
    ShopList {
        rows,
        second_column,
    }
}
