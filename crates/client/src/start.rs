//! 시작 화면입니다. 화면 가운데의 패널에서 이름과 방 코드를 글자로 받고, Enter를 누르면 입장을 요청합니다.
//!
//! 칸의 글자와 검사는 `warship_client_core::StartForm`이 맡고, 이 모듈은 눌린 키를 읽어서 넘기고 칸을 화면에 그립니다.
//! 글자는 `KeyboardInput`의 `text`(Shift가 반영된 글자)를 씁니다.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::ui::widget::NodeImageMode;
use warship_client_core::{Field, FormKey, JoinRequest, StartForm};

use crate::Screen;
use crate::ui::{self, BAD, DIM, GOLD, LIT, Panel, SLOT, TEXT, words};

/// 시작 화면의 두 칸입니다.
#[derive(Resource)]
pub struct Start(pub StartForm);

/// 시작 화면이나 실행 인자가 정한 입장 요청입니다. `net::open_room`이 이 요청으로 접속을 엽니다.
#[derive(Resource)]
pub struct Pending(pub JoinRequest);

/// 시작 화면 전체입니다. 가운데에 패널이 있습니다.
#[derive(Component)]
pub struct StartRoot;

/// 시작 화면의 패널입니다. 제목, 두 칸, 안내가 들어갑니다.
#[derive(Component)]
pub struct StartPanel;

/// 시작 화면의 바탕 그림과 제목 그림입니다. 에셋 폴더에 있으면 씁니다.
const BACKGROUND: &str = "ui/start.png";
const LOGO: &str = "ui/logo.png";

pub fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let root = Node {
        position_type: PositionType::Absolute,
        width: percent(100),
        height: percent(100),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let panel = Node {
        width: px(420),
        padding: UiRect::all(px(28)),
        row_gap: px(10),
        flex_direction: FlexDirection::Column,
        ..default()
    };
    let panel = (StartPanel, Panel, panel, BackgroundColor(ui::PANEL));
    let mut root = commands.spawn((StartRoot, root, children![panel]));
    if ui::has(BACKGROUND) {
        let image = ImageNode::new(assets.load(BACKGROUND)).with_mode(NodeImageMode::Stretch);
        root.insert(image);
    }
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
/// 시작 화면을 그립니다. 방 안에서는 숨깁니다.
pub fn show_start(
    screen: Res<Screen>,
    start: Res<Start>,
    assets: Res<AssetServer>,
    mut root: Single<&mut Node, With<StartRoot>>,
    panel: Single<Entity, With<StartPanel>>,
    mut commands: Commands,
) {
    commands.entity(*panel).despawn_children();
    if *screen != Screen::Start {
        root.display = Display::None;
        return;
    }
    root.display = Display::Flex;
    let form = &start.0;
    commands.entity(*panel).with_children(|parent| {
        if ui::has(LOGO) {
            let logo = Node {
                height: px(96),
                ..default()
            };
            parent.spawn((logo, ImageNode::new(assets.load(LOGO))));
        } else {
            parent.spawn(words("WARSHIP", 44.0, GOLD));
        }
        for (label, field) in [("Name", Field::Name), ("Room code", Field::Room)] {
            parent.spawn(words(label, 13.0, DIM));
            let focused = form.focus == field;
            let boxed = Node {
                height: px(36),
                padding: UiRect::horizontal(px(10)),
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            };
            let back = if focused { LIT } else { SLOT };
            parent
                .spawn((boxed, BackgroundColor(back)))
                .with_children(|boxed| {
                    boxed.spawn(words(field_text(form, field), 18.0, TEXT));
                });
        }
        parent.spawn(words("Tab: next field   Enter: join", 12.0, DIM));
        parent.spawn(words("An empty room code opens a new room", 12.0, DIM));
        if let Some(notice) = &form.notice {
            parent.spawn(words(notice.clone(), 14.0, BAD));
        }
    });
}

/// 칸에 보여 주는 글자입니다. 입력을 받는 칸은 글자 뒤에 커서가 붙습니다.
fn field_text(form: &StartForm, field: Field) -> String {
    let value = match field {
        Field::Name => &form.name,
        Field::Room => &form.room,
    };
    let cursor = if form.focus == field { "_" } else { "" };
    format!("{value}{cursor}")
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
    fn the_focused_field_shows_a_cursor_after_its_text() {
        let mut form = StartForm::new("Kang", "duel");
        form.focus = Field::Room;

        assert_eq!(field_text(&form, Field::Name), "Kang");
        assert_eq!(field_text(&form, Field::Room), "duel_");
    }
}
