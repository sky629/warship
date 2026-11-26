//! 접속과 입력입니다. 접속 방식에 도착한 이벤트를 `Session`에 넘기고, 클릭을 이동 입력으로 보냅니다.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use warship_client_core::{Frame, LaunchOptions, Session};
use warship_protocol::{ClientEvent, ClientTransport};
use warship_ws_client::WsClient;

use crate::draw::TILE_PX;

/// 접속 방식과 접속 상태입니다. 웹의 WebSocket은 다른 스레드로 옮길 수 없으므로 non-send 리소스로 보관합니다.
/// Bevy는 non-send 리소스를 사용하는 시스템을 항상 메인 스레드에서 실행합니다. 접속을 열지 못했으면 `transport`가 없습니다.
pub struct Net {
    transport: Option<Box<dyn ClientTransport>>,
    pub session: Session,
}

/// 이번 프레임에 그릴 방의 모습입니다. 입장하기 전에는 없습니다.
#[derive(Resource, Default)]
pub struct CurrentFrame(pub Option<Frame>);

/// 마지막으로 클릭한 이동 지점입니다(타일 좌표).
#[derive(Resource, Default)]
pub struct Destination(pub Option<Vec2>);

/// 접속을 시작하고 리소스를 넣습니다. 실행 인자가 잘못되었거나 접속을 열지 못하면 그 이유로 끝난 접속 상태를 넣습니다.
pub fn install(app: &mut App, launch: Result<LaunchOptions, String>) {
    let room = launch.as_ref().map_or("", |launch| launch.room.as_str());
    let mut session = Session::new(room);
    let opened = launch.and_then(|launch| {
        info!("connecting to {} (room {})", launch.server, launch.room);
        WsClient::connect(&launch.server)
    });
    let transport: Option<Box<dyn ClientTransport>> = match opened {
        Ok(client) => Some(Box::new(client)),
        Err(reason) => {
            session.handle(ClientEvent::Disconnected { reason }, 0.0);
            None
        }
    };
    app.init_resource::<CurrentFrame>()
        .init_resource::<Destination>()
        .insert_non_send(Net { transport, session });
}

/// 접속 방식에 도착한 이벤트를 모두 처리하고, 이번 프레임에 그릴 방의 모습을 정합니다.
pub fn pump_network(
    mut net: NonSendMut<Net>,
    time: Res<Time<Real>>,
    mut frame: ResMut<CurrentFrame>,
) {
    let now = time.elapsed_secs_f64();
    let Net { transport, session } = &mut *net;
    if let Some(transport) = transport {
        while let Some(event) = transport.poll() {
            if let Some(reply) = session.handle(event, now) {
                transport.send(&reply);
            }
        }
    }
    frame.0 = session.frame(now);
}

/// 오른쪽 클릭한 자리로 배를 보냅니다.
pub fn order_moves(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut net: NonSendMut<Net>,
    mut destination: ResMut<Destination>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let (camera, camera_transform) = *camera;
    let clicked = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok());
    let Some(point) = clicked.map(|world| world / TILE_PX) else {
        return;
    };
    let Net { transport, session } = &mut *net;
    if let (Some(order), Some(transport)) = (session.sail_to(point.x, point.y), transport) {
        transport.send(&order);
        destination.0 = Some(point);
    }
}
