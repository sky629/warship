//! 카메라입니다. 수면을 비스듬히 내려다보고, 처음에는 내 배를 따라갑니다.
//!
//! 방향키를 누르거나 커서를 창의 가장자리에 대면 화면이 그쪽으로 옮겨지고, 내 배를 따라가기를 멈춥니다.
//! Space를 누르면 다시 내 배를 따라갑니다. 마우스 휠은 거리를 바꾸고, 미니맵을 왼쪽 버튼으로 누르면 그 자리를 봅니다.
//! 보는 점의 계산은 `warship_client_core::CameraRig`가 합니다.

use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use warship_client_core::{CameraRig, Minimap};
use warship_protocol::Welcome;

use crate::draw::on_water;
use crate::net::{CurrentFrame, Net};

/// 카메라가 보는 점에서 카메라까지의 방향과 거리입니다. 수면을 약 55도로 내려다봅니다.
pub const CAMERA_OFFSET: Vec3 = Vec3::new(0.0, 30.0, 21.0);
/// 커서가 창의 가장자리에서 이 거리(픽셀) 안에 있으면 화면을 그쪽으로 옮깁니다.
const EDGE_PX: f32 = 12.0;

/// 방 안에서 카메라가 보는 점입니다. 입장 안내를 받으면 만들고, 방을 나오면 지웁니다.
#[derive(Resource, Default)]
pub struct Rig(pub Option<CameraRig>);

/// 맵의 크기(타일)입니다.
pub fn map_size(welcome: &Welcome) -> warship_rules::Vec2 {
    warship_rules::Vec2::new(welcome.map_width, welcome.map_height)
}

/// 이 창의 미니맵과, 커서가 그 위에 있으면 커서가 가리키는 타일 좌표입니다.
pub fn minimap_under(window: &Window, welcome: &Welcome) -> (Minimap, Option<warship_rules::Vec2>) {
    let minimap = Minimap::in_window((window.width(), window.height()), map_size(welcome));
    let cursor = window.cursor_position();
    let tile = cursor.and_then(|cursor| minimap.tile_at((cursor.x, cursor.y)));
    (minimap, tile)
}

/// 방향키와 창의 가장자리에 있는 커서가 가리키는 방향입니다. x는 동쪽, y는 북쪽입니다. 미니맵 위의 커서는 가장자리로 세지 않습니다.
fn pan_direction(
    keys: &ButtonInput<KeyCode>,
    window: &Window,
    over_minimap: bool,
) -> warship_rules::Vec2 {
    let cursor = window.cursor_position().filter(|_| !over_minimap);
    let (width, height) = (window.width(), window.height());
    let toward = |key: KeyCode, at_edge: fn(Vec2, f32, f32) -> bool| {
        let pushed = cursor.is_some_and(|cursor| at_edge(cursor, width, height));
        f32::from(u8::from(keys.pressed(key) || pushed))
    };
    let east = toward(KeyCode::ArrowRight, |cursor, width, _| {
        cursor.x >= width - EDGE_PX
    });
    let west = toward(KeyCode::ArrowLeft, |cursor, _, _| cursor.x <= EDGE_PX);
    let north = toward(KeyCode::ArrowUp, |cursor, _, _| cursor.y <= EDGE_PX);
    let south = toward(KeyCode::ArrowDown, |cursor, _, height| {
        cursor.y >= height - EDGE_PX
    });
    warship_rules::Vec2::new(east - west, north - south)
}

/// 조작에 따라 보는 점을 옮기고, 카메라를 그 점을 내려다보는 자리에 놓습니다.
#[allow(clippy::too_many_arguments)]
pub fn steer_camera(
    net: NonSend<Net>,
    frame: Res<CurrentFrame>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time<Real>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut rig: ResMut<Rig>,
    mut camera: Single<&mut Transform, With<Camera3d>>,
) {
    let Some(welcome) = net.session.welcome() else {
        return;
    };
    let map = map_size(welcome);
    let rig = rig.0.get_or_insert_with(|| CameraRig::new(map * 0.5));
    let (_, on_minimap) = minimap_under(&window, welcome);

    // 브라우저와 트랙패드는 휠을 픽셀로 알려 줍니다. 칸으로 알려 주는 마우스와 같은 양이 되게 바꿉니다.
    match scroll.unit {
        MouseScrollUnit::Line => rig.zoom_by(scroll.delta.y),
        MouseScrollUnit::Pixel => rig.zoom_by_pixels(scroll.delta.y),
    }
    let direction = pan_direction(&keys, &window, on_minimap.is_some());
    rig.pan(direction, time.delta_secs(), map);
    // 한 프레임 안에 눌렀다 뗀 버튼은 `pressed`에 나타나지 않으므로 `just_pressed`도 봅니다.
    let held = mouse.pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Left);
    if let (true, Some(tile)) = (held, on_minimap) {
        rig.look_at(tile, map);
    }
    if keys.pressed(KeyCode::Space) {
        rig.following = true;
    }
    let ships = frame.0.iter().flat_map(|frame| &frame.ships);
    let mut mine = ships.filter(|ship| ship.mine).map(|ship| ship.pos);
    rig.track(mine.next());

    let target = on_water(rig.focus);
    **camera =
        Transform::from_translation(target + CAMERA_OFFSET * rig.zoom).looking_at(target, Vec3::Y);
}
