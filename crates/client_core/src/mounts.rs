//! 화면 아래의 칸 줄에 보여 줄 것입니다. 칸마다 아이템의 이름과 짧은 안내가 있습니다.

use warship_protocol::{Welcome, WireItem};

use crate::MyShip;
use crate::assets::{Icon, IconKind};
use crate::words::ko;

/// 칸 하나에 보여 주는 것입니다.
#[derive(Clone, Debug, PartialEq)]
pub struct MountView {
    /// 달린 아이템의 아이콘입니다. 빈 칸에는 없습니다.
    pub icon: Option<Icon>,
    /// 아이템의 이름입니다. 빈 칸은 `-`입니다.
    pub name: String,
    /// 이 아이템으로 할 일의 안내입니다. 없으면 빈 글입니다.
    pub note: String,
    /// 눌러서 쓸 수 있는 소모품인지입니다.
    pub usable: bool,
}

/// 내 배의 칸을 앞에서부터 차례로 만듭니다.
pub fn mount_views(welcome: &Welcome, me: &MyShip) -> Vec<MountView> {
    let view = |item: &Option<WireItem>| {
        let named = |name: Option<&String>| name.map_or("?", String::as_str).to_string();
        let (name, note) = match item {
            None => ("-".to_string(), String::new()),
            Some(WireItem::Weapon(weapon)) => {
                let weapon = welcome.weapons.get(usize::from(*weapon));
                (named(weapon.map(|weapon| &weapon.name)), String::new())
            }
            Some(WireItem::Gear(gear)) => {
                let gear = welcome.gear.get(usize::from(*gear));
                (named(gear.map(|gear| &gear.name)), String::new())
            }
            Some(WireItem::Supply(supply)) => {
                let supply = welcome.supplies.get(usize::from(*supply));
                // 소모품은 하나를 쓴 뒤에 얼마 동안 다시 쓰지 못합니다.
                let note = if me.supply_secs > 0.0 {
                    format!("{:.0}초 뒤", me.supply_secs.ceil())
                } else {
                    "클릭: 사용".to_string()
                };
                (named(supply.map(|supply| &supply.name)), note)
            }
            Some(WireItem::Contract(trade)) => {
                let trade = welcome.trades.get(usize::from(*trade));
                (
                    named(trade.map(|trade| &trade.name)),
                    "항구로 운반".to_string(),
                )
            }
            Some(WireItem::Cargo(trade)) => {
                let trade = welcome.trades.get(usize::from(*trade));
                (
                    named(trade.map(|trade| &trade.name)),
                    "본진으로 운반".to_string(),
                )
            }
        };
        let kind = item.map(|item| match item {
            WireItem::Weapon(_) => IconKind::Weapon,
            WireItem::Gear(_) => IconKind::Gear,
            WireItem::Supply(_) => IconKind::Supply,
            WireItem::Contract(_) | WireItem::Cargo(_) => IconKind::Trade,
        });
        MountView {
            // 아이콘의 파일은 서버의 이름에서 나오고, 글자는 한국어 이름입니다.
            icon: kind.map(|kind| Icon::new(kind, &name)),
            name: ko(&name).to_string(),
            note,
            usable: matches!(item, Some(WireItem::Supply(_))),
        }
    };
    me.mounts.iter().map(view).collect()
}
