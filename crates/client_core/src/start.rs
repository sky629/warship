//! 시작 화면의 입력 칸입니다. 이름과 방 코드를 받아서 입장할 방을 정합니다.
//!
//! 이 모듈은 눌린 키(`FormKey`)를 받아서 칸의 글자를 바꾸고, Enter를 누르면 검사한 결과를 돌려줍니다.
//! 키를 읽는 것과 화면에 그리는 것은 클라이언트(`start`)가 합니다. 검사는 서버와 같은 함수(`PlayerName`, `RoomCode`)로 합니다.

use warship_protocol::{PlayerName, RoomCode};

/// 시작 화면이 받는 키입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormKey {
    Char(char),
    Backspace,
    /// 다음 칸으로 갑니다.
    Tab,
    /// 입장합니다.
    Enter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Room,
}

/// 시작 화면에서 정한 입장 요청입니다. 값은 검사를 통과한 글자입니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinRequest {
    pub name: String,
    /// 들어갈 방의 코드입니다. 없으면 서버가 새 방을 만듭니다.
    pub room: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartForm {
    pub name: String,
    /// 방 코드 칸입니다. 비어 있으면 새 방입니다.
    pub room: String,
    pub focus: Field,
    /// 화면에 보여 줄 문장입니다. 입장하지 못한 이유나 방에서 나온 이유입니다.
    pub notice: Option<String>,
}

impl StartForm {
    pub fn new(name: &str, room: &str) -> StartForm {
        StartForm {
            name: name.to_string(),
            room: room.to_string(),
            focus: Field::Name,
            notice: None,
        }
    }

    /// 키 하나를 처리합니다. Enter를 눌렀고 두 칸이 검사를 통과하면 입장 요청을 돌려줍니다.
    /// 칸에 넣을 수 없는 문자와 칸의 길이를 넘는 문자는 버립니다. 방 코드 칸은 대문자를 소문자로 바꿉니다.
    pub fn on_key(&mut self, key: FormKey) -> Option<JoinRequest> {
        match (key, self.focus) {
            (FormKey::Char(c), Field::Name) => {
                if PlayerName::allows(c) && self.name.len() < PlayerName::MAX_CHARS {
                    self.name.push(c);
                }
            }
            (FormKey::Char(c), Field::Room) => {
                let c = c.to_ascii_lowercase();
                if RoomCode::allows(c) && self.room.len() < RoomCode::MAX_LEN {
                    self.room.push(c);
                }
            }
            (FormKey::Backspace, Field::Name) => {
                self.name.pop();
            }
            (FormKey::Backspace, Field::Room) => {
                self.room.pop();
            }
            (FormKey::Tab, Field::Name) => self.focus = Field::Room,
            (FormKey::Tab, Field::Room) => self.focus = Field::Name,
            (FormKey::Enter, _) => return self.submit(),
        }
        None
    }

    /// 두 칸을 검사해서 입장 요청을 만듭니다. 검사에 걸리면 그 칸으로 가서 이유를 보여 줍니다.
    pub fn submit(&mut self) -> Option<JoinRequest> {
        let name = match PlayerName::parse(&self.name) {
            Ok(name) => name,
            Err(err) => {
                self.focus = Field::Name;
                self.notice = Some(err.to_string());
                return None;
            }
        };
        let room = if self.room.trim().is_empty() {
            None
        } else {
            match RoomCode::parse(&self.room) {
                Ok(room) => Some(room.as_str().to_string()),
                Err(err) => {
                    self.focus = Field::Room;
                    self.notice = Some(err.to_string());
                    return None;
                }
            }
        };
        self.notice = None;
        Some(JoinRequest {
            name: name.as_str().to_string(),
            room,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_text(form: &mut StartForm, text: &str) {
        for c in text.chars() {
            assert_eq!(form.on_key(FormKey::Char(c)), None);
        }
    }

    #[test]
    fn typing_fills_the_focused_field_and_tab_moves_between_the_fields() {
        let mut form = StartForm::new("", "");
        type_text(&mut form, "Kang");
        form.on_key(FormKey::Tab);
        type_text(&mut form, "Duel-1");
        form.on_key(FormKey::Backspace);
        form.on_key(FormKey::Tab);
        form.on_key(FormKey::Backspace);

        assert_eq!((form.name.as_str(), form.room.as_str()), ("Kan", "duel-"));
        assert_eq!(form.focus, Field::Name);
    }

    #[test]
    fn letters_a_field_cannot_hold_are_dropped() {
        let mut form = StartForm::new("", "");
        type_text(&mut form, "K@ng!ABCDEFGHIJKLMN");
        assert_eq!(form.name, "KngABCDEFGHI", "twelve letters at most");

        form.on_key(FormKey::Tab);
        type_text(&mut form, "my room#1");
        assert_eq!(form.room, "myroom1");
    }

    #[test]
    fn enter_joins_the_room_of_the_code_or_a_new_room_when_the_code_is_empty() {
        let mut form = StartForm::new(" Kang ", "duel");
        let request = form.on_key(FormKey::Enter);
        let named = |room: Option<&str>| JoinRequest {
            name: "Kang".to_string(),
            room: room.map(str::to_string),
        };
        assert_eq!(request, Some(named(Some("duel"))));

        let mut form = StartForm::new("Kang", "");
        assert_eq!(form.on_key(FormKey::Enter), Some(named(None)));
    }

    #[test]
    fn enter_with_an_empty_name_shows_the_reason_and_moves_to_the_name() {
        let mut form = StartForm::new("", "duel");
        form.focus = Field::Room;

        assert_eq!(form.on_key(FormKey::Enter), None);

        assert_eq!(form.notice.as_deref(), Some("이름을 입력하십시오"));
        assert_eq!(form.focus, Field::Name);
        type_text(&mut form, "Kang");
        assert!(form.on_key(FormKey::Enter).is_some());
        assert_eq!(form.notice, None);
    }
}
