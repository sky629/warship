//! 상점 창입니다. B 키로 열고 닫고, 탭을 눌러서 상점을 고르고, 줄을 왼쪽 클릭하면 그 줄의 명령을 보냅니다.
//!
//! 창은 화면의 왼쪽 위에 있습니다. 위에서부터 탭 줄, 열린 탭의 줄(아이콘, 이름, 짧은 설명, 값), 커서 아래의 줄의 설명입니다.
//! 창을 열면 내 배가 있는 상점의 탭이 열립니다. 내 배가 있는 상점의 탭은 초록색이고, 다른 상점의 줄은 흐립니다.
//! 자리는 `ShopWindow`가 정합니다. 그리는 자리와 클릭을 찾는 자리가 같은 계산을 씁니다.
//! 조건은 서버가 검사하고, 거절한 이유는 칸 줄의 위에 나옵니다.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use warship_client_core::{ShopTab, ShopWindow, first_tab, ko, shop_tabs, shops_near};
use warship_protocol::Welcome;

use crate::net::{CurrentFrame, Net};
use crate::ui::{self, BAD, DIM, GOLD, GOOD, LIT, Panel, SLOT, TEXT, words};

/// 상점 창의 상태입니다.
#[derive(Resource, Default)]
pub struct ShopOpen {
    pub open: bool,
    /// 열려 있는 탭의 번호입니다.
    pub tab: usize,
}

/// 상점 창의 바탕입니다.
#[derive(Component)]
pub struct ShopRoot;

pub fn setup(mut commands: Commands) {
    commands.spawn((
        ShopRoot,
        Panel,
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(ui::PANEL),
    ));
}

/// 상점 창이 열려 있으면 그 탭들과 이 창에서의 자리입니다.
fn open_shop(
    window: &Window,
    welcome: &Welcome,
    open: &ShopOpen,
) -> Option<(Vec<ShopTab>, ShopWindow)> {
    open.open.then(|| {
        let tabs = shop_tabs(welcome);
        let tab = tabs.get(open.tab);
        let rows = tab.map_or(0, |tab| tab.rows.len());
        let lines = tab.map_or(0, ShopTab::detail_lines);
        let shop = ShopWindow::in_window(window.width(), tabs.len(), rows, lines);
        (tabs, shop)
    })
}

/// 커서가 열려 있는 상점 창 위에 있는지입니다.
pub fn under_cursor(window: &Window, welcome: &Welcome, open: &ShopOpen) -> bool {
    let shop = open_shop(window, welcome, open).map(|(_, shop)| shop);
    let cursor = window.cursor_position();
    shop.zip(cursor)
        .is_some_and(|(shop, cursor)| shop.covers((cursor.x, cursor.y)))
}

/// B 키로 상점 창을 열고 닫고, 왼쪽 클릭한 탭을 열고, 왼쪽 클릭한 줄의 명령을 보냅니다.
pub fn use_shop(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    frame: Res<CurrentFrame>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut open: ResMut<ShopOpen>,
    mut net: NonSendMut<Net>,
) {
    let Some(welcome) = net.session.welcome().cloned() else {
        return;
    };
    if keys.just_pressed(KeyCode::KeyB) {
        open.open = !open.open;
        // 창을 열 때는 내 배가 있는 상점의 탭을 엽니다.
        let near = frame.0.as_ref().map(|frame| shops_near(&welcome, frame));
        open.tab = first_tab(&shop_tabs(&welcome), &near.unwrap_or_default());
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let shop = open_shop(&window, &welcome, &open);
    let (Some((tabs, shop)), Some(cursor)) = (shop, window.cursor_position()) else {
        return;
    };
    let cursor = (cursor.x, cursor.y);
    if let Some(tab) = shop.tab_at(cursor) {
        open.tab = tab;
    }
    let row = shop.row_at(cursor);
    let row = row.and_then(|row| tabs.get(open.tab)?.rows.get(row));
    if let Some(row) = row {
        net.send_command(row.order);
    }
}

/// 열려 있는 상점 창을 그립니다. 커서 아래의 줄은 밝게, 골드가 모자란 줄의 값은 붉게, 내 배가 없는 상점의 줄은 흐리게 보입니다.
pub fn show_shop(
    net: Option<NonSend<Net>>,
    frame: Res<CurrentFrame>,
    open: Res<ShopOpen>,
    assets: Res<AssetServer>,
    window: Single<&Window, With<PrimaryWindow>>,
    root: Single<(Entity, &mut Node), With<ShopRoot>>,
    mut commands: Commands,
) {
    let (root, mut node) = root.into_inner();
    commands.entity(root).despawn_children();
    let welcome = net.as_ref().and_then(|net| net.session.welcome());
    let shown = welcome.and_then(|welcome| Some((welcome, open_shop(&window, welcome, &open)?)));
    let Some((welcome, (tabs, shop))) = shown else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    node.left = px(shop.left);
    node.top = px(shop.top);
    node.width = px(shop.width());
    node.height = px(shop.height());
    let gold = frame.0.as_ref().and_then(|frame| frame.me.as_ref());
    let gold = gold.map_or(0, |me| me.gold);
    let near = frame.0.as_ref().map(|frame| shops_near(welcome, frame));
    let near = near.unwrap_or_default();
    let cursor = window.cursor_position().map(|cursor| (cursor.x, cursor.y));
    let lit = cursor.and_then(|cursor| shop.row_at(cursor));
    let tab = tabs.get(open.tab);
    let here = tab.is_some_and(|tab| near.contains(&tab.shop));
    // 설명 줄은 커서 아래의 줄의 설명이고, 내 배가 이 상점에 없으면 어디로 가야 하는지입니다.
    let detail = lit.and_then(|row| tab?.rows.get(row));
    let footer = match (here, tab, detail) {
        (false, Some(tab), _) => {
            let name = welcome
                .shops
                .get(tab.shop)
                .map_or("?", |shop| ko(&shop.name));
            format!("{name}에 가면 이 탭의 것을 살 수 있습니다")
        }
        (_, _, Some(row)) => row.detail.clone(),
        _ => String::new(),
    };
    let cell = |at: (f32, f32), size: (f32, f32)| Node {
        position_type: PositionType::Absolute,
        left: px(at.0),
        top: px(at.1),
        width: px(size.0),
        height: px(size.1),
        padding: UiRect::horizontal(px(6)),
        column_gap: px(8),
        align_items: AlignItems::Center,
        overflow: Overflow::clip(),
        ..default()
    };
    commands.entity(root).with_children(|parent| {
        for (index, tab) in tabs.iter().enumerate() {
            let at = shop.tab_place(index);
            let back = if index == open.tab { LIT } else { SLOT };
            let color = if near.contains(&tab.shop) { GOOD } else { TEXT };
            let size = (shop.tab_width() - 2.0, ShopWindow::TAB - 2.0);
            parent
                .spawn((cell(at, size), BackgroundColor(back)))
                .with_children(|tab_cell| {
                    tab_cell.spawn(words(tab.title.clone(), 12.0, color));
                });
        }
        for (index, row) in tab.map_or(&[][..], |tab| &tab.rows).iter().enumerate() {
            let back = if lit == Some(index) { LIT } else { Color::NONE };
            let size = (ShopWindow::COLUMN, ShopWindow::ROW);
            let name = if here { TEXT } else { DIM };
            let dear = row.cost.is_some_and(|cost| cost > gold);
            let price = if dear { BAD } else { GOLD };
            parent
                .spawn((cell(shop.place(index), size), BackgroundColor(back)))
                .with_children(|line| {
                    ui::icon_tile(line, &assets, Some(&row.icon), 32.0);
                    let text = Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        overflow: Overflow::clip(),
                        ..default()
                    };
                    line.spawn(text).with_children(|text| {
                        text.spawn(words(row.name.clone(), 14.0, name));
                        text.spawn(words(row.note.clone(), 11.0, DIM));
                    });
                    line.spawn(words(format!("{}g", row.price), 14.0, price));
                });
        }
        let size = (shop.width() - ShopWindow::FRAME * 2.0, shop.footer_height());
        parent
            .spawn(cell(shop.footer(), size))
            .with_children(|line| {
                line.spawn(words(footer, 13.0, GOLD));
            });
    });
}
