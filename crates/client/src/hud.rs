//! 내 배의 상태를 화면 왼쪽 아래에 글자로 보여 주고, 단축키를 명령으로 보냅니다. 상점의 목록은 상점 창(`shop`)이 보여 줍니다.
//!
//! 숫자 키는 상점의 그 번호의 무기를 사고, Alt와 숫자 키는 그 번호의 장비를 삽니다. 무기와 장비는 같은 칸에 달립니다.
//! Shift와 숫자 키는 그 번호의 칸의 아이템을 팝니다.
//! Z, X, C, V는 조합 표의 차례대로 무기를 조합합니다. F1부터의 키는 선체를 바꿉니다.
//! Q, W, E, R은 지금 선체의 스킬을 차례로 쓰고, Ctrl과 함께 누르면 스킬 포인트를 넣습니다. 조건은 서버가 검사하고, 거절한 이유가 화면에 나옵니다.

use bevy::prelude::*;
use warship_client_core::{MyShip, weapons_for_sale};
use warship_protocol::{Welcome, WireCommand, WireItem, WireRefusal};

use crate::net::{CurrentFrame, Net};

const DIGITS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];
const HULL_KEYS: [KeyCode; 6] = [
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
];
const RECIPE_KEYS: [KeyCode; 4] = [KeyCode::KeyZ, KeyCode::KeyX, KeyCode::KeyC, KeyCode::KeyV];
const SKILL_KEYS: [(KeyCode, char); 4] = [
    (KeyCode::KeyQ, 'Q'),
    (KeyCode::KeyW, 'W'),
    (KeyCode::KeyE, 'E'),
    (KeyCode::KeyR, 'R'),
];

#[derive(Component)]
pub struct HudText;

pub fn setup(mut commands: Commands) {
    commands.spawn((
        HudText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            bottom: px(10),
            left: px(10),
            ..default()
        },
    ));
}

/// 이번 프레임에 눌린 단축키를 명령으로 보냅니다.
pub fn order_commands(
    keys: Res<ButtonInput<KeyCode>>,
    frame: Res<CurrentFrame>,
    mut net: NonSendMut<Net>,
) {
    let held = |pair: [KeyCode; 2]| keys.any_pressed(pair);
    let shift = held([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let ctrl = held([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let alt = held([KeyCode::AltLeft, KeyCode::AltRight]);
    let welcome = net.session.welcome();
    // 숫자 키의 번호는 상점이 파는 무기의 순서입니다. 조합으로만 얻는 무기는 건너뜁니다.
    let for_sale: Vec<u16> =
        welcome.map_or(Vec::new(), |welcome| weapons_for_sale(welcome).collect());
    let recipes = welcome.map_or(0, |welcome| welcome.recipes.len());
    let mut orders = Vec::new();
    for (index, key) in DIGITS.into_iter().enumerate() {
        if !keys.just_pressed(key) {
            continue;
        }
        let slot = index as u8;
        let order = match (shift, alt) {
            (true, _) => Some(WireCommand::Sell { slot }),
            (false, true) => Some(WireCommand::BuyGear { gear: index as u16 }),
            (false, false) => for_sale
                .get(index)
                .map(|&weapon| WireCommand::Buy { weapon }),
        };
        orders.extend(order);
    }
    for (index, key) in RECIPE_KEYS.into_iter().enumerate().take(recipes) {
        if keys.just_pressed(key) {
            let recipe = index as u16;
            orders.push(WireCommand::Combine { recipe });
        }
    }
    for (index, key) in HULL_KEYS.into_iter().enumerate() {
        if keys.just_pressed(key) {
            orders.push(WireCommand::ChangeHull { hull: index as u16 });
        }
    }
    let me = frame.0.as_ref().and_then(|frame| frame.me.as_ref());
    let hull_skills = match (net.session.welcome(), me) {
        (Some(welcome), Some(me)) => hull_skills(welcome, me),
        _ => &[],
    };
    for (&skill, (key, _)) in hull_skills.iter().zip(SKILL_KEYS) {
        if keys.just_pressed(key) {
            orders.push(if ctrl {
                WireCommand::Learn { skill }
            } else {
                WireCommand::Cast { skill }
            });
        }
    }
    for order in orders {
        net.send_command(order);
    }
}

pub fn show_hud(
    net: Option<NonSend<Net>>,
    frame: Res<CurrentFrame>,
    mut text: Single<&mut Text, With<HudText>>,
) {
    let me = frame.0.as_ref().and_then(|frame| frame.me.as_ref());
    let welcome = net.as_ref().and_then(|net| net.session.welcome());
    let lines = match (welcome, me) {
        (Some(welcome), Some(me)) => hud_lines(welcome, me),
        _ => String::new(),
    };
    if text.0 != lines {
        text.0 = lines;
    }
}

/// 내 배의 선체가 쓰는 스킬의 번호입니다. 단축키의 순서입니다.
fn hull_skills<'a>(welcome: &'a Welcome, me: &MyShip) -> &'a [u16] {
    let hull = welcome.hulls.get(usize::from(me.hull));
    hull.map_or(&[], |hull| hull.skills.as_slice())
}

fn hud_lines(welcome: &Welcome, me: &MyShip) -> String {
    let hull = welcome.hulls.get(usize::from(me.hull));
    let hull_name = hull.map_or("?", |hull| hull.name.as_str());
    let mut lines = format!(
        "{hull_name}   Level {} (xp {})   Skill points {}\nMounts:",
        me.level, me.xp, me.skill_points
    );
    for (slot, item) in me.mounts.iter().enumerate() {
        let name = match item {
            None => Some("-"),
            Some(WireItem::Weapon(weapon)) => {
                let weapon = welcome.weapons.get(usize::from(*weapon));
                weapon.map(|weapon| weapon.name.as_str())
            }
            Some(WireItem::Gear(gear)) => {
                let gear = welcome.gear.get(usize::from(*gear));
                gear.map(|gear| gear.name.as_str())
            }
        };
        lines += &format!("  [{}] {}", slot + 1, name.unwrap_or("?"));
    }
    lines += "\nSkills:";
    for (&id, (_, key)) in hull_skills(welcome, me).iter().zip(SKILL_KEYS) {
        let index = usize::from(id);
        let (Some(spec), Some(skill)) = (welcome.skills.get(index), me.skills.get(index)) else {
            continue;
        };
        lines += &format!("  [{key}] {} {}/{}", spec.name, skill.rank, spec.max_rank);
        if skill.rank > 0 && skill.ready_secs > 0.0 {
            lines += &format!(" ({:.0}s)", skill.ready_secs.ceil());
        }
    }
    lines +=
        "\nB: shop   Number: buy weapon   Alt+number: buy gear   Z, X, C, V: combine   F1-F6: hull";
    lines += "\nShift+number: sell mount   Ctrl+key: learn skill";
    if let Some(refusal) = me.refusal {
        lines += &format!("\nRefused: {}", refusal_text(refusal));
    }
    lines
}

fn refusal_text(refusal: WireRefusal) -> &'static str {
    match refusal {
        WireRefusal::Sunk => "the ship is sunk",
        WireRefusal::NoShopInRange => "go near your base or a harbor",
        WireRefusal::UnknownItem => "there is no such item",
        WireRefusal::NotEnoughGold => "not enough gold",
        WireRefusal::NoFreeMount => "every mount is in use",
        WireRefusal::EmptyMount => "that mount is empty",
        WireRefusal::MountsInUse => "sell the items the new hull has no mount for",
        WireRefusal::SameHull => "the ship already has that hull",
        WireRefusal::NoSkillPoint => "no skill point",
        WireRefusal::MaxRank => "the skill is at its highest rank",
        WireRefusal::NotLearned => "learn the skill first",
        WireRefusal::OnCooldown => "the skill is not ready",
        WireRefusal::NotOnThisHull => "this hull does not have that skill",
        WireRefusal::NotForSale => "that weapon is made by combining, not sold",
        WireRefusal::MissingParts => "mount every part of the recipe first",
    }
}
