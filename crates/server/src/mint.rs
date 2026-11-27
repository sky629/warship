//! 서버가 만드는 새 방의 코드와 자리의 표식입니다.
//!
//! 둘 다 무작위 원천(`RandomSource`)이 준 바이트로 만듭니다. 표식은 16바이트(128비트)를 그대로 쓰고, 방 코드는 바이트마다 글자 하나를 고릅니다.
//! 공개되는 방 코드와 비밀인 표식이 같은 원천에서 나오므로, 방 코드를 아는 것은 표식을 알아내는 데 도움이 되지 않으려면 원천이 예측할 수 없어야 합니다.

use std::collections::BTreeMap;

use warship_protocol::{RoomCode, SeatToken};

use crate::random::RandomSource;
use crate::room::Room;

/// 새 방의 코드에 쓰는 문자입니다. 헷갈리는 0, o, 1, l, i를 뺀 31자입니다.
const CODE_ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
/// 새 방의 코드의 길이입니다. 31의 5제곱은 약 2,860만 가지입니다.
const CODE_LEN: usize = 5;
/// 이 값 미만인 바이트만 글자를 고르는 데 씁니다. 256이 문자표의 크기로 나누어떨어지지 않으므로, 나머지가 고르게 나오는 가장 큰 범위입니다.
const EVEN_BYTES: usize = 256 / CODE_ALPHABET.len() * CODE_ALPHABET.len();

pub(crate) struct Mint {
    random: Box<dyn RandomSource>,
}

impl Mint {
    pub fn new(random: Box<dyn RandomSource>) -> Mint {
        Mint { random }
    }

    /// 새 자리의 표식입니다. 무작위 원천이 준 16바이트 그대로이므로, 이미 만든 표식과 겹칠 확률은 없는 것으로 봅니다.
    pub fn token(&mut self) -> SeatToken {
        let mut bytes = [0u8; 16];
        self.random.fill(&mut bytes);
        SeatToken(bytes)
    }

    /// 지금 없는 방의 새 코드입니다. 있는 방과 겹치면 다시 만듭니다.
    pub fn room_code(&mut self, rooms: &BTreeMap<RoomCode, Room>) -> RoomCode {
        loop {
            // 문자표가 방 코드의 문자만 담고 있으므로 해석에 실패하지 않습니다. 실패하면 다시 만듭니다.
            if let Ok(code) = RoomCode::parse(&self.code_text())
                && !rooms.contains_key(&code)
            {
                return code;
            }
        }
    }

    /// 문자표의 글자 `CODE_LEN`개입니다. 바이트를 문자표의 크기로 나눈 나머지로 글자를 고릅니다.
    /// 나머지가 고르지 않게 되는 큰 바이트(`EVEN_BYTES` 이상)는 버리고 새 바이트를 받습니다.
    fn code_text(&mut self) -> String {
        let mut text = String::with_capacity(CODE_LEN);
        while text.len() < CODE_LEN {
            let mut bytes = [0u8; CODE_LEN];
            self.random.fill(&mut bytes);
            let wanted = CODE_LEN - text.len();
            for byte in bytes
                .into_iter()
                .filter(|&byte| usize::from(byte) < EVEN_BYTES)
                .take(wanted)
            {
                text.push(char::from(
                    CODE_ALPHABET[usize::from(byte) % CODE_ALPHABET.len()],
                ));
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::random::PredictableRandom;

    /// 정해 둔 바이트를 차례로 주는 원천입니다. 바이트가 모자라면 테스트가 실패합니다.
    struct Script(VecDeque<u8>);

    impl Script {
        fn of(bytes: &[u8]) -> Box<Script> {
            Box::new(Script(bytes.iter().copied().collect()))
        }
    }

    impl RandomSource for Script {
        fn fill(&mut self, bytes: &mut [u8]) {
            for slot in bytes {
                *slot = self.0.pop_front().expect("the script ran out of bytes");
            }
        }
    }

    fn predictable(seed: u64) -> Mint {
        Mint::new(Box::new(PredictableRandom::new(seed)))
    }

    #[test]
    fn a_token_is_the_bytes_the_source_gave() {
        let given: Vec<u8> = (100..132).collect();
        let mut mint = Mint::new(Script::of(&given));

        assert_eq!(mint.token().0.to_vec(), given[..16]);
        assert_eq!(mint.token().0.to_vec(), given[16..]);
    }

    #[test]
    fn successive_tokens_differ() {
        let mut mint = predictable(1);
        let tokens: Vec<_> = (0..50).map(|_| mint.token().0).collect();
        let distinct: std::collections::BTreeSet<_> = tokens.iter().collect();
        assert_eq!(distinct.len(), tokens.len(), "no token twice");
    }

    #[test]
    fn a_code_picks_one_letter_per_byte_and_skips_bytes_that_would_favor_some_letters() {
        // 문자표는 a, b, c, ... 순서이고 마지막(30번째)이 9입니다. 255와 248과 249는 버리고, 31은 나머지가 0이므로 a가 됩니다.
        let mut mint = Mint::new(Script::of(&[255, 0, 1, 248, 30, 31, 2, 255, 249, 29]));

        let code = mint.room_code(&BTreeMap::new());

        assert_eq!(code.as_str(), "ab9ac");
    }

    #[test]
    fn a_new_code_has_five_unambiguous_characters() {
        let mut mint = predictable(1);
        for _ in 0..200 {
            let code = mint.room_code(&BTreeMap::new());
            assert_eq!(code.as_str().len(), 5);
            assert!(
                code.as_str()
                    .chars()
                    .all(|c| !"0o1li".contains(c) && RoomCode::allows(c)),
                "{code}"
            );
        }
    }

    #[test]
    fn successive_codes_differ() {
        let mut mint = predictable(1);
        let first = mint.room_code(&BTreeMap::new());
        let second = mint.room_code(&BTreeMap::new());
        assert_ne!(first, second);
    }
}
