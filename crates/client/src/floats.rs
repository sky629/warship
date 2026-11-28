//! 떠오르는 숫자입니다. 내 배가 준 피해, 받은 피해, 받은 골드를 일어난 자리 위에 띄웁니다.
//!
//! 숫자는 나온 뒤 위로 올라가면서 옅어집니다. 프레임마다 다시 만듭니다.

use bevy::prelude::*;
use bevy::ui::widget::TextShadow;
use warship_client_core::{FloatKind, FloatView};

use crate::net::CurrentFrame;
use crate::ui::{BAD, GOLD, TEXT, words};

/// 숫자가 사라질 때까지 올라가는 높이(픽셀)입니다.
const RISE: f32 = 44.0;
/// 같은 자리에서 나온 숫자를 옆으로 벌리는 간격(픽셀)입니다.
const SPREAD: f32 = 12.0;

/// 떠오르는 숫자들의 바탕입니다. 창 전체를 덮고 보이지 않습니다.
#[derive(Component)]
pub struct FloatRoot;

pub fn setup(mut commands: Commands) {
    commands.spawn((
        FloatRoot,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
    ));
}

/// 숫자의 글자, 크기, 색입니다.
fn look(float: &FloatView) -> (String, f32, Color) {
    match float.kind {
        FloatKind::Dealt => (float.amount.to_string(), 15.0, TEXT),
        FloatKind::Taken => (format!("-{}", float.amount), 15.0, BAD),
        FloatKind::Gold => (format!("+{}g", float.amount), 19.0, GOLD),
    }
}

pub fn show_floats(
    frame: Res<CurrentFrame>,
    camera: Single<(&Camera, &Transform), With<Camera3d>>,
    root: Single<Entity, With<FloatRoot>>,
    mut commands: Commands,
) {
    commands.entity(*root).despawn_children();
    let Some(frame) = frame.0.as_ref() else {
        return;
    };
    // 카메라의 이번 프레임의 자리로 계산해서, 화면을 옮길 때 숫자가 한 프레임 늦게 따라가지 않게 합니다.
    let (camera, transform) = camera.into_inner();
    let transform = &GlobalTransform::from(*transform);
    commands.entity(*root).with_children(|parent| {
        for float in &frame.floats {
            let above = Vec3::new(float.pos.x, 1.2, -float.pos.y);
            let Ok(at) = camera.world_to_viewport(transform, above) else {
                continue;
            };
            let (text, size, color) = look(float);
            // 번호에 따라 다섯 자리 가운데 하나에 놓아서, 잇따라 나온 숫자가 겹치지 않게 합니다.
            let lane = (float.id % 5) as f32 - 2.0;
            let place = Node {
                position_type: PositionType::Absolute,
                left: px(at.x + lane * SPREAD - text.len() as f32 * size * 0.3),
                top: px(at.y - size - RISE * float.age),
                ..default()
            };
            // 절반이 지난 뒤부터 옅어집니다.
            let fade = (2.0 - 2.0 * float.age).min(1.0);
            let shadow = TextShadow {
                offset: Vec2::splat(1.0),
                color: Color::BLACK.with_alpha(0.8 * fade),
            };
            parent.spawn((place, words(text, size, color.with_alpha(fade)), shadow));
        }
    });
}
