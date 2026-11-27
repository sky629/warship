//! 자리의 표식입니다. 서버가 입장을 받아들일 때 만들어서 `Welcome`으로 알려 주고, 클라이언트가 기기에 저장했다가
//! 다시 입장할 때 `Join`에 실어서 자기 자리를 알립니다. 표식을 아는 사람은 그 자리를 가져갈 수 있으므로,
//! 표식은 링크와 로그에 넣지 않습니다. `Debug`는 값을 보여 주지 않습니다.

use std::fmt;

use serde::{Deserialize, Serialize};

/// 16바이트(128비트)입니다. postcard로 길이 없이 16바이트가 됩니다. 기기에는 32자의 16진수 글자로 저장합니다.
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct SeatToken(pub [u8; 16]);

/// 같은지 알아내는 시간이 앞에서부터 맞는 바이트의 수에 따르지 않도록, 16바이트를 모두 본 뒤에 결과를 냅니다.
/// 걸린 시간으로 표식을 한 바이트씩 알아내는 일을 막습니다.
impl PartialEq for SeatToken {
    fn eq(&self, other: &SeatToken) -> bool {
        let differing_bits = self
            .0
            .iter()
            .zip(&other.0)
            .fold(0u8, |bits, (mine, theirs)| bits | (mine ^ theirs));
        differing_bits == 0
    }
}

impl Eq for SeatToken {}

impl SeatToken {
    /// 소문자 16진수 32자입니다.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// `to_hex`가 만든 글자를 되돌립니다. 길이나 문자가 맞지 않으면 `None`입니다. 기기에 저장된 글자는 믿을 수 없으므로 하나라도 틀리면 버립니다.
    pub fn from_hex(text: &str) -> Option<SeatToken> {
        if text.len() != 32 || !text.is_ascii() {
            return None;
        }
        let mut bytes = [0u8; 16];
        for (byte, pair) in bytes.iter_mut().zip(text.as_bytes().chunks(2)) {
            // 위에서 ASCII만 남겼으므로 두 바이트가 곧 두 글자입니다.
            let pair = std::str::from_utf8(pair).ok()?;
            if !pair.bytes().all(|digit| digit.is_ascii_hexdigit()) {
                return None;
            }
            *byte = u8::from_str_radix(pair, 16).ok()?;
        }
        Some(SeatToken(bytes))
    }
}

/// `--debug-net`의 로그와 전용 서버의 로그는 메시지를 `{:?}`로 출력하므로, 표식의 값이 로그에 남지 않게 합니다.
impl fmt::Debug for SeatToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SeatToken(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: SeatToken = SeatToken([
        0x00, 0x01, 0x7f, 0x80, 0xab, 0xcd, 0xef, 0xff, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70,
        0x08,
    ]);

    #[test]
    fn a_token_survives_its_hex_text() {
        let text = TOKEN.to_hex();
        assert_eq!(text, "00017f80abcdefff1020304050607008");
        assert_eq!(SeatToken::from_hex(&text), Some(TOKEN));
        assert_eq!(
            SeatToken::from_hex(&text.to_uppercase()),
            Some(TOKEN),
            "the case of the digits does not matter"
        );
    }

    #[test]
    fn two_tokens_are_equal_only_when_every_byte_is_equal() {
        assert_eq!(TOKEN, SeatToken(TOKEN.0));
        for index in 0..16 {
            for bit in 0..8 {
                let mut other = TOKEN.0;
                other[index] ^= 1 << bit;
                assert_ne!(TOKEN, SeatToken(other), "byte {index} bit {bit}");
            }
        }
    }

    #[test]
    fn text_of_the_wrong_length_or_with_other_characters_is_not_a_token() {
        for text in [
            "",
            "00017f80abcdefff102030405060700",
            "00017f80abcdefff10203040506070088",
            "00017f80abcdefff1020304050607g08",
            "00017f80abcdefff102030405060 008",
            "+0017f80abcdefff1020304050607008",
            "한한한한한한한한한한한한한한한한",
        ] {
            assert_eq!(SeatToken::from_hex(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_message_that_carries_the_token_does_not_show_it_when_printed() {
        let join = crate::ClientMsg::Join {
            version: 9,
            room: None,
            name: "KANG".to_string(),
            token: Some(TOKEN),
        };
        let shown = format!("{join:?} {join:#?}");
        assert!(
            shown.contains("KANG") && shown.contains("SeatToken(..)"),
            "{shown}"
        );
        assert!(!shown.contains(&TOKEN.to_hex()), "{shown}");
        assert!(!shown.contains("171") && !shown.contains("0x"), "{shown}");
    }

    #[test]
    fn the_debug_text_never_shows_the_value() {
        let shown = format!("{:?} {:#?}", TOKEN, TOKEN);
        assert!(!shown.contains("7f") && !shown.contains("80"), "{shown}");
        assert_eq!(format!("{TOKEN:?}"), "SeatToken(..)");
    }
}
