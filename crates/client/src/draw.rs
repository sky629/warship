//! 방을 3D로 그립니다. 타일 좌표의 (x, y)는 수면 위의 점 (x, 0, -y)입니다. 타일 하나의 한 변이 길이 1이고, 북쪽(y가 큰 쪽)이 화면의 위쪽입니다.
//!
//! 게임은 수면 위의 평면에서 진행되므로, 높이는 화면에만 있습니다. 배, NPC 배, 건물, 포탄은 간단한 입체 도형이고,
//! 스냅샷에 있는 개체마다 도형 하나를 두고 프레임마다 위치를 맞춥니다(`sync_pieces`).
//! HP 막대, 사거리, 라인, 목적지는 선으로 그립니다(`draw_overlays`). 카메라는 `camera`가 움직입니다.

use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use warship_client_core::{Frame, Phase};
use warship_protocol::{WireOutcome, WireStructureKind, WireTeam};

use crate::camera::{CAMERA_OFFSET, Rig};
use crate::net::{CurrentFrame, Destination, LeavePrompt, Net};

/// 맵 밖의 바다와 하늘의 색입니다.
pub const SEA: Color = Color::srgb(0.03, 0.07, 0.1);
const WATER: Color = Color::srgb(0.07, 0.22, 0.32);
const ISLAND: Color = Color::srgb(0.42, 0.5, 0.28);
const PORT: Color = Color::srgb(0.95, 0.8, 0.3);
/// 은신한 배의 둘레에 그리는 원의 색입니다.
const CLOAKED: Color = Color::srgb(0.7, 0.5, 1.0);
/// 감속된 배의 둘레에 그리는 원의 색입니다.
const SLOWED: Color = Color::srgb(0.4, 0.9, 0.9);
const BORDER: Color = Color::srgb(0.35, 0.55, 0.65);
const LANE: Color = Color::srgb(0.2, 0.36, 0.45);
const RED: Color = Color::srgb(0.9, 0.3, 0.25);
const BLUE: Color = Color::srgb(0.3, 0.55, 1.0);
const MINE: Color = Color::srgb(1.0, 1.0, 1.0);
const SUNK: Color = Color::srgb(0.35, 0.35, 0.35);
const HP: Color = Color::srgb(0.4, 0.9, 0.4);
const HP_BACK: Color = Color::srgb(0.35, 0.1, 0.1);

/// 선으로 그리는 것을 수면에서 띄우는 높이입니다. 수면과 겹쳐서 깜빡이지 않게 합니다.
const LINE_HEIGHT: f32 = 0.05;
/// 섬의 높이입니다.
const ISLAND_HEIGHT: f32 = 0.5;

/// 타일 좌표를 수면 위의 점으로 바꿉니다.
pub fn on_water(pos: warship_rules::Vec2) -> Vec3 {
    Vec3::new(pos.x, 0.0, -pos.y)
}

/// 화면에 놓인 개체 하나입니다. 값은 스냅샷에서 그 개체를 가리키는 번호입니다. 포탄은 번호가 없어서 프레임 안의 순서를 씁니다.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Piece {
    Ship(u32),
    Creep(u32),
    Structure(u32),
    Shot(usize),
}

/// 방마다 한 번 놓는 것(수면과 섬)입니다.
#[derive(Component)]
pub struct Scenery;

/// 방에 놓은 모든 것을 고르는 조건입니다.
type InRoom = Or<(With<Piece>, With<Scenery>)>;

/// 화면에 놓을 도형 하나입니다.
struct Wanted {
    piece: Piece,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    /// 도형의 중심이 놓일 자리입니다.
    at: Vec3,
    /// 움직인 방향으로 도형을 돌리는지입니다.
    turns: bool,
    /// 도형의 크기의 배율입니다.
    scale: f32,
}

/// 지금 화면에 놓인 개체와 그 엔티티입니다.
#[derive(Resource, Default)]
pub struct Pieces(HashMap<Piece, Entity>);

/// 도형과 재질입니다. 한 번 만들어서 모든 개체가 함께 씁니다.
#[derive(Resource)]
pub struct Look {
    ship: Handle<Mesh>,
    creep: Handle<Mesh>,
    tower: Handle<Mesh>,
    harbor: Handle<Mesh>,
    base: Handle<Mesh>,
    shot: Handle<Mesh>,
    /// 반지름과 높이가 1인 원기둥입니다. 섬마다 크기를 맞춰서 씁니다.
    island: Handle<Mesh>,
    port: Handle<Mesh>,
    /// 한 변이 1인 평면입니다. 맵의 크기에 맞춰서 씁니다.
    water: Handle<Mesh>,
    red: Handle<StandardMaterial>,
    blue: Handle<StandardMaterial>,
    sunk: Handle<StandardMaterial>,
    land: Handle<StandardMaterial>,
    sea: Handle<StandardMaterial>,
    /// 어느 진영의 것도 아닌 교역항의 재질입니다.
    neutral: Handle<StandardMaterial>,
}

impl Look {
    fn team(&self, team: WireTeam) -> Handle<StandardMaterial> {
        match team {
            WireTeam::Red => self.red.clone(),
            WireTeam::Blue => self.blue.clone(),
        }
    }
}

#[derive(Component)]
pub struct StatusText;

/// 방을 나갈지 묻는 확인창의 글자입니다.
#[derive(Component)]
pub struct PromptText;

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(CAMERA_OFFSET).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // 해가 남서쪽 위에서 비춥니다.
    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            ..default()
        },
        Transform::from_xyz(-8.0, 20.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight {
        brightness: 150.0,
        ..default()
    });
    let mut matte = |color: Color| {
        materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.9,
            ..default()
        })
    };
    let look = Look {
        red: matte(RED),
        blue: matte(BLUE),
        sunk: matte(SUNK),
        land: matte(ISLAND),
        sea: matte(WATER),
        neutral: matte(PORT),
        // 배와 NPC 배는 x축 방향으로 긴 상자입니다. 뱃머리가 x축의 양의 방향입니다.
        ship: meshes.add(Cuboid::new(1.0, 0.4, 0.45)),
        creep: meshes.add(Cuboid::new(0.55, 0.25, 0.25)),
        tower: meshes.add(Cylinder::new(0.5, 1.6)),
        harbor: meshes.add(Cuboid::new(1.8, 0.6, 1.8)),
        base: meshes.add(Cuboid::new(2.4, 1.4, 2.4)),
        shot: meshes.add(Sphere::new(0.12)),
        island: meshes.add(Cylinder::new(1.0, 1.0)),
        port: meshes.add(Cuboid::new(1.4, 0.8, 1.4)),
        water: meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
    };
    commands.insert_resource(look);
    commands.init_resource::<Pieces>();
    commands.spawn((
        PromptText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: percent(40),
            left: percent(35),
            padding: UiRect::all(px(16)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
    ));
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

fn team_color(team: WireTeam) -> Color {
    match team {
        WireTeam::Red => RED,
        WireTeam::Blue => BLUE,
    }
}

/// 입장하면 수면, 섬, 교역항을 한 번 놓습니다.
pub fn build_scenery(
    net: NonSend<Net>,
    look: Res<Look>,
    scenery: Query<(), With<Scenery>>,
    mut commands: Commands,
) {
    let Some(welcome) = net.session.welcome() else {
        return;
    };
    if !scenery.is_empty() {
        return;
    }
    let (width, height) = (welcome.map_width, welcome.map_height);
    commands.spawn((
        Scenery,
        Mesh3d(look.water.clone()),
        MeshMaterial3d(look.sea.clone()),
        Transform::from_xyz(width / 2.0, 0.0, -height / 2.0)
            .with_scale(Vec3::new(width, 1.0, height)),
    ));
    for island in &welcome.islands {
        let center = Vec3::new(island.center.x, ISLAND_HEIGHT / 2.0, -island.center.y);
        commands.spawn((
            Scenery,
            Mesh3d(look.island.clone()),
            MeshMaterial3d(look.land.clone()),
            Transform::from_translation(center).with_scale(Vec3::new(
                island.radius,
                ISLAND_HEIGHT,
                island.radius,
            )),
        ));
    }
    for port in &welcome.ports {
        commands.spawn((
            Scenery,
            Mesh3d(look.port.clone()),
            MeshMaterial3d(look.neutral.clone()),
            Transform::from_xyz(port.x, 0.4, -port.y),
        ));
    }
}

/// 시작 화면으로 돌아오면 방에 놓았던 것을 모두 치우고, 카메라가 보던 점을 잊습니다.
pub fn clear_room(
    placed: Query<Entity, InRoom>,
    mut pieces: ResMut<Pieces>,
    mut rig: ResMut<Rig>,
    mut commands: Commands,
) {
    rig.0 = None;
    for entity in &placed {
        commands.entity(entity).despawn();
    }
    pieces.0.clear();
}

/// 프레임에 있는 개체마다 도형 하나를 두고 위치를 맞춥니다. 프레임에서 사라진 개체의 도형은 치웁니다.
pub fn sync_pieces(
    net: NonSend<Net>,
    frame: Res<CurrentFrame>,
    look: Res<Look>,
    mut pieces: ResMut<Pieces>,
    mut placed: Query<(&mut Transform, &mut MeshMaterial3d<StandardMaterial>)>,
    mut commands: Commands,
) {
    let Some(frame) = &frame.0 else {
        return;
    };
    let hulls = net
        .session
        .welcome()
        .map_or(&[][..], |welcome| &welcome.hulls);
    let mut wanted: Vec<Wanted> = Vec::new();
    let mut want = |piece, mesh: &Handle<Mesh>, material, at, turns| {
        let mesh = mesh.clone();
        // 배는 선체의 최대 HP가 클수록 크게 그립니다. 시작 선체의 크기가 1입니다.
        let scale = match piece {
            Piece::Ship(id) => {
                let ship = frame.ships.iter().find(|ship| ship.id == id);
                let hp = |hull: u16| hulls.get(usize::from(hull)).map(|hull| hull.max_hp as f32);
                let ratio = ship.and_then(|ship| Some(hp(ship.hull)? / hp(0)?));
                ratio.map_or(1.0, f32::sqrt)
            }
            _ => 1.0,
        };
        wanted.push(Wanted {
            piece,
            mesh,
            material,
            at,
            turns,
            scale,
        });
    };
    for site in &frame.structures {
        let (mesh, lift) = match site.kind {
            WireStructureKind::Tower => (&look.tower, 0.8),
            WireStructureKind::Harbor => (&look.harbor, 0.3),
            WireStructureKind::Base => (&look.base, 0.7),
        };
        let at = on_water(site.pos) + Vec3::Y * lift;
        want(
            Piece::Structure(site.id),
            mesh,
            look.team(site.team),
            at,
            false,
        );
    }
    for creep in &frame.creeps {
        let at = on_water(creep.pos) + Vec3::Y * 0.1;
        let material = look.team(creep.team);
        want(Piece::Creep(creep.id), &look.creep, material, at, true);
    }
    for ship in &frame.ships {
        // 가라앉은 배는 회색이고 수면 아래로 내려가 있습니다.
        let (material, lift) = if ship.sunk {
            (look.sunk.clone(), -0.1)
        } else {
            (look.team(ship.team), 0.15)
        };
        let at = on_water(ship.pos) + Vec3::Y * lift;
        want(Piece::Ship(ship.id), &look.ship, material, at, true);
    }
    for (index, shot) in frame.shots.iter().enumerate() {
        let at = on_water(shot.pos) + Vec3::Y * 0.5;
        want(
            Piece::Shot(index),
            &look.shot,
            look.team(shot.team),
            at,
            false,
        );
    }

    let mut seen = Vec::with_capacity(wanted.len());
    for Wanted {
        piece,
        mesh,
        material,
        at,
        turns,
        scale,
    } in wanted
    {
        seen.push(piece);
        let found = pieces
            .0
            .get(&piece)
            .and_then(|entity| placed.get_mut(*entity).ok());
        match found {
            Some((mut transform, mut shown)) => {
                // 움직인 방향으로 뱃머리를 돌립니다. 멈춘 배는 마지막 방향을 유지합니다.
                let moved = at - transform.translation;
                if turns && moved.xz().length_squared() > 1e-6 {
                    transform.rotation = Quat::from_rotation_y((-moved.z).atan2(moved.x));
                }
                transform.translation = at;
                transform.scale = Vec3::splat(scale);
                if shown.0 != material {
                    shown.0 = material;
                }
            }
            None => {
                let spawned = (
                    piece,
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(at).with_scale(Vec3::splat(scale)),
                );
                pieces.0.insert(piece, commands.spawn(spawned).id());
            }
        }
    }
    pieces.0.retain(|piece, entity| {
        let keep = seen.contains(piece);
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });
}

/// 수면에 누운 원을 그립니다.
fn flat_circle(gizmos: &mut Gizmos, center: Vec3, radius: f32, color: Color) {
    let flat = Quat::from_rotation_x(-FRAC_PI_2);
    let center = center + Vec3::Y * LINE_HEIGHT;
    gizmos.circle(Isometry3d::new(center, flat), radius, color);
}

/// 맵의 경계, 라인, 목적지, 사거리, HP 막대를 선으로 그립니다.
pub fn draw_overlays(
    net: NonSend<Net>,
    frame: Res<CurrentFrame>,
    destination: Res<Destination>,
    mut gizmos: Gizmos,
) {
    let (Some(welcome), Some(frame)) = (net.session.welcome(), &frame.0) else {
        return;
    };
    let lifted = |x: f32, y: f32| Vec3::new(x, LINE_HEIGHT, -y);
    let (width, height) = (welcome.map_width, welcome.map_height);
    let corners = [
        lifted(0.0, 0.0),
        lifted(width, 0.0),
        lifted(width, height),
        lifted(0.0, height),
        lifted(0.0, 0.0),
    ];
    gizmos.linestrip(corners, BORDER);
    for lane in &welcome.lanes {
        gizmos.linestrip(lane.iter().map(|point| lifted(point.x, point.y)), LANE);
    }
    if let Some(point) = destination.0 {
        let (center, arm) = (lifted(point.x, point.y), 0.3);
        gizmos.line(center - Vec3::X * arm, center + Vec3::X * arm, MINE);
        gizmos.line(center - Vec3::Z * arm, center + Vec3::Z * arm, MINE);
    }
    draw_frame(&mut gizmos, frame);
}

fn draw_frame(gizmos: &mut Gizmos, frame: &Frame) {
    for site in &frame.structures {
        let center = on_water(site.pos);
        if let Some(range) = site.range {
            let color = team_color(site.team).with_alpha(0.25);
            flat_circle(gizmos, center, range, color);
        }
        let top = match site.kind {
            WireStructureKind::Tower => 2.0,
            WireStructureKind::Harbor => 1.0,
            WireStructureKind::Base => 1.8,
        };
        draw_hp(gizmos, center + Vec3::Y * top, site.hp);
    }
    for creep in &frame.creeps {
        draw_hp(gizmos, on_water(creep.pos) + Vec3::Y * 0.5, creep.hp);
    }
    for ship in frame.ships.iter().filter(|ship| !ship.sunk) {
        let center = on_water(ship.pos);
        // 내 배는 흰 원으로 표시하고, 무기마다 사거리를 그립니다.
        if ship.mine {
            flat_circle(gizmos, center, 0.8, MINE);
            for range in &ship.ranges {
                flat_circle(gizmos, center, *range, MINE.with_alpha(0.25));
            }
        }
        if ship.slowed {
            flat_circle(gizmos, center, 0.5, SLOWED);
        }
        if ship.cloaked {
            flat_circle(gizmos, center, 0.65, CLOAKED);
        }
        draw_hp(gizmos, center + Vec3::Y * 0.8, ship.hp);
    }
}

/// `center`에 HP 막대를 그립니다. `fraction`은 최대 HP에 대한 비율입니다. 카메라가 북쪽을 보므로 막대는 동서 방향입니다.
fn draw_hp(gizmos: &mut Gizmos, center: Vec3, fraction: f32) {
    let half = Vec3::X * 0.6;
    let (left, right) = (center - half, center + half);
    // 남은 HP와 잃은 HP를 이어서 그립니다. 두 선을 겹쳐 그리면 어느 선이 보일지 정해지지 않습니다.
    let split = left + half * (2.0 * fraction.clamp(0.0, 1.0));
    gizmos.line(left, split, HP);
    gizmos.line(split, right, HP_BACK);
}

/// 방을 나갈지 묻는 동안 확인창을 보여 줍니다.
pub fn show_prompt(
    prompt: Res<LeavePrompt>,
    window: Single<(&mut Node, &mut Text), With<PromptText>>,
) {
    let (mut node, mut text) = window.into_inner();
    let (display, lines) = if prompt.0 {
        (Display::Flex, "Leave the room?\n\nEnter: leave   Esc: stay")
    } else {
        (Display::None, "")
    };
    if node.display != display {
        node.display = display;
        text.0 = lines.to_string();
    }
}

/// 방 안에서 접속 상태, 골드, 다시 나타날 때까지 남은 시간, 대전의 결과, 방에 있는 사람의 이름을 화면 왼쪽 위에 글자로 보여 줍니다.
pub fn show_status(
    net: Option<NonSend<Net>>,
    frame: Res<CurrentFrame>,
    mut text: Single<&mut Text, With<StatusText>>,
) {
    let line = net.map_or(String::new(), |net| status_line(&net, frame.0.as_ref()));
    if text.0 != line {
        text.0 = line;
    }
}

fn status_line(net: &Net, frame: Option<&Frame>) -> String {
    match (net.session.phase(), net.session.welcome()) {
        (Phase::Connecting, _) => "Connecting...".to_string(),
        (Phase::Joining, _) => "Joining...".to_string(),
        (Phase::Playing, Some(welcome)) => {
            let mut line = format!("Room {}   Team {:?}", welcome.room, welcome.team);
            if let Some(frame) = frame {
                let me = frame.me.as_ref();
                line += &format!("   Gold {}", me.map_or(0, |me| me.gold));
                line += &match (frame.outcome, me.and_then(|me| me.respawn_secs)) {
                    (Some(WireOutcome::Victory(winner)), _) if winner == welcome.team => {
                        "   VICTORY".to_string()
                    }
                    (Some(WireOutcome::Victory(_)), _) => "   DEFEAT".to_string(),
                    (Some(WireOutcome::Draw), _) => "   DRAW".to_string(),
                    (None, Some(secs)) => format!("   Sunk: back in {secs:.0}s"),
                    (None, None) => {
                        "   Right-click: sail   Arrows: view   Space: my ship   Esc: leave"
                            .to_string()
                    }
                };
            }
            // 둘째 줄은 방에 있는 사람의 이름입니다. 접속이 끊긴 사람에는 표시를 붙입니다.
            let names = net.session.roster().iter().map(|member| {
                let away = if member.connected { "" } else { " (away)" };
                format!("{}{away}", member.name)
            });
            line += &format!("\nCaptains: {}", names.collect::<Vec<_>>().join(", "));
            line
        }
        (Phase::Reconnecting, _) => "Connection lost. Reconnecting...".to_string(),
        (Phase::Playing, None) => String::new(),
        (Phase::Ended(reason), _) => format!("Disconnected: {reason}\nEsc: back to the start"),
    }
}
