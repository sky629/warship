//! 미니맵입니다. 창의 오른쪽 아래에 맵 전체를 작게 보여 줍니다.
//!
//! 섬, 항구, 중립 몬스터의 자리, 건물, NPC 배, 배를 표식으로 그리고, 카메라가 보는 자리를 흰 테두리로 표시합니다. 적은 내 진영에 보이는 것만 나옵니다.
//! 왼쪽 버튼으로 누르면 카메라가 그 자리를 보고(`camera`), 오른쪽 버튼으로 누르면 배가 그 자리로 갑니다(`net::order_moves`).
//! 표식은 프레임마다 지우고 다시 놓습니다. 표식의 수가 적어서 그 비용은 작습니다.
//!
//! 표식마다 에셋 폴더의 그림(`ui/minimap/<종류>.png`)을 쓸 수 있습니다. 그림은 표식의 색(진영의 색)으로 물들이므로 흰색으로 만듭니다.
//! 그림이 없으면 색칠한 네모이고, 항구와 중립 몬스터의 자리는 테두리만 있는 네모입니다.
//! 맵 전체의 그림(`ui/minimap.png`)이 있으면 그 그림이 바탕이 되고, 섬의 표식은 그리지 않습니다.

use bevy::prelude::*;
use bevy::ui::widget::NodeImageMode;
use bevy::window::PrimaryWindow;
use warship_client_core::{Frame, Minimap};
use warship_protocol::{Welcome, WireStructureKind, WireTeam};

use crate::camera::{Rig, map_size};
use crate::net::{CurrentFrame, Net};
use crate::ui::{self, BLUE, GOLD, RED, TEXT};

const BACK: Color = Color::srgba(0.02, 0.08, 0.12, 0.85);
const EDGE: Color = Color::srgb(0.35, 0.55, 0.65);
const ISLAND: Color = Color::srgb(0.42, 0.5, 0.28);
const WILD: Color = Color::srgb(0.85, 0.85, 0.75);
/// 맵 전체의 그림입니다. 에셋 폴더에 있으면 미니맵의 바탕이 됩니다.
const MAP_IMAGE: &str = "ui/minimap.png";

/// 카메라가 보는 범위를 나타내는 테두리의 크기(타일)입니다. 카메라 거리의 배율이 1일 때의 대략적인 값입니다.
const VIEW_TILES: (f32, f32) = (40.0, 26.0);

/// 미니맵에 놓을 표식 하나입니다.
struct Mark {
    /// 미니맵의 왼쪽 위에서 잰 자리(픽셀)입니다.
    center: (f32, f32),
    /// 한 변의 길이(픽셀)입니다.
    side: f32,
    color: Color,
    /// 표식의 종류입니다. 그림의 파일의 이름입니다.
    kind: &'static str,
    /// 그림이 없을 때 속을 채우지 않고 테두리만 그리는지입니다.
    hollow: bool,
}

/// 미니맵의 바탕입니다.
#[derive(Component)]
pub struct MinimapRoot;

pub fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let mut root = commands.spawn((
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
        BorderColor::all(EDGE),
    ));
    if ui::has(MAP_IMAGE) {
        root.insert(ImageNode::new(assets.load(MAP_IMAGE)).with_mode(NodeImageMode::Stretch));
    } else {
        root.insert(BackgroundColor(BACK));
    }
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
        WireTeam::Wild => WILD,
    }
}

/// 방 안에서 미니맵을 그립니다. 시작 화면에서는 숨깁니다.
pub fn show_minimap(
    net: Option<NonSend<Net>>,
    frame: Res<CurrentFrame>,
    rig: Res<Rig>,
    assets: Res<AssetServer>,
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
        for mark in marks(&minimap, welcome, frame) {
            let place = dot(mark.center, (mark.side, mark.side));
            let image = format!("ui/minimap/{}.png", mark.kind);
            if ui::has(&image) {
                let image = ImageNode::new(assets.load(image))
                    .with_color(mark.color)
                    .with_mode(NodeImageMode::Stretch);
                parent.spawn((place, image));
            } else if mark.hollow {
                let outline = Node {
                    border: UiRect::all(px(1)),
                    ..place
                };
                parent.spawn((outline, BorderColor::all(mark.color)));
            } else {
                parent.spawn((place, BackgroundColor(mark.color)));
            }
        }
        if let Some(rig) = rig.0 {
            let scale = Minimap::PX_PER_TILE * rig.zoom;
            let size = (VIEW_TILES.0 * scale, VIEW_TILES.1 * scale);
            let center = minimap.dot(rig.focus);
            let outline = Node {
                border: UiRect::all(px(1)),
                ..dot(center, size)
            };
            parent.spawn((outline, BorderColor::all(TEXT)));
        }
    });
}

/// 미니맵에 놓을 표식입니다. 나중에 놓는 표식이 위에 그려지므로 섬, 항구, 중립 몬스터의 자리, 건물, NPC 배, 배의 순서입니다.
fn marks(minimap: &Minimap, welcome: &Welcome, frame: &Frame) -> Vec<Mark> {
    let at = |x: f32, y: f32| minimap.dot(warship_rules::Vec2::new(x, y));
    let mark = |center, side, color, kind| Mark {
        center,
        side,
        color,
        kind,
        hollow: false,
    };
    let mut marks = Vec::new();
    // 맵 전체의 그림에는 섬이 그려져 있으므로, 그 그림이 있으면 섬의 표식을 놓지 않습니다.
    for island in welcome.islands.iter().filter(|_| !ui::has(MAP_IMAGE)) {
        let side = island.radius * 2.0 * Minimap::PX_PER_TILE * 0.85;
        let center = at(island.center.x, island.center.y);
        marks.push(mark(center, side, ISLAND, "island"));
    }
    for port in &welcome.ports {
        marks.push(Mark {
            hollow: true,
            ..mark(at(port.x, port.y), 8.0, GOLD, "port")
        });
    }
    for camp in &welcome.camps {
        marks.push(Mark {
            hollow: true,
            ..mark(at(camp.x, camp.y), 6.0, WILD, "camp")
        });
    }
    for site in &frame.structures {
        let (side, kind) = match site.kind {
            WireStructureKind::Tower => (5.0, "tower"),
            WireStructureKind::Harbor => (7.0, "harbor"),
            WireStructureKind::Base => (9.0, "base"),
        };
        let center = minimap.dot(site.pos);
        marks.push(mark(center, side, team_color(site.team), kind));
    }
    for creep in &frame.creeps {
        // 중립 몬스터는 자리의 표식 안에 조금 크게 그려서, 그 자리에 몬스터가 있는지 보이게 합니다.
        let side = if creep.team == WireTeam::Wild {
            4.0
        } else {
            2.0
        };
        let center = minimap.dot(creep.pos);
        marks.push(mark(center, side, team_color(creep.team), "creep"));
    }
    for ship in frame.ships.iter().filter(|ship| !ship.sunk) {
        let (color, kind) = if ship.mine {
            (TEXT, "me")
        } else {
            (team_color(ship.team), "ship")
        };
        marks.push(mark(minimap.dot(ship.pos), 4.0, color, kind));
    }
    marks
}
