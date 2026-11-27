//! 상점 창에 보여 줄 줄입니다. 무기, 장비, 조합, 선체, 교역의 순서이고, 묶음마다 제목 줄이 있습니다.

use warship_protocol::{Welcome, WireCommand};

/// 상점 창의 줄 하나입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct ShopRow {
    pub label: String,
    /// 이 줄을 누르면 보내는 명령입니다. 제목 줄에는 없습니다.
    pub order: Option<WireCommand>,
    /// 이 줄의 명령에 드는 골드입니다. 선체는 지금 선체의 값을 일부 돌려받으므로 여기에 적지 않습니다.
    pub cost: Option<u32>,
}

impl ShopRow {
    fn heading(label: &str) -> ShopRow {
        ShopRow {
            label: label.to_string(),
            order: None,
            cost: None,
        }
    }

    fn item(label: String, order: WireCommand, cost: Option<u32>) -> ShopRow {
        ShopRow {
            label: format!("  {label}"),
            order: Some(order),
            cost,
        }
    }
}

/// 상점이 파는 무기의 번호입니다. 무기 표의 순서이고, 조합으로만 얻는 무기는 건너뜁니다.
pub fn weapons_for_sale(welcome: &Welcome) -> impl Iterator<Item = u16> + '_ {
    let weapons = welcome.weapons.iter().enumerate();
    weapons
        .filter(|(_, weapon)| weapon.buyable)
        .map(|(index, _)| index as u16)
}

/// 상점 창의 줄을 위에서부터 차례로 만듭니다. 묶음 안의 번호는 그 묶음의 단축키의 순서입니다.
pub fn shop_rows(welcome: &Welcome) -> Vec<ShopRow> {
    let weapon_name = |id: u16| {
        let weapon = welcome.weapons.get(usize::from(id));
        weapon.map_or("?", |weapon| weapon.name.as_str())
    };
    let mut rows = vec![ShopRow::heading("Weapons")];
    for (index, weapon) in weapons_for_sale(welcome).enumerate() {
        let price = welcome.weapons[usize::from(weapon)].price;
        let label = format!("{}  {}  {price}g", index + 1, weapon_name(weapon));
        rows.push(ShopRow::item(
            label,
            WireCommand::Buy { weapon },
            Some(price),
        ));
    }
    rows.push(ShopRow::heading("Gear"));
    for (index, gear) in welcome.gear.iter().enumerate() {
        let (name, price, effect) = (&gear.name, gear.price, &gear.effect);
        let label = format!("{}  {name}  {price}g  ({effect})", index + 1);
        let order = WireCommand::BuyGear { gear: index as u16 };
        rows.push(ShopRow::item(label, order, Some(price)));
    }
    rows.push(ShopRow::heading("Combine"));
    for (index, recipe) in welcome.recipes.iter().enumerate() {
        let parts: Vec<&str> = recipe.parts.iter().map(|part| weapon_name(*part)).collect();
        let (result, parts) = (weapon_name(recipe.result), parts.join(" + "));
        let label = format!("{}  {result} = {parts} + {}g", index + 1, recipe.price);
        let order = WireCommand::Combine {
            recipe: index as u16,
        };
        rows.push(ShopRow::item(label, order, Some(recipe.price)));
    }
    rows.push(ShopRow::heading("Hulls"));
    for (index, hull) in welcome.hulls.iter().enumerate() {
        let label = format!(
            "{}  {}  {}g  ({} HP, {} mounts)",
            index + 1,
            hull.name,
            hull.price,
            hull.max_hp,
            hull.mounts
        );
        let order = WireCommand::ChangeHull { hull: index as u16 };
        rows.push(ShopRow::item(label, order, None));
    }
    rows.push(ShopRow::heading("Trade (at your base)"));
    for (index, trade) in welcome.trades.iter().enumerate() {
        let label = format!(
            "{}  {}g  (port {}, pays {}g)",
            trade.name,
            trade.price,
            trade.port + 1,
            trade.reward
        );
        let order = WireCommand::Contract {
            trade: index as u16,
        };
        rows.push(ShopRow::item(label, order, Some(trade.price)));
    }
    rows
}
