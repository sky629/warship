//! 접속과 입력입니다. 입장 요청으로 접속을 열고, 접속 방식에 도착한 이벤트를 `Session`에 넘기고,
//! 클릭을 이동 입력으로, 단축키의 명령을 서버로 보냅니다. Esc를 누르면 나갈지 묻고, Enter로 확인하면 방을 나가서 시작 화면으로 돌아갑니다.

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use warship_client_core::{
    DEFAULT_NAME, Frame, LaunchOptions, Memory, Phase, RememberedSeat, RoomChoice, Session,
    StartForm,
};
use warship_protocol::{ClientEvent, ClientTransport, PlayerName, RoomCode, WireCommand};
use warship_ws_client::WsClient;

use crate::camera::minimap_under;
use crate::start::{Pending, Start};
use crate::{Screen, storage};

/// 방 하나에 대한 접속 방식과 접속 상태입니다. 방 안에 있는 동안에만 있습니다.
/// 웹의 WebSocket은 다른 스레드로 옮길 수 없으므로 non-send 리소스로 보관합니다.
/// Bevy는 non-send 리소스를 사용하는 시스템을 항상 메인 스레드에서 실행합니다. 접속을 열지 못했으면 `transport`가 없습니다.
pub struct Net {
    transport: Option<Box<dyn ClientTransport>>,
    pub session: Session,
}

impl Net {
    /// 구매, 판매, 선체 교체, 스킬의 명령을 보냅니다. 방 안에 있지 않으면 아무 일도 하지 않습니다.
    pub fn send_command(&mut self, command: WireCommand) {
        if let (Some(order), Some(transport)) = (self.session.command(command), &mut self.transport)
        {
            transport.send(&order);
        }
    }
}

/// 이 기기의 설정과 기억입니다. 기억이 바뀌면 곧바로 저장합니다.
#[derive(Resource)]
pub struct Device {
    /// 서버 주소입니다.
    server: String,
    /// 기억을 따로 쓰는 이름입니다(`--profile`).
    profile: Option<String>,
    memory: Memory,
}

impl Device {
    fn save(&self) {
        storage::save(self.profile.as_deref(), &self.memory.to_text());
    }
}

/// 방을 나갈지 묻는 중인지입니다. 묻는 동안 화면 한가운데에 확인창이 보입니다.
#[derive(Resource, Default)]
pub struct LeavePrompt(pub bool);

/// Esc와 Enter에 따라 나가기 확인이 다음에 할 일입니다.
#[derive(Debug, PartialEq, Eq)]
enum LeaveStep {
    /// 아무 일도 없습니다.
    Wait,
    /// 확인창을 엽니다.
    Ask,
    /// 확인창을 닫고 방에 남습니다.
    Stay,
    /// 방을 나갑니다.
    Leave,
}

/// 접속이 이미 끝났으면 Esc로 바로 나갑니다. 방 안에 있으면 Esc가 확인창을 열고, 확인창에서 Enter는 나가고 Esc는 남습니다.
fn leave_step(asking: bool, ended: bool, esc: bool, enter: bool) -> LeaveStep {
    match (asking, ended, esc, enter) {
        (_, true, true, _) | (true, _, _, true) => LeaveStep::Leave,
        (true, _, true, _) => LeaveStep::Stay,
        (false, _, true, _) => LeaveStep::Ask,
        _ => LeaveStep::Wait,
    }
}

/// 방을 나가기로 했습니다. 값은 시작 화면에 보여 줄 문장입니다.
#[derive(Resource)]
pub struct Closing(String);

/// 이번 프레임에 그릴 방의 모습입니다. 입장하기 전에는 없습니다.
#[derive(Resource, Default)]
pub struct CurrentFrame(pub Option<Frame>);

/// 마지막으로 클릭한 이동 지점입니다(타일 좌표).
#[derive(Resource, Default)]
pub struct Destination(pub Option<Vec2>);

/// 기기의 기억을 읽고 시작 화면을 준비합니다. 실행 인자가 방을 정했으면 시작 화면 없이 입장을 요청합니다.
/// 실행 인자가 잘못되었으면 그 이유를 시작 화면에 보여 줍니다.
pub fn install(app: &mut App, launch: Result<LaunchOptions, String>) {
    let (options, problem) = match launch {
        Ok(options) => (options, None),
        Err(problem) => (LaunchOptions::default(), Some(problem)),
    };
    let memory = storage::load(options.profile.as_deref());
    let remembered = memory.name.as_ref().map(|name| name.as_str().to_string());
    let name = options.name.or(remembered);
    let name = name.unwrap_or_else(|| DEFAULT_NAME.to_string());
    let room = memory.room.as_ref().map_or("", RoomCode::as_str);
    let mut form = StartForm::new(&name, room);
    let request = match (&problem, options.room) {
        (None, RoomChoice::New) => {
            form.room.clear();
            form.submit()
        }
        (None, RoomChoice::Code(code)) => {
            form.room = code;
            form.submit()
        }
        _ => None,
    };
    if problem.is_some() {
        form.notice = problem;
    }
    if let Some(request) = request {
        app.insert_resource(Pending(request));
    }
    app.init_resource::<CurrentFrame>()
        .init_resource::<Destination>()
        .insert_resource(Start(form))
        .insert_resource(Device {
            server: options.server,
            profile: options.profile,
            memory,
        });
}

/// 입장 요청으로 접속을 열고 방의 화면으로 갑니다. 기기가 이 서버의 그 방의 자리를 기억하면 그 표식을 냅니다.
/// 접속을 열지 못하면 그 이유로 끝난 접속 상태가 되고, 방의 화면이 이유를 보여 줍니다.
pub fn open_room(world: &mut World) {
    let Some(Pending(request)) = world.remove_resource::<Pending>() else {
        return;
    };
    let mut device = world.resource_mut::<Device>();
    let room = request.room.as_deref();
    let token = device.memory.token_for(&device.server, room);
    let mut session = Session::new(room, &request.name, token);
    info!(
        "connecting to {} ({})",
        device.server,
        room.unwrap_or("a new room")
    );
    let transport: Option<Box<dyn ClientTransport>> = match WsClient::connect(&device.server) {
        Ok(client) => Some(Box::new(client)),
        Err(reason) => {
            session.handle(ClientEvent::Disconnected { reason }, 0.0);
            None
        }
    };
    device.memory.name = PlayerName::parse(&request.name).ok();
    device.save();
    world.insert_non_send(Net { transport, session });
    world.insert_resource(LeavePrompt(false));
    world.insert_resource(Screen::Room);
}

/// 방을 나와서 시작 화면으로 돌아갑니다. 앉았던 자리는 기억에서 지우고, 방 코드 칸에는 나온 방의 코드를 남깁니다.
pub fn close_room(world: &mut World) {
    let Some(Closing(notice)) = world.remove_resource::<Closing>() else {
        return;
    };
    let net = world.remove_non_send::<Net>();
    let room = net.and_then(|net| net.session.welcome().map(|welcome| welcome.room.clone()));
    let mut device = world.resource_mut::<Device>();
    if device.memory.seat.take().is_some() {
        device.save();
    }
    let mut start = world.resource_mut::<Start>();
    if let Some(room) = room {
        start.0.room = room;
    }
    start.0.notice = Some(notice);
    world.resource_mut::<CurrentFrame>().0 = None;
    world.resource_mut::<Destination>().0 = None;
    world.resource_mut::<LeavePrompt>().0 = false;
    // 방 안에서 누른 키(확인의 Enter 등)가 시작 화면의 칸에 들어가지 않게, 아직 읽지 않은 키 입력을 버립니다.
    world.resource_mut::<Messages<KeyboardInput>>().clear();
    world.insert_resource(Screen::Start);
}

/// 접속 방식에 도착한 이벤트를 모두 처리하고, 이번 프레임에 그릴 방의 모습을 정합니다.
/// 방 안에 있다가 접속이 끊겼으면 새 접속을 열고, 방 안에 있는 동안에는 접속이 살아 있다는 표시를 보냅니다.
/// 입장 안내로 받은 자리가 기억과 다르면 기억을 고쳐서 저장합니다.
pub fn pump_network(
    mut net: NonSendMut<Net>,
    mut device: ResMut<Device>,
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
    if session.should_reconnect(now) {
        info!("connecting to {} again", device.server);
        match WsClient::connect(&device.server) {
            Ok(client) => *transport = Some(Box::new(client)),
            Err(reason) => {
                session.handle(ClientEvent::Disconnected { reason }, now);
            }
        }
    }
    if let (Some(ping), Some(transport)) = (session.ping(now), transport.as_mut()) {
        transport.send(&ping);
    }
    let seat = session.welcome().and_then(|welcome| {
        let room = RoomCode::parse(&welcome.room).ok()?;
        Some(RememberedSeat {
            server: device.server.clone(),
            room,
            token: welcome.token,
        })
    });
    if seat.is_some() && seat != device.memory.seat {
        device.memory.room = seat.as_ref().map(|seat| seat.room.clone());
        device.memory.seat = seat;
        device.save();
    }
    frame.0 = session.frame(now);
}

/// Esc를 누르면 방을 나갈지 묻고, 확인하면 나갑니다. 접속이 이미 끝났으면 묻지 않고 나갑니다.
/// 접속이 끝난 이유나 스스로 나갔다는 문장을 시작 화면에 남깁니다.
pub fn leave_room(
    keys: Res<ButtonInput<KeyCode>>,
    mut net: NonSendMut<Net>,
    mut prompt: ResMut<LeavePrompt>,
    mut commands: Commands,
) {
    let Net { transport, session } = &mut *net;
    let ended = match session.phase() {
        Phase::Ended(reason) => Some(reason.clone()),
        _ => None,
    };
    let esc = keys.just_pressed(KeyCode::Escape);
    let enter = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter);
    match leave_step(prompt.0, ended.is_some(), esc, enter) {
        LeaveStep::Wait => return,
        LeaveStep::Ask => {
            prompt.0 = true;
            return;
        }
        LeaveStep::Stay => {
            prompt.0 = false;
            return;
        }
        LeaveStep::Leave => {}
    }
    if let (Some(leave), Some(transport)) = (session.leave(), transport) {
        transport.send(&leave);
    }
    let notice = ended.unwrap_or_else(|| "You left the room".to_string());
    commands.insert_resource(Closing(notice));
}

/// 오른쪽 클릭한 수면의 자리나 미니맵의 자리로 배를 보냅니다.
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
    // 미니맵 위를 눌렀으면 미니맵이 가리키는 자리이고, 아니면 커서를 지나는 시선이 수면(높이 0)과 만나는 점입니다.
    // 시선이 수면과 나란하거나 위를 향하면 만나지 않습니다.
    let on_minimap = net.session.welcome().and_then(|welcome| {
        let (_, tile) = minimap_under(&window, welcome);
        tile.map(|tile| Vec2::new(tile.x, tile.y))
    });
    let on_water = || {
        let cursor = window.cursor_position()?;
        let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
        let hits = ray.direction.y < -1e-3;
        let water = ray.origin + ray.direction * (-ray.origin.y / ray.direction.y);
        hits.then_some(Vec2::new(water.x, -water.z))
    };
    let Some(point) = on_minimap.or_else(on_water) else {
        return;
    };
    let Net { transport, session } = &mut *net;
    if let (Some(order), Some(transport)) = (session.sail_to(point.x, point.y), transport) {
        transport.send(&order);
        destination.0 = Some(point);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_asks_first_and_enter_confirms() {
        assert_eq!(leave_step(false, false, false, false), LeaveStep::Wait);
        assert_eq!(leave_step(false, false, true, false), LeaveStep::Ask);
        assert_eq!(
            leave_step(false, false, false, true),
            LeaveStep::Wait,
            "nothing to confirm"
        );
        assert_eq!(leave_step(true, false, false, true), LeaveStep::Leave);
        assert_eq!(leave_step(true, false, true, false), LeaveStep::Stay);
        assert_eq!(leave_step(true, false, false, false), LeaveStep::Wait);
    }

    #[test]
    fn escape_leaves_at_once_when_the_link_has_ended() {
        assert_eq!(leave_step(false, true, true, false), LeaveStep::Leave);
        assert_eq!(leave_step(false, true, false, false), LeaveStep::Wait);
    }
}
