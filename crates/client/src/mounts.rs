//! 화면의 왼쪽 아래에 있는 칸 줄입니다. 내 배의 칸마다 아이템의 이름과 안내를 보여 줍니다.
//!
//! 소모품이 달린 칸을 왼쪽 클릭하면 그 소모품을 쓰고, 아이템이 달린 칸을 오른쪽 클릭하면 그 아이템을 팝니다.
//! 칸의 자리는 `MountBar`가 정합니다. 그리는 자리와 클릭을 찾는 자리가 같은 계산을 씁니다.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use warship_client_core::{Frame, MountBar, mount_views};
use warship_protocol::{WireCommand, WireItem};

use crate::net::{CurrentFrame, Net};

const BACK: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const UNDER_CURSOR: Color = Color::srgba(0.25, 0.25, 0.25, 0.85);
const NAME: Color = Color::WHITE;
const NOTE: Color = Color::srgb(1.0, 0.85, 0.4);

/// 칸 줄의 바탕입니다.
#[derive(Component)]
pub struct MountRoot;

pub fn setup(mut commands: Commands) {
    commands.spawn((
        MountRoot,
        Node {
            position_type: PositionType::Absolute,
            height: px(MountBar::SLOT.1),
            display: Display::None,
            ..default()
        },
    ));
}

/// 내 배가 있으면, 이 창에서의 칸 줄의 자리입니다.
fn bar(window: &Window, frame: Option<&Frame>) -> Option<MountBar> {
    let me = frame?.me.as_ref()?;
    Some(MountBar::in_window(window.height(), me.mounts.len()))
}

/// 커서 아래에 있는 칸의 번호입니다.
pub fn slot_under(window: &Window, frame: Option<&Frame>) -> Option<usize> {
    let cursor = window.cursor_position()?;
    bar(window, frame)?.slot_at((cursor.x, cursor.y))
}

/// 왼쪽 클릭한 칸의 소모품을 쓰고, 오른쪽 클릭한 칸의 아이템을 팝니다.
pub fn use_mounts(
    mouse: Res<ButtonInput<MouseButton>>,
    frame: Res<CurrentFrame>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut net: NonSendMut<Net>,
) {
    let (uses, sells) = (
        mouse.just_pressed(MouseButton::Left),
        mouse.just_pressed(MouseButton::Right),
    );
    let Some(slot) = slot_under(&window, frame.0.as_ref()).filter(|_| uses || sells) else {
        return;
    };
    let me = frame.0.as_ref().and_then(|frame| frame.me.as_ref());
    let item = me.and_then(|me| me.mounts.get(slot).copied().flatten());
    let Some(item) = item else { return };
    let slot = slot as u8;
    if sells {
        net.send_command(WireCommand::Sell { slot });
    } else if matches!(item, WireItem::Supply(_)) {
        net.send_command(WireCommand::Use { slot });
    }
}

/// 내 배의 칸을 그립니다. 커서 아래의 칸은 밝게 보입니다.
pub fn show_mounts(
    net: Option<NonSend<Net>>,
    frame: Res<CurrentFrame>,
    window: Single<&Window, With<PrimaryWindow>>,
    root: Single<(Entity, &mut Node), With<MountRoot>>,
    mut commands: Commands,
) {
    let (root, mut node) = root.into_inner();
    commands.entity(root).despawn_children();
    let welcome = net.as_ref().and_then(|net| net.session.welcome());
    let me = frame.0.as_ref().and_then(|frame| frame.me.as_ref());
    let shown = bar(&window, frame.0.as_ref()).zip(welcome).zip(me);
    let Some(((bar, welcome), me)) = shown else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    node.left = px(bar.left);
    node.top = px(bar.top);
    node.width = px(bar.width());
    let lit = slot_under(&window, frame.0.as_ref());
    let line = |text: String, color: Color| {
        (
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(color),
        )
    };
    commands.entity(root).with_children(|parent| {
        for (slot, view) in mount_views(welcome, me).into_iter().enumerate() {
            let back = if lit == Some(slot) {
                UNDER_CURSOR
            } else {
                BACK
            };
            let cell = Node {
                position_type: PositionType::Absolute,
                left: px(bar.place(slot)),
                top: px(0),
                width: px(MountBar::SLOT.0),
                height: px(MountBar::SLOT.1),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(4), px(2)),
                overflow: Overflow::clip(),
                ..default()
            };
            parent
                .spawn((cell, BackgroundColor(back)))
                .with_children(|cell| {
                    cell.spawn(line(view.name, NAME));
                    cell.spawn(line(format!("{}  {}", slot + 1, view.note), NOTE));
                });
        }
    });
}
