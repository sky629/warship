//! 내 배의 상태를 화면 왼쪽 아래에 글자로 보여 주고, 단축키를 명령으로 보냅니다. 상점의 목록은 상점 창(`shop`)이, 칸의 아이템은 칸 줄(`mounts`)이 보여 줍니다.
//!
//! Q, W, E, R은 지금 선체의 스킬을 차례로 쓰고, Shift와 함께 누르면 스킬 포인트를 넣습니다.
//! 숫자 키는 그 번호의 칸의 소모품을 씁니다. 사고파는 일은 상점 창과 칸 줄의 클릭으로 합니다.
//! Ctrl과 글자, Ctrl과 숫자, F키는 브라우저의 기능(새로 고침, 탭 닫기, 탭 바꾸기)이므로 단축키로 쓰지 않습니다.
//! 조건은 서버가 검사하고, 거절한 이유가 화면에 나옵니다.

use bevy::prelude::*;
use warship_client_core::MyShip;
use warship_protocol::{Welcome, WireCommand, WireRefusal};

use crate::net::{CurrentFrame, Net};

/// 칸의 번호의 키입니다. 가장 큰 선체의 칸이 6개입니다.
const MOUNT_KEYS: [KeyCode; 6] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
];
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
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let mut orders = Vec::new();
    for (slot, key) in MOUNT_KEYS.into_iter().enumerate() {
        if keys.just_pressed(key) {
            orders.push(WireCommand::Use { slot: slot as u8 });
        }
    }
    let me = frame.0.as_ref().and_then(|frame| frame.me.as_ref());
    let hull_skills = match (net.session.welcome(), me) {
        (Some(welcome), Some(me)) => hull_skills(welcome, me),
        _ => &[],
    };
    for (&skill, (key, _)) in hull_skills.iter().zip(SKILL_KEYS) {
        if keys.just_pressed(key) {
            orders.push(if shift {
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
        "{hull_name}   Level {} (xp {})   Skill points {}",
        me.level, me.xp, me.skill_points
    );
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
    lines += "\nB: shop (click a row to buy)   Shift+key: learn skill   Number: use a supply";
    lines += "\nMount: click to use a supply, right-click to sell   Tab: scores";
    if let Some(refusal) = me.refusal {
        lines += &format!("\nRefused: {}", refusal_text(refusal));
    }
    lines
}

fn refusal_text(refusal: WireRefusal) -> &'static str {
    match refusal {
        WireRefusal::Sunk => "the ship is sunk",
        WireRefusal::NoShopInRange => "go near your base, a harbor or a port",
        WireRefusal::UnknownItem => "there is no such item",
        WireRefusal::NotEnoughGold => "not enough gold",
        WireRefusal::NoFreeMount => "every mount is in use",
        WireRefusal::EmptyMount => "that mount is empty",
        WireRefusal::MountsInUse => "sell the items the new hull has no mount for",
        WireRefusal::SameHull => "the ship already has that hull",
        WireRefusal::NoSkillPoint => "no skill point",
        WireRefusal::MaxRank => "the skill is at its highest rank",
        WireRefusal::NotLearned => "learn the skill first",
        WireRefusal::OnCooldown => "not ready yet",
        WireRefusal::NotOnThisHull => "this hull does not have that skill",
        WireRefusal::NotForSale => "that weapon is made by combining, not sold",
        WireRefusal::MissingParts => "mount every part of the recipe first",
        WireRefusal::NoBaseInRange => "go near your base",
        WireRefusal::NotUsable => "only a supply is used",
        WireRefusal::NotATrader => "only a trader hull trades",
        WireRefusal::NotSoldHere => "this shop does not sell that; see the shop window for where",
        WireRefusal::NoHarbor => "your harbor on that lane is gone",
        WireRefusal::CarryingFreight => "sell the contract or cargo before leaving the trader hull",
    }
}
