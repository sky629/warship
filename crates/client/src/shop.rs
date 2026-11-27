//! 상점 창입니다. B 키로 열고 닫고, 줄을 왼쪽 클릭하면 그 줄의 명령을 보냅니다.
//!
//! 창은 화면의 왼쪽 위에 있고, 줄의 자리는 `ShopWindow`가 정합니다. 그리는 자리와 클릭을 찾는 자리가 같은 계산을 씁니다.
//! 맨 아래의 줄은 커서 아래의 줄을 설명합니다(무기의 사거리와 다시 쏘는 시간, 선체의 스킬).
//! 조건은 서버가 검사하고, 거절한 이유는 HUD에 나옵니다.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use warship_client_core::{ShopRow, ShopWindow, shop_rows};
use warship_protocol::Welcome;

use crate::net::{CurrentFrame, Net};

const BACK: Color = Color::srgba(0.0, 0.0, 0.0, 0.8);
const UNDER_CURSOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.15);
const HEADING: Color = Color::srgb(1.0, 0.85, 0.4);
const AFFORDABLE: Color = Color::WHITE;
const TOO_DEAR: Color = Color::srgb(0.55, 0.55, 0.55);

/// 상점 창이 열려 있는지입니다.
#[derive(Resource, Default)]
pub struct ShopOpen(pub bool);

/// 상점 창의 바탕입니다.
#[derive(Component)]
pub struct ShopRoot;

pub fn setup(mut commands: Commands) {
    commands.spawn((
        ShopRoot,
        Node {
            position_type: PositionType::Absolute,
            width: px(ShopWindow::WIDTH),
            display: Display::None,
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(BACK),
    ));
}

/// 상점 창이 열려 있으면 그 자리입니다. 목록의 아래에 커서 아래의 줄을 설명하는 줄이 하나 더 있습니다.
fn shop_window(welcome: &Welcome, open: &ShopOpen) -> Option<ShopWindow> {
    open.0
        .then(|| ShopWindow::with_rows(shop_rows(welcome).len() + 1))
}

/// 커서가 열려 있는 상점 창 위에 있으면, 커서 아래의 줄의 번호입니다.
pub fn row_under(window: &Window, welcome: &Welcome, open: &ShopOpen) -> Option<usize> {
    let shop = shop_window(welcome, open)?;
    let cursor = window.cursor_position()?;
    shop.row_at((cursor.x, cursor.y))
}

/// B 키로 상점 창을 열고 닫고, 왼쪽 클릭한 줄의 명령을 보냅니다.
pub fn use_shop(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut open: ResMut<ShopOpen>,
    mut net: NonSendMut<Net>,
) {
    if keys.just_pressed(KeyCode::KeyB) {
        open.0 = !open.0;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(welcome) = net.session.welcome() else {
        return;
    };
    let row = row_under(&window, welcome, &open);
    let order = row.and_then(|row| shop_rows(welcome).get(row)?.order);
    if let Some(order) = order {
        net.send_command(order);
    }
}

/// 열려 있는 상점 창을 그립니다. 커서 아래의 줄은 밝게, 골드가 모자란 줄은 흐리게 보입니다.
pub fn show_shop(
    net: Option<NonSend<Net>>,
    frame: Res<CurrentFrame>,
    open: Res<ShopOpen>,
    window: Single<&Window, With<PrimaryWindow>>,
    root: Single<(Entity, &mut Node), With<ShopRoot>>,
    mut commands: Commands,
) {
    let (root, mut node) = root.into_inner();
    commands.entity(root).despawn_children();
    let welcome = net.as_ref().and_then(|net| net.session.welcome());
    let shop = welcome.and_then(|welcome| Some((welcome, shop_window(welcome, &open)?)));
    let Some((welcome, shop)) = shop else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    node.left = px(shop.left);
    node.top = px(shop.top);
    node.height = px(shop.height());
    let me = frame.0.as_ref().and_then(|frame| frame.me.as_ref());
    let gold = me.map_or(0, |me| me.gold);
    let under_cursor = row_under(&window, welcome, &open);
    let rows = shop_rows(welcome);
    let detail = under_cursor.and_then(|row| rows.get(row));
    let detail = detail.map_or("", |row| row.detail.as_str());
    let footer = ShopRow {
        label: detail.to_string(),
        detail: String::new(),
        order: None,
        cost: None,
    };
    commands.entity(root).with_children(|parent| {
        for (index, row) in rows.iter().chain([&footer]).enumerate() {
            let color = match (row.order, row.cost) {
                (None, _) => HEADING,
                (Some(_), Some(cost)) if cost > gold => TOO_DEAR,
                (Some(_), _) => AFFORDABLE,
            };
            let lit = row.order.is_some() && under_cursor == Some(index);
            parent.spawn((
                Text::new(format!(" {}", row.label)),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(color),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(index as f32 * ShopWindow::ROW),
                    width: px(ShopWindow::WIDTH),
                    height: px(ShopWindow::ROW),
                    ..default()
                },
                BackgroundColor(if lit { UNDER_CURSOR } else { Color::NONE }),
            ));
        }
    });
}
