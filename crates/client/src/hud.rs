//! 화면 위쪽의 상태 막대와 왼쪽 아래의 내 배 패널, 항구의 이름표를 그리고, 단축키를 명령으로 보냅니다. 상점의 목록은 상점 창(`shop`)이, 칸의 아이템은 칸 줄(`mounts`)이 보여 줍니다.
//!
//! Q, W, E, R은 지금 선체의 스킬을 차례로 쓰고, Shift와 함께 누르면 스킬 포인트를 넣습니다.
//! 숫자 키는 그 번호의 칸의 소모품을 씁니다. 사고파는 일은 상점 창과 칸 줄의 클릭으로 합니다.
//! Ctrl과 글자, Ctrl과 숫자, F키는 브라우저의 기능(새로 고침, 탭 닫기, 탭 바꾸기)이므로 단축키로 쓰지 않습니다.
//! 조건은 서버가 검사하고, 거절한 이유가 화면에 나옵니다.

use bevy::prelude::*;
use warship_client_core::{MyShip, SkillSlot, hud_view, top_bar};
use warship_protocol::{Welcome, WireCommand, WireRefusal, WireTeam};

use crate::net::{CurrentFrame, Net};
use crate::ui::{
    self, BAD, BLUE, DIM, GOLD, GOOD, HP, HP_BACK, PANEL, Panel, RED, SLOT, TEXT, XP, XP_BACK,
    words,
};

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

/// 화면 위쪽의 상태 막대입니다.
#[derive(Component)]
pub struct TopRoot;

/// 화면 왼쪽 아래의 내 배 패널입니다.
#[derive(Component)]
pub struct PanelRoot;

/// 거절된 명령의 이유를 보여 주는 글자입니다.
#[derive(Component)]
pub struct ToastText;

/// 항구의 이름표를 담는 바탕입니다.
#[derive(Component)]
pub struct LabelRoot;

/// 내 배 패널의 너비입니다.
const PANEL_WIDTH: f32 = 480.0;

pub fn setup(mut commands: Commands) {
    commands.spawn((
        TopRoot,
        Panel,
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            left: px(0),
            width: percent(100),
            height: px(30),
            padding: UiRect::horizontal(px(12)),
            column_gap: px(22),
            align_items: AlignItems::Center,
            display: Display::None,
            ..default()
        },
        BackgroundColor(PANEL),
    ));
    commands.spawn((
        PanelRoot,
        Panel,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(10),
            left: px(10),
            width: px(PANEL_WIDTH),
            padding: UiRect::all(px(8)),
            row_gap: px(5),
            flex_direction: FlexDirection::Column,
            display: Display::None,
            ..default()
        },
        BackgroundColor(PANEL),
    ));
    commands.spawn((
        ToastText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(BAD),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(176),
            left: px(10),
            ..default()
        },
    ));
    commands.spawn((
        LabelRoot,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
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

/// 방 안에서 위쪽 상태 막대(방, 진영, 골드, 지난 시간, 진영별 가라앉힌 배의 수, 방에 있는 사람의 이름)를 그립니다.
pub fn show_top(
    net: Option<NonSend<Net>>,
    frame: Res<CurrentFrame>,
    root: Single<(Entity, &mut Node), With<TopRoot>>,
    mut commands: Commands,
) {
    let (root, mut node) = root.into_inner();
    commands.entity(root).despawn_children();
    let welcome = net.as_ref().and_then(|net| net.session.welcome());
    let Some((welcome, frame)) = welcome.zip(frame.0.as_ref()) else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    let bar = top_bar(frame.secs, &frame.scores);
    let (team, color) = match welcome.team {
        WireTeam::Red => ("Red team", RED),
        WireTeam::Blue => ("Blue team", BLUE),
        WireTeam::Wild => ("Wild", DIM),
    };
    let gold = frame.me.as_ref().map_or(0, |me| me.gold);
    let roster = net.as_ref().map_or(&[][..], |net| net.session.roster());
    commands.entity(root).with_children(|parent| {
        parent.spawn(words(format!("Room {}", welcome.room), 14.0, DIM));
        parent.spawn(words(team, 14.0, color));
        parent.spawn(words(format!("Gold {gold}"), 16.0, GOLD));
        parent.spawn(words(bar.clock, 16.0, TEXT));
        parent.spawn(words(format!("Red {}", bar.red_kills), 16.0, RED));
        parent.spawn(words(format!("Blue {}", bar.blue_kills), 16.0, BLUE));
        // 방에 있는 사람의 이름입니다. 접속이 끊긴 사람에는 표시를 붙입니다.
        let names = roster.iter().map(|member| {
            let away = if member.connected { "" } else { " (away)" };
            format!("{}{away}", member.name)
        });
        let names = names.collect::<Vec<_>>().join(", ");
        parent.spawn(words(format!("Captains: {names}"), 13.0, DIM));
    });
}

/// 내 배 패널(선체, 레벨, HP 막대, 경험치 막대, 스킬 칸)을 그리고, 거절된 명령의 이유를 그 위에 보여 줍니다.
pub fn show_hud(
    net: Option<NonSend<Net>>,
    frame: Res<CurrentFrame>,
    assets: Res<AssetServer>,
    root: Single<(Entity, &mut Node), With<PanelRoot>>,
    mut toast: Single<&mut Text, With<ToastText>>,
    mut commands: Commands,
) {
    let (root, mut node) = root.into_inner();
    commands.entity(root).despawn_children();
    let me = frame.0.as_ref().and_then(|frame| frame.me.as_ref());
    let welcome = net.as_ref().and_then(|net| net.session.welcome());
    let refusal = me.and_then(|me| me.refusal).map(refusal_text);
    let refusal = refusal.map_or(String::new(), |why| format!("Refused: {why}"));
    if toast.0 != refusal {
        toast.0 = refusal;
    }
    let Some((welcome, me)) = welcome.zip(me) else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    let view = hud_view(welcome, me);
    let bar = |height: f32, share: f32, fill: Color, back: Color| {
        let frame = Node {
            width: percent(100),
            height: px(height),
            ..default()
        };
        let filled = Node {
            width: percent(share.clamp(0.0, 1.0) * 100.0),
            height: percent(100),
            ..default()
        };
        (
            frame,
            BackgroundColor(back),
            children![(filled, BackgroundColor(fill))],
        )
    };
    commands.entity(root).with_children(|parent| {
        let title = Node {
            column_gap: px(14),
            align_items: AlignItems::Baseline,
            ..default()
        };
        parent.spawn(title).with_children(|title| {
            title.spawn(words(view.hull.clone(), 17.0, TEXT));
            title.spawn(words(format!("Level {}", view.level), 14.0, GOLD));
            let hp = format!("HP {} / {}", view.hp, view.max_hp);
            title.spawn(words(hp, 14.0, TEXT));
            if view.skill_points > 0 {
                let points = format!("+{} skill (Shift+key)", view.skill_points);
                title.spawn(words(points, 13.0, GOOD));
            }
        });
        parent.spawn(bar(12.0, view.hp_share(), HP, HP_BACK));
        parent.spawn(bar(5.0, view.xp_share(), XP, XP_BACK));
        let row = Node {
            column_gap: px(4),
            ..default()
        };
        parent.spawn(row).with_children(|row| {
            for slot in &view.skills {
                show_skill(row, &assets, slot);
            }
        });
    });
}

/// 스킬 칸 하나입니다. 아이콘, 단축키와 이름, 단계를 나타내는 네모, 남은 쿨다운을 보여 줍니다.
fn show_skill(row: &mut ChildSpawnerCommands, assets: &AssetServer, slot: &SkillSlot) {
    let cell = Node {
        width: px(113),
        height: px(46),
        padding: UiRect::all(px(5)),
        column_gap: px(6),
        align_items: AlignItems::Center,
        overflow: Overflow::clip(),
        ..default()
    };
    let text = Node {
        row_gap: px(3),
        flex_grow: 1.0,
        flex_direction: FlexDirection::Column,
        overflow: Overflow::clip(),
        ..default()
    };
    let learned = slot.rank > 0;
    let name = if learned { TEXT } else { DIM };
    row.spawn((cell, BackgroundColor(SLOT)))
        .with_children(|cell| {
            // 쿨다운이 남은 만큼 칸을 아래에서부터 어둡게 덮습니다.
            if slot.wait > 0.0 {
                cell.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        bottom: px(0),
                        width: percent(100),
                        height: percent(slot.wait * 100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                ));
            }
            ui::icon_tile(cell, assets, Some(&slot.icon), 36.0);
            cell.spawn(text).with_children(|cell| {
                let key = if slot.learnable {
                    format!("{} +", slot.key)
                } else {
                    slot.key.to_string()
                };
                let top = Node {
                    column_gap: px(6),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                };
                cell.spawn(top).with_children(|top| {
                    let color = if slot.learnable { GOOD } else { GOLD };
                    top.spawn(words(key, 13.0, color));
                    if slot.ready_secs > 0.0 && learned {
                        let secs = format!("{:.0}s", slot.ready_secs.ceil());
                        top.spawn(words(secs, 13.0, TEXT));
                    }
                });
                cell.spawn(words(slot.name.clone(), 11.0, name));
                let pips = Node {
                    column_gap: px(3),
                    ..default()
                };
                cell.spawn(pips).with_children(|pips| {
                    for step in 0..slot.max_rank {
                        let color = if step < slot.rank { GOLD } else { XP_BACK };
                        let pip = Node {
                            width: px(10),
                            height: px(4),
                            ..default()
                        };
                        pips.spawn((pip, BackgroundColor(color)));
                    }
                });
            });
        });
}

/// 항구마다 그 위에 이름표를 띄웁니다. 이름은 그 항구의 상점의 이름입니다.
pub fn show_labels(
    net: Option<NonSend<Net>>,
    camera: Single<(&Camera, &Transform), With<Camera3d>>,
    root: Single<Entity, With<LabelRoot>>,
    mut commands: Commands,
) {
    commands.entity(*root).despawn_children();
    let Some(welcome) = net.as_ref().and_then(|net| net.session.welcome()) else {
        return;
    };
    // 카메라의 이번 프레임의 자리로 계산해서, 화면을 옮길 때 이름표가 항구를 한 프레임 늦게 따라가지 않게 합니다.
    let (camera, transform) = camera.into_inner();
    let transform = &GlobalTransform::from(*transform);
    // 첫 번째 상점은 자기 진영의 본진과 항구이고, 그 뒤가 항구의 순서입니다.
    let names = welcome.shops.iter().skip(1).map(|shop| shop.name.as_str());
    commands.entity(*root).with_children(|parent| {
        for (port, name) in welcome.ports.iter().zip(names) {
            let above = Vec3::new(port.x, 1.8, -port.y);
            let Ok(at) = camera.world_to_viewport(transform, above) else {
                continue;
            };
            let label = Node {
                position_type: PositionType::Absolute,
                left: px(at.x - name.len() as f32 * 3.6),
                top: px(at.y - 16.0),
                padding: UiRect::horizontal(px(4)),
                ..default()
            };
            parent
                .spawn((label, BackgroundColor(PANEL)))
                .with_children(|label| {
                    label.spawn(words(name, 12.0, GOLD));
                });
        }
    });
}

/// 내 배의 선체가 쓰는 스킬의 번호입니다. 단축키의 순서입니다.
fn hull_skills<'a>(welcome: &'a Welcome, me: &MyShip) -> &'a [u16] {
    let hull = welcome.hulls.get(usize::from(me.hull));
    hull.map_or(&[], |hull| hull.skills.as_slice())
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
