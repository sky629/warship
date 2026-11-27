//! 미니맵입니다. 창의 오른쪽 아래에 맵 전체를 작게 보여 줍니다.
//!
//! 섬, 건물, 배, NPC 배를 점으로 그리고, 카메라가 보는 자리를 흰 테두리로 표시합니다. 적은 내 진영에 보이는 것만 나옵니다.
//! 왼쪽 버튼으로 누르면 카메라가 그 자리를 보고(`camera`), 오른쪽 버튼으로 누르면 배가 그 자리로 갑니다(`net::order_moves`).
//! 점은 프레임마다 지우고 다시 놓습니다. 점의 수가 적어서 그 비용은 작습니다.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use warship_client_core::{Frame, Minimap};
use warship_protocol::{Welcome, WireStructureKind, WireTeam};

use crate::camera::{Rig, map_size};
use crate::net::{CurrentFrame, Net};

const BACK: Color = Color::srgba(0.02, 0.08, 0.12, 0.85);
const EDGE: Color = Color::srgb(0.35, 0.55, 0.65);
const ISLAND: Color = Color::srgb(0.42, 0.5, 0.28);
const RED: Color = Color::srgb(0.9, 0.3, 0.25);
const BLUE: Color = Color::srgb(0.3, 0.55, 1.0);
const MINE: Color = Color::srgb(1.0, 1.0, 1.0);

/// 카메라가 보는 범위를 나타내는 테두리의 크기(타일)입니다. 카메라 거리의 배율이 1일 때의 대략적인 값입니다.
const VIEW_TILES: (f32, f32) = (40.0, 26.0);

/// 미니맵에 놓을 점 하나의 자리, 크기, 색입니다. 자리는 미니맵의 왼쪽 위에서 잰 픽셀입니다.
type Dot = ((f32, f32), (f32, f32), Color);

/// 미니맵의 바탕입니다.
#[derive(Component)]
pub struct MinimapRoot;

pub fn setup(mut commands: Commands) {
    commands.spawn((
        MinimapRoot,
        Node {
            position_type: PositionType::Absolute,
            right: px(Minimap::MARGIN),
            bottom: px(Minimap::MARGIN),
            border: UiRect::all(px(1)),
            display: Display::None,
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(BACK),
        BorderColor::all(EDGE),
    ));
}

/// 크기가 `size` 픽셀인 사각형을 `center`에 놓는 배치입니다. `center`는 미니맵의 왼쪽 위에서 잰 자리입니다.
fn dot(center: (f32, f32), size: (f32, f32)) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(center.0 - size.0 / 2.0),
        top: px(center.1 - size.1 / 2.0),
        width: px(size.0),
        height: px(size.1),
        ..default()
    }
}

fn team_color(team: WireTeam) -> Color {
    match team {
        WireTeam::Red => RED,
        WireTeam::Blue => BLUE,
    }
}

/// 방 안에서 미니맵을 그립니다. 시작 화면에서는 숨깁니다.
pub fn show_minimap(
    net: Option<NonSend<Net>>,
    frame: Res<CurrentFrame>,
    rig: Res<Rig>,
    window: Single<&Window, With<PrimaryWindow>>,
    root: Single<(Entity, &mut Node), With<MinimapRoot>>,
    mut commands: Commands,
) {
    let (root, mut node) = root.into_inner();
    commands.entity(root).despawn_children();
    let welcome = net.as_ref().and_then(|net| net.session.welcome());
    let (Some(welcome), Some(frame)) = (welcome, &frame.0) else {
        node.display = Display::None;
        return;
    };
    let minimap = Minimap::in_window((window.width(), window.height()), map_size(welcome));
    let (width, height) = minimap.size();
    node.display = Display::Flex;
    node.width = px(width);
    node.height = px(height);
    commands.entity(root).with_children(|parent| {
        for (center, size, color) in dots(&minimap, welcome, frame) {
            parent.spawn((dot(center, size), BackgroundColor(color)));
        }
        if let Some(rig) = rig.0 {
            let scale = Minimap::PX_PER_TILE * rig.zoom;
            let size = (VIEW_TILES.0 * scale, VIEW_TILES.1 * scale);
            let center = minimap.dot(rig.focus);
            let outline = Node {
                border: UiRect::all(px(1)),
                ..dot(center, size)
            };
            parent.spawn((outline, BorderColor::all(MINE)));
        }
    });
}

/// 미니맵에 놓을 점의 자리, 크기, 색입니다. 나중에 놓는 점이 위에 그려지므로 섬, 건물, NPC 배, 배의 순서입니다.
fn dots(minimap: &Minimap, welcome: &Welcome, frame: &Frame) -> Vec<Dot> {
    let square = |side: f32| (side, side);
    let mut dots = Vec::new();
    for island in &welcome.islands {
        let center = warship_rules::Vec2::new(island.center.x, island.center.y);
        let side = island.radius * 2.0 * Minimap::PX_PER_TILE * 0.85;
        dots.push((minimap.dot(center), square(side), ISLAND));
    }
    for site in &frame.structures {
        let side = match site.kind {
            WireStructureKind::Tower => 5.0,
            WireStructureKind::Harbor => 7.0,
            WireStructureKind::Base => 9.0,
        };
        dots.push((minimap.dot(site.pos), square(side), team_color(site.team)));
    }
    for creep in &frame.creeps {
        dots.push((minimap.dot(creep.pos), square(2.0), team_color(creep.team)));
    }
    for ship in frame.ships.iter().filter(|ship| !ship.sunk) {
        let color = if ship.mine {
            MINE
        } else {
            team_color(ship.team)
        };
        dots.push((minimap.dot(ship.pos), square(4.0), color));
    }
    dots
}
