//! Bevy 클라이언트입니다. 이 워크스페이스에서 Bevy를 사용하는 크레이트는 이것 하나입니다.
//!
//! PC: `cargo run -p warship_client` 또는 `cargo run -p warship_client -- --room duel --name Kang --server ws://127.0.0.1:7777`
//! `--new-room`을 주면 서버가 만든 새 방에 들어가고, 화면 왼쪽 위에 그 방의 코드가 나옵니다.
//! 웹: `cd crates/client && trunk serve` 뒤에 `http://127.0.0.1:8080/` 열기. 주소에 `?room=duel&name=Kang&server=ws://127.0.0.1:7777`을 붙이면 바로 입장합니다.
//!
//! 서버가 Tick마다 보내는, 내 진영에 보이는 방의 상태를 받아서 비스듬히 내려다본 3D 도형으로 그립니다. 카메라는 내 배를 따라가고, 방향키나 창의 가장자리로 화면을 옮기고, Space로 내 배로 돌아오고, 마우스 휠로 거리를 바꿉니다. 오른쪽 아래에 미니맵이 있습니다.
//! 오른쪽 클릭으로 배가 갈 지점을 정합니다. 무기는 스스로 쏩니다. 상점과 스킬은 단축키로 씁니다.
//! 실행 인자 없이 시작하면 시작 화면에서 이름과 방 코드를 받습니다. 방 안에서 Esc를 누르면 나갈지 묻고, Enter로 확인하면 시작 화면으로 돌아옵니다.
//! 기기는 마지막 이름과 방, 자리의 표식을 기억합니다. 그래서 다시 실행해서 같은 방에 들어가면 같은 배로 돌아갑니다.
//! 읽는 순서는 `main`(한 프레임의 순서), `start`, `net`, `storage`, `draw`, `camera`, `minimap`, `hud`입니다.

mod camera;
mod draw;
mod hud;
mod minimap;
mod net;
mod shapes;
mod shop;
mod start;
mod storage;

use bevy::prelude::*;
use warship_client_core::LaunchOptions;

/// 지금 보이는 화면입니다.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// 이름과 방 코드를 받는 시작 화면입니다.
    Start,
    /// 방의 화면입니다. 접속 상태(`net::Net`)가 있습니다.
    Room,
}

fn main() {
    let launch = read_launch_options();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "WARSHIP".to_string(),
            // 웹에서는 index.html의 이 캔버스에 그립니다. PC에서는 무시됩니다.
            canvas: Some("#bevy".to_string()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }));
    net::install(&mut app, launch);
    let in_room = resource_equals(Screen::Room);
    app.insert_resource(ClearColor(draw::SEA))
        .insert_resource(Screen::Start)
        .init_resource::<camera::Rig>()
        .init_resource::<net::LeavePrompt>()
        .init_resource::<shop::ShopOpen>()
        .add_systems(
            Startup,
            (
                draw::setup,
                hud::setup,
                start::setup,
                minimap::setup,
                shop::setup,
            ),
        )
        .add_systems(
            Update,
            (
                start::type_keys.run_if(resource_equals(Screen::Start)),
                net::open_room.run_if(resource_exists::<start::Pending>),
                (
                    net::pump_network,
                    net::order_moves,
                    hud::order_commands,
                    shop::use_shop,
                    net::leave_room,
                    draw::build_scenery,
                    draw::sync_pieces,
                    camera::steer_camera,
                    draw::draw_overlays,
                )
                    .chain()
                    .run_if(in_room),
                net::close_room.run_if(resource_exists::<net::Closing>),
                draw::clear_room.run_if(resource_equals(Screen::Start)),
                draw::show_status,
                draw::show_prompt,
                hud::show_hud,
                start::show_start,
                minimap::show_minimap,
                shop::show_shop,
            )
                .chain(),
        )
        .run();
}

/// PC에서는 실행 인자를 읽습니다. 잘못된 인자는 이유와 함께 돌려주고, 화면이 그 이유를 보여 줍니다.
#[cfg(not(target_arch = "wasm32"))]
fn read_launch_options() -> Result<LaunchOptions, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    LaunchOptions::parse(&args)
}

/// 웹에서는 페이지 주소의 쿼리(`?room=duel&name=Kang&server=wss://example.test`)를 읽습니다.
/// 쿼리가 없으면 기본 서버를 쓰고, 시작 화면에서 이름과 방 코드를 받습니다.
#[cfg(target_arch = "wasm32")]
fn read_launch_options() -> Result<LaunchOptions, String> {
    let search = web_sys::window().and_then(|window| window.location().search().ok());
    LaunchOptions::from_query(&search.unwrap_or_default())
}
