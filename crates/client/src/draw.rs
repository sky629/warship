//! 방을 위에서 내려다본 도형으로 그립니다. 타일 좌표의 (0, 0)이 화면 좌표의 원점이고, y는 위쪽입니다.

use bevy::prelude::*;
use warship_client_core::{Frame, Phase};
use warship_protocol::{WireOutcome, WireStructureKind, WireTeam};

use crate::net::{CurrentFrame, Destination, Net};

/// 타일 하나의 한 변의 길이(픽셀)입니다.
pub const TILE_PX: f32 = 14.0;

pub const SEA: Color = Color::srgb(0.05, 0.11, 0.15);
const BORDER: Color = Color::srgb(0.25, 0.4, 0.5);
const RED: Color = Color::srgb(0.95, 0.35, 0.3);
const BLUE: Color = Color::srgb(0.35, 0.6, 1.0);
const MINE: Color = Color::srgb(1.0, 1.0, 1.0);
const SUNK: Color = Color::srgb(0.4, 0.4, 0.4);
const HP: Color = Color::srgb(0.4, 0.9, 0.4);
const HP_BACK: Color = Color::srgb(0.25, 0.1, 0.1);

/// 배의 몸을 나타내는 원의 반지름(타일)입니다.
const SHIP_RADIUS: f32 = 0.45;
/// NPC 배의 몸을 나타내는 원의 반지름(타일)입니다.
const CREEP_RADIUS: f32 = 0.25;
/// 건물을 나타내는 도형의 중심에서 꼭짓점 쪽 변까지의 거리(타일)입니다.
const TOWER_HALF: f32 = 0.6;
const HARBOR_HALF: f32 = 0.9;
const BASE_HALF: f32 = 1.2;
const LANE: Color = Color::srgb(0.12, 0.2, 0.26);
const ISLAND: Color = Color::srgb(0.45, 0.55, 0.3);

/// 중심에서 본 정사각형과 마름모의 꼭짓점입니다. 처음 점으로 돌아와서 닫습니다.
const SQUARE: [Vec2; 5] = [
    Vec2::new(-1.0, -1.0),
    Vec2::new(1.0, -1.0),
    Vec2::new(1.0, 1.0),
    Vec2::new(-1.0, 1.0),
    Vec2::new(-1.0, -1.0),
];
const DIAMOND: [Vec2; 5] = [
    Vec2::new(0.0, -1.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(-1.0, 0.0),
    Vec2::new(0.0, -1.0),
];

#[derive(Component)]
pub struct StatusText;

pub fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        StatusText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(10),
            left: px(10),
            ..default()
        },
    ));
}

/// 입장하면 카메라를 맵의 한가운데에 놓습니다.
pub fn center_camera(net: NonSend<Net>, mut camera: Single<&mut Transform, With<Camera2d>>) {
    if let Some(welcome) = net.session.welcome() {
        let center = Vec2::new(welcome.map_width, welcome.map_height) * (TILE_PX / 2.0);
        camera.translation = center.extend(camera.translation.z);
    }
}

fn team_color(team: WireTeam) -> Color {
    match team {
        WireTeam::Red => RED,
        WireTeam::Blue => BLUE,
    }
}

fn px_pos(pos: warship_rules::Vec2) -> Vec2 {
    Vec2::new(pos.x, pos.y) * TILE_PX
}

pub fn draw_room(
    net: NonSend<Net>,
    frame: Res<CurrentFrame>,
    destination: Res<Destination>,
    mut gizmos: Gizmos,
) {
    let (Some(welcome), Some(frame)) = (net.session.welcome(), &frame.0) else {
        return;
    };
    let (width, height) = (welcome.map_width * TILE_PX, welcome.map_height * TILE_PX);
    let corners = [
        Vec2::ZERO,
        Vec2::new(width, 0.0),
        Vec2::new(width, height),
        Vec2::new(0.0, height),
        Vec2::ZERO,
    ];
    gizmos.linestrip_2d(corners, BORDER);
    for lane in &welcome.lanes {
        let points = lane
            .iter()
            .map(|point| Vec2::new(point.x, point.y) * TILE_PX);
        gizmos.linestrip_2d(points, LANE);
    }
    // 섬은 가장자리의 원과 그 안쪽의 흐린 원으로 그립니다.
    for island in &welcome.islands {
        let center = Vec2::new(island.center.x, island.center.y) * TILE_PX;
        let radius = island.radius * TILE_PX;
        gizmos.circle_2d(center, radius, ISLAND);
        gizmos.circle_2d(center, radius * 0.6, ISLAND.with_alpha(0.4));
    }
    if let Some(point) = destination.0 {
        let (center, arm) = (point * TILE_PX, TILE_PX * 0.3);
        gizmos.line_2d(center - Vec2::X * arm, center + Vec2::X * arm, MINE);
        gizmos.line_2d(center - Vec2::Y * arm, center + Vec2::Y * arm, MINE);
    }
    draw_frame(&mut gizmos, frame);
}

fn draw_frame(gizmos: &mut Gizmos, frame: &Frame) {
    for site in &frame.structures {
        let center = px_pos(site.pos);
        let color = team_color(site.team);
        // 타워는 작은 정사각형, 항구는 마름모, 본진은 큰 정사각형입니다.
        let (half, corners) = match site.kind {
            WireStructureKind::Tower => (TOWER_HALF, SQUARE),
            WireStructureKind::Harbor => (HARBOR_HALF, DIAMOND),
            WireStructureKind::Base => (BASE_HALF, SQUARE),
        };
        let half = half * TILE_PX;
        gizmos.linestrip_2d(corners.map(|corner| center + corner * half), color);
        if let Some(range) = site.range {
            gizmos.circle_2d(center, range * TILE_PX, color.with_alpha(0.15));
        }
        draw_hp(gizmos, center + Vec2::Y * (half + 6.0), site.hp);
    }
    for creep in &frame.creeps {
        let (center, radius) = (px_pos(creep.pos), CREEP_RADIUS * TILE_PX);
        gizmos.circle_2d(center, radius, team_color(creep.team).with_alpha(0.7));
        draw_hp(gizmos, center + Vec2::Y * (radius + 4.0), creep.hp);
    }
    for ship in &frame.ships {
        let (center, radius) = (px_pos(ship.pos), SHIP_RADIUS * TILE_PX);
        if ship.sunk {
            gizmos.line_2d(
                center - Vec2::splat(radius),
                center + Vec2::splat(radius),
                SUNK,
            );
            let slant = Vec2::new(radius, -radius);
            gizmos.line_2d(center - slant, center + slant, SUNK);
            continue;
        }
        gizmos.circle_2d(center, radius, team_color(ship.team));
        if ship.mine {
            gizmos.circle_2d(center, radius + 3.0, MINE);
            for range in &ship.ranges {
                gizmos.circle_2d(center, range * TILE_PX, MINE.with_alpha(0.2));
            }
        }
        draw_hp(gizmos, center + Vec2::Y * (radius + 6.0), ship.hp);
    }
    for shot in &frame.shots {
        gizmos.circle_2d(px_pos(shot.pos), 2.0, team_color(shot.team));
    }
}

/// `center` 위에 HP 막대를 그립니다. `fraction`은 최대 HP에 대한 비율입니다.
fn draw_hp(gizmos: &mut Gizmos, center: Vec2, fraction: f32) {
    let half = Vec2::X * (TILE_PX * 0.6);
    let left = center - half;
    gizmos.line_2d(left, center + half, HP_BACK);
    gizmos.line_2d(left, left + half * (2.0 * fraction.clamp(0.0, 1.0)), HP);
}

/// 접속 상태, 골드, 다시 나타날 때까지 남은 시간, 대전의 결과를 화면 왼쪽 위에 글자로 보여 줍니다.
pub fn show_status(
    net: NonSend<Net>,
    frame: Res<CurrentFrame>,
    mut text: Single<&mut Text, With<StatusText>>,
) {
    let line = match (net.session.phase(), net.session.welcome()) {
        (Phase::Connecting, _) => "Connecting...".to_string(),
        (Phase::Joining, _) => "Joining...".to_string(),
        (Phase::Playing, Some(welcome)) => {
            let mut line = format!("Room {}   Team {:?}", welcome.room, welcome.team);
            if let Some(frame) = &frame.0 {
                let me = frame.me.as_ref();
                line += &format!("   Gold {}", me.map_or(0, |me| me.gold));
                line += &match (frame.outcome, me.and_then(|me| me.respawn_secs)) {
                    (Some(WireOutcome::Victory(winner)), _) if winner == welcome.team => {
                        "   VICTORY".to_string()
                    }
                    (Some(WireOutcome::Victory(_)), _) => "   DEFEAT".to_string(),
                    (Some(WireOutcome::Draw), _) => "   DRAW".to_string(),
                    (None, Some(secs)) => format!("   Sunk: back in {secs:.0}s"),
                    (None, None) => "   Right-click: sail".to_string(),
                };
            }
            line
        }
        (Phase::Playing, None) => String::new(),
        (Phase::Ended(reason), _) => format!("Disconnected: {reason}"),
    };
    if text.0 != line {
        text.0 = line;
    }
}
