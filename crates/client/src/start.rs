//! 시작 화면입니다. 이름과 방 코드를 글자로 받고, Enter를 누르면 입장을 요청합니다.
//!
//! 칸의 글자와 검사는 `warship_client_core::StartForm`이 맡고, 이 모듈은 눌린 키를 읽어서 넘기고 칸을 화면에 그립니다.
//! 글자는 `KeyboardInput`의 `text`(Shift가 반영된 글자)를 씁니다.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use warship_client_core::{Field, FormKey, JoinRequest, StartForm};

use crate::Screen;

/// 시작 화면의 두 칸입니다.
#[derive(Resource)]
pub struct Start(pub StartForm);

/// 시작 화면이나 실행 인자가 정한 입장 요청입니다. `net::open_room`이 이 요청으로 접속을 엽니다.
#[derive(Resource)]
pub struct Pending(pub JoinRequest);

#[derive(Component)]
pub struct StartText;

pub fn setup(mut commands: Commands) {
    commands.spawn((
        StartText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: percent(30),
            left: percent(30),
            ..default()
        },
    ));
}

/// 눌린 키 하나를 시작 화면의 키로 바꿉니다. 뗀 키와 글자가 없는 키는 없습니다.
/// Tab과 Enter는 누르고 있어도 한 번만 처리하고, 글자와 Backspace는 누르고 있으면 반복합니다.
fn form_keys(event: &KeyboardInput) -> Vec<FormKey> {
    if event.state != ButtonState::Pressed {
        return Vec::new();
    }
    match &event.logical_key {
        Key::Backspace => vec![FormKey::Backspace],
        Key::Tab if !event.repeat => vec![FormKey::Tab],
        Key::Enter if !event.repeat => vec![FormKey::Enter],
        Key::Tab | Key::Enter => Vec::new(),
        _ => {
            let typed = event.text.iter().flat_map(|text| text.chars());
            typed
                .filter(|c| !c.is_control())
                .map(FormKey::Char)
                .collect()
        }
    }
}

/// 시작 화면에서 눌린 키를 칸에 넘기고, Enter로 입장을 요청하면 그 요청을 남깁니다.
pub fn type_keys(
    mut events: MessageReader<KeyboardInput>,
    mut start: ResMut<Start>,
    mut commands: Commands,
) {
    for key in events.read().flat_map(form_keys) {
        if let Some(request) = start.0.on_key(key) {
            commands.insert_resource(Pending(request));
            return;
        }
    }
}

/// 시작 화면을 글자로 그립니다. 방 안에서는 비웁니다.
pub fn show_start(
    screen: Res<Screen>,
    start: Res<Start>,
    mut text: Single<&mut Text, With<StartText>>,
) {
    let lines = match *screen {
        Screen::Start => start_lines(&start.0),
        Screen::Room => String::new(),
    };
    if text.0 != lines {
        text.0 = lines;
    }
}

fn start_lines(form: &StartForm) -> String {
    // 입력을 받는 칸의 앞에 표시를 하고, 글자 뒤에 커서를 붙입니다.
    let field = |label: &str, value: &str, field: Field| {
        if form.focus == field {
            format!("> {label}: {value}_")
        } else {
            format!("  {label}: {value}")
        }
    };
    let mut lines = format!(
        "WARSHIP\n\n{}\n{}\n\nTab: next field   Enter: join\nAn empty room code opens a new room",
        field("Name", &form.name, Field::Name),
        field("Room", &form.room, Field::Room),
    );
    if let Some(notice) = &form.notice {
        lines += &format!("\n\n{notice}");
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(logical_key: Key, text: Option<&str>) -> KeyboardInput {
        KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key,
            state: ButtonState::Pressed,
            text: text.map(Into::into),
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    #[test]
    fn a_pressed_key_gives_its_typed_letters_and_a_released_key_gives_nothing() {
        let letter = key(Key::Character("k".into()), Some("K"));
        assert_eq!(form_keys(&letter), [FormKey::Char('K')]);

        let released = KeyboardInput {
            state: ButtonState::Released,
            ..letter
        };
        assert_eq!(form_keys(&released), []);
        let escape = key(Key::Escape, Some("\u{1b}"));
        assert_eq!(
            form_keys(&escape),
            [],
            "a control character is not a letter"
        );
    }

    #[test]
    fn tab_and_enter_count_once_while_held_and_backspace_repeats() {
        let held = |logical_key| KeyboardInput {
            repeat: true,
            ..key(logical_key, None)
        };
        assert_eq!(form_keys(&key(Key::Tab, None)), [FormKey::Tab]);
        assert_eq!(form_keys(&key(Key::Enter, None)), [FormKey::Enter]);
        assert_eq!(form_keys(&held(Key::Tab)), []);
        assert_eq!(form_keys(&held(Key::Enter)), []);
        assert_eq!(form_keys(&held(Key::Backspace)), [FormKey::Backspace]);
    }

    #[test]
    fn the_start_screen_marks_the_focused_field_and_shows_the_notice() {
        let mut form = StartForm::new("Kang", "duel");
        form.focus = Field::Room;
        form.notice = Some("The room is full".to_string());

        let lines = start_lines(&form);

        assert!(lines.contains("  Name: Kang\n> Room: duel_"), "{lines}");
        assert!(lines.ends_with("The room is full"));
    }
}
