//! Bevy 클라이언트입니다. 이 워크스페이스에서 Bevy를 사용하는 크레이트는 이것 하나입니다.
//!
//! PC: `cargo run -p warship_client`
//! 웹: `cd crates/client && trunk serve` 뒤에 `http://127.0.0.1:8080/` 열기

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "WARSHIP".to_string(),
                // 웹에서는 index.html의 이 캔버스에 그립니다. PC에서는 무시됩니다.
                canvas: Some("#bevy".to_string()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .run();
}
