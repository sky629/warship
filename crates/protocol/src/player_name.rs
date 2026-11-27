//! 플레이어의 이름입니다. 서버는 `Join`의 이름을, 클라이언트는 시작 화면의 이름 칸을 같은 함수로 검사합니다.
//! 규칙 크레이트는 이름을 알지 못합니다. 이름은 서버의 자리(`Seat`)와 명단 메시지에만 있습니다.

use std::fmt;

/// 앞뒤 공백을 지운 뒤 1자 이상 `MAX_CHARS`자 이하이고, 영문 대소문자, 숫자, 공백, `-`, `_`만 있는 이름입니다. 같은 이름을 허용합니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerName(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong { max: usize },
    BadChar(char),
}

impl PlayerName {
    pub const MAX_CHARS: usize = 12;

    /// 이름에 넣을 수 있는 문자입니다. 시작 화면의 이름 칸이 이 함수로 입력을 거릅니다.
    pub fn allows(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_'
    }

    pub fn parse(raw: &str) -> Result<PlayerName, NameError> {
        let name = raw.trim();
        if name.is_empty() {
            return Err(NameError::Empty);
        }
        if let Some(bad) = name.chars().find(|&c| !PlayerName::allows(c)) {
            return Err(NameError::BadChar(bad));
        }
        // 위에서 ASCII 문자만 남겼으므로 바이트 수가 글자 수와 같습니다.
        if name.len() > Self::MAX_CHARS {
            return Err(NameError::TooLong {
                max: Self::MAX_CHARS,
            });
        }
        Ok(PlayerName(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlayerName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 시작 화면에 그대로 보이는 문장이므로 영어입니다.
impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NameError::Empty => write!(f, "Enter a name"),
            NameError::TooLong { max } => write!(f, "Name is longer than {max} letters"),
            NameError::BadChar(c) => write!(
                f,
                "Name cannot contain '{c}'. Use letters, digits, spaces, - or _"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces_around_the_name_are_dropped_and_the_rest_is_kept() {
        let name = PlayerName::parse("  Kang Lee ").unwrap();
        assert_eq!(name.as_str(), "Kang Lee");
    }

    #[test]
    fn a_name_of_twelve_letters_is_the_longest_allowed() {
        assert!(PlayerName::parse("ABCDEFGHIJKL").is_ok());
        assert_eq!(
            PlayerName::parse("ABCDEFGHIJKLM"),
            Err(NameError::TooLong { max: 12 })
        );
    }

    #[test]
    fn the_length_is_counted_after_the_spaces_are_dropped() {
        assert!(PlayerName::parse("   ABCDEFGHIJKL   ").is_ok());
    }

    #[test]
    fn a_blank_name_is_empty() {
        assert_eq!(PlayerName::parse(""), Err(NameError::Empty));
        assert_eq!(PlayerName::parse("   "), Err(NameError::Empty));
    }

    #[test]
    fn only_letters_digits_spaces_dashes_and_underscores_are_allowed() {
        assert!(PlayerName::parse("Kang_7-b c").is_ok());
        assert_eq!(PlayerName::parse("한"), Err(NameError::BadChar('한')));
        assert_eq!(PlayerName::parse("kang!"), Err(NameError::BadChar('!')));
        assert_eq!(PlayerName::parse("a\tb"), Err(NameError::BadChar('\t')));
        assert_eq!(PlayerName::parse("a\nb"), Err(NameError::BadChar('\n')));
    }

    #[test]
    fn the_same_name_is_allowed_twice() {
        assert_eq!(PlayerName::parse("KANG"), PlayerName::parse("KANG"));
    }

    #[test]
    fn every_error_is_a_sentence_for_the_screen() {
        assert_eq!(NameError::Empty.to_string(), "Enter a name");
        assert_eq!(
            NameError::TooLong { max: 12 }.to_string(),
            "Name is longer than 12 letters"
        );
        assert!(NameError::BadChar('!').to_string().contains("'!'"));
    }
}
