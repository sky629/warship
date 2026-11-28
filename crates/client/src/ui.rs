//! 화면의 모든 UI가 함께 쓰는 것입니다: 색과 크기(테마), 글자, 아이콘 칸, 패널의 바탕, 에셋의 목록.
//!
//! 에셋은 `assets` 폴더에 정해진 이름으로 넣습니다(그 폴더의 README에 이름이 있습니다). 빌드할 때 그 폴더의 파일 목록이 `FILES`에 들어가고,
//! 화면은 목록에 있는 파일만 불러옵니다. 아이콘의 파일이 없으면 종류의 색의 네모에 이름의 머리글자를 적습니다.
//! 패널의 그림이 없으면 패널은 한 가지 색이고, 글꼴의 파일이 없으면 내장 글꼴입니다.

use bevy::prelude::*;
use bevy::ui::widget::NodeImageMode;
use warship_client_core::{Icon, IconKind};

include!(concat!(env!("OUT_DIR"), "/asset_files.rs"));

/// 모든 패널의 바탕 그림입니다.
const PANEL_IMAGE: &str = "ui/panel.png";
/// 패널의 바탕 그림에서 늘이지 않는 가장자리의 두께(픽셀)입니다.
const PANEL_BORDER: f32 = 12.0;
/// 모든 글자의 글꼴입니다.
const FONT: &str = "fonts/ui.ttf";

pub const PANEL: Color = Color::srgba(0.03, 0.06, 0.09, 0.88);
/// 패널 안의 칸(스킬 칸, 칸 줄의 칸, 탭)의 바탕입니다.
pub const SLOT: Color = Color::srgba(0.16, 0.2, 0.25, 0.95);
/// 커서 아래에 있는 줄이나 칸, 열린 탭의 바탕입니다.
pub const LIT: Color = Color::srgba(0.3, 0.36, 0.44, 0.95);
pub const TEXT: Color = Color::WHITE;
pub const DIM: Color = Color::srgb(0.6, 0.65, 0.7);
pub const GOLD: Color = Color::srgb(1.0, 0.82, 0.3);
pub const GOOD: Color = Color::srgb(0.45, 0.9, 0.5);
pub const BAD: Color = Color::srgb(1.0, 0.45, 0.4);
pub const RED: Color = Color::srgb(0.95, 0.4, 0.35);
pub const BLUE: Color = Color::srgb(0.45, 0.65, 1.0);
pub const HP: Color = Color::srgb(0.25, 0.7, 0.3);
pub const HP_BACK: Color = Color::srgb(0.35, 0.1, 0.1);
pub const XP: Color = Color::srgb(0.5, 0.45, 0.95);
pub const XP_BACK: Color = Color::srgb(0.12, 0.12, 0.2);

/// 패널의 바탕을 가진 UI입니다. 바탕 그림이 있으면 그 그림을 입힙니다.
#[derive(Component)]
pub struct Panel;

/// 에셋 폴더에 이 파일이 있는지입니다.
pub fn has(path: &str) -> bool {
    FILES.contains(&path)
}

/// 한 줄의 글자입니다. 자리가 좁아도 다음 줄로 넘기지 않습니다.
pub fn words(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        TextLayout::default().with_no_wrap(),
    )
}

/// 아이콘이 없을 때 그리는 네모의 색입니다. 종류마다 다릅니다.
fn tile_color(kind: IconKind) -> Color {
    match kind {
        IconKind::Weapon => Color::srgb(0.55, 0.25, 0.2),
        IconKind::Gear => Color::srgb(0.2, 0.4, 0.5),
        IconKind::Supply => Color::srgb(0.25, 0.5, 0.3),
        IconKind::Hull => Color::srgb(0.35, 0.3, 0.55),
        IconKind::Skill => Color::srgb(0.55, 0.45, 0.15),
        IconKind::Trade => Color::srgb(0.5, 0.35, 0.2),
        IconKind::Hire => Color::srgb(0.4, 0.4, 0.45),
    }
}

/// 한 변이 `size`인 아이콘 칸을 그립니다. 에셋이 있으면 그 그림이고, 없으면 종류의 색의 네모에 머리글자입니다.
/// 아이콘이 없는 칸(빈 칸)은 어두운 네모입니다.
pub fn icon_tile(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    icon: Option<&Icon>,
    size: f32,
) {
    let tile = Node {
        width: px(size),
        height: px(size),
        flex_shrink: 0.0,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        overflow: Overflow::clip(),
        ..default()
    };
    let Some(icon) = icon else {
        parent.spawn((tile, BackgroundColor(XP_BACK)));
        return;
    };
    let path = icon.path();
    if has(&path) {
        let image = ImageNode::new(assets.load(path)).with_mode(NodeImageMode::Stretch);
        parent.spawn((tile, image));
    } else {
        let initials = words(icon.initials(), size * 0.42, TEXT);
        parent.spawn((
            tile,
            BackgroundColor(tile_color(icon.kind)),
            children![initials],
        ));
    }
}

/// 패널의 바탕 그림을 그리는 UI입니다. 패널의 첫째 자식이고, 패널의 안쪽 여백까지 덮습니다.
#[derive(Component)]
pub struct Backdrop;

/// 패널의 바탕 그림이 에셋 폴더에 있으면, 바탕 그림이 없는 패널마다 바탕 그림을 첫째 자식으로 넣습니다.
/// 패널이 자식을 지우고 다시 만들면 바탕 그림도 다시 넣습니다. 가장자리는 늘이지 않습니다.
pub fn back_panels(
    assets: Res<AssetServer>,
    panels: Query<(Entity, Option<&Children>), With<Panel>>,
    backdrops: Query<(), With<Backdrop>>,
    mut commands: Commands,
) {
    if !has(PANEL_IMAGE) {
        return;
    }
    let slicer = TextureSlicer {
        border: BorderRect::all(PANEL_BORDER),
        ..default()
    };
    for (panel, children) in &panels {
        let backed = |children: &Children| children.iter().any(|child| backdrops.contains(child));
        if children.is_some_and(backed) {
            continue;
        }
        let image = ImageNode::new(assets.load(PANEL_IMAGE))
            .with_mode(NodeImageMode::Sliced(slicer.clone()));
        let place = Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        };
        let backdrop = commands.spawn((Backdrop, place, image)).id();
        commands
            .entity(panel)
            .remove::<BackgroundColor>()
            .insert_children(0, &[backdrop]);
    }
}

/// 글꼴의 파일이 에셋 폴더에 있으면, 새로 생긴 글자에 그 글꼴을 입힙니다.
pub fn dress_text(assets: Res<AssetServer>, mut texts: Query<&mut TextFont, Added<TextFont>>) {
    if !has(FONT) {
        return;
    }
    for mut font in &mut texts {
        font.font = assets.load(FONT).into();
    }
}
