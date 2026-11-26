//! Bevy 클라이언트입니다. 이 워크스페이스에서 Bevy를 사용하는 크레이트는 이것 하나입니다.
//!
//! PC: `cargo run -p warship_client` 또는 `cargo run -p warship_client -- --room duel --server ws://127.0.0.1:7777`
//! 웹: `cd crates/client && trunk serve` 뒤에 `http://127.0.0.1:8080/` 열기 (기본 서버와 기본 방에 들어갑니다)
//!
//! 서버가 Tick마다 보내는, 내 진영에 보이는 방의 상태를 받아서 위에서 내려다본 도형으로 그립니다.
//! 오른쪽 클릭으로 배가 갈 지점을 정합니다. 무기는 스스로 쏩니다.
//! 읽는 순서는 `main`(한 프레임의 순서), `net`, `draw`입니다.

mod draw;
mod net;

use bevy::prelude::*;
use warship_client_core::LaunchOptions;

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
    app.insert_resource(ClearColor(draw::SEA))
        .add_systems(Startup, draw::setup)
        .add_systems(
            Update,
            (
                net::pump_network,
                net::order_moves,
                draw::center_camera,
                draw::draw_room,
                draw::show_status,
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

/// 웹에서는 기본 서버와 기본 방을 씁니다.
#[cfg(target_arch = "wasm32")]
fn read_launch_options() -> Result<LaunchOptions, String> {
    Ok(LaunchOptions::default())
}
