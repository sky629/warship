//! 메시지와 바이트 사이의 변환입니다. 전송 형식(postcard)을 아는 곳은 이 파일 하나이고, 접속 방식 어댑터는 이 함수들을 호출합니다.
//!
//! postcard는 필드 이름을 기록하지 않고 선언 순서대로 값만 기록합니다. 정수는 가변 길이로 기록하므로 작은 값은 1바이트입니다.

use std::fmt;

use serde::Deserialize;

use crate::messages::{ClientMsg, PROTOCOL_VERSION, ServerMsg};

/// 받은 바이트가 메시지 하나로 해석되지 않습니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeError {
    reason: String,
    /// 입장 요청(`Join`)인데 `version`이 이 빌드의 `PROTOCOL_VERSION`과 다르면 그 `version`입니다.
    /// 그 나머지는 해석하지 않았습니다. 형식이 바뀐 이전 클라이언트의 입장 요청은 해석에 실패하므로, 이 값으로 이유를 알려 줍니다.
    foreign_join: Option<u32>,
}

impl DecodeError {
    fn other(reason: String) -> DecodeError {
        DecodeError {
            reason,
            foreign_join: None,
        }
    }

    /// 입장 요청인데 다른 `version`의 것이면 그 `version`입니다. 이때 요청의 나머지는 해석하지 않았으므로 믿을 값이 없습니다.
    pub fn foreign_join_version(&self) -> Option<u32> {
        self.foreign_join
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot decode message: {}", self.reason)
    }
}

pub fn encode_client(msg: &ClientMsg) -> Vec<u8> {
    postcard::to_allocvec(msg).expect(ENCODE_CANNOT_FAIL)
}

/// 클라이언트가 보낸 바이트를 메시지로 되돌립니다. 입장 요청은 변형 번호와 `version`을 먼저 읽고,
/// `version`이 다르면 나머지를 해석하지 않고 `foreign_join_version`이 있는 오류를 돌려줍니다.
/// 메시지의 형식이 바뀐 빌드의 입장 요청은 `version` 뒤의 모양이 다르므로, 먼저 읽지 않으면 이유 없이 해석에 실패하기 때문입니다.
pub fn decode_client(bytes: &[u8]) -> Result<ClientMsg, DecodeError> {
    if let Some(version) = foreign_join_version(bytes) {
        return Err(DecodeError {
            reason: format!(
                "a Join of protocol {version}, but this build speaks {PROTOCOL_VERSION}"
            ),
            foreign_join: Some(version),
        });
    }
    decode_exact(bytes)
}

pub fn encode_server(msg: &ServerMsg) -> Vec<u8> {
    postcard::to_allocvec(msg).expect(ENCODE_CANNOT_FAIL)
}

pub fn decode_server(bytes: &[u8]) -> Result<ServerMsg, DecodeError> {
    decode_exact(bytes)
}

/// `ClientMsg::Join`의 변형 번호입니다. 열거형의 변형은 선언 순서의 번호로 기록되고 `Join`이 첫째이므로 0입니다.
/// 이 약속이 지켜지는지는 이 파일의 시험이 확인합니다.
const JOIN_VARIANT: u32 = 0;

/// 입장 요청의 앞부분입니다. 변형 번호와 `version`까지만 읽습니다.
#[derive(Deserialize)]
struct JoinHead {
    variant: u32,
    version: u32,
}

/// 입장 요청이고 `version`이 `PROTOCOL_VERSION`과 다르면 그 `version`입니다. 입장 요청의 `version` 앞은 어느 빌드에서나 같은 모양입니다.
fn foreign_join_version(bytes: &[u8]) -> Option<u32> {
    let (head, _rest) = postcard::take_from_bytes::<JoinHead>(bytes).ok()?;
    (head.variant == JOIN_VARIANT && head.version != PROTOCOL_VERSION).then_some(head.version)
}

/// postcard가 메모리 안의 `Vec`에 기록할 때 실패하는 경우는 길이를 미리 알 수 없는 목록뿐입니다.
/// serde의 derive가 만드는 `Vec`과 `String`의 기록은 항상 길이를 먼저 알려 주므로 실패하지 않습니다.
const ENCODE_CANNOT_FAIL: &str =
    "postcard encoding into a Vec cannot fail for derived message types";

/// 메시지 하나를 읽은 뒤에 남는 바이트가 있으면 오류로 처리합니다. 다른 형식의 데이터를 메시지로 잘못 읽는 일을 막습니다.
fn decode_exact<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, DecodeError> {
    let (msg, rest) =
        postcard::take_from_bytes::<T>(bytes).map_err(|err| DecodeError::other(err.to_string()))?;
    if !rest.is_empty() {
        return Err(DecodeError::other(format!(
            "{} unexpected trailing bytes",
            rest.len()
        )));
    }
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::InputMsg;

    fn join(version: u32) -> ClientMsg {
        ClientMsg::Join {
            version,
            room: Some("test".to_string()),
            name: "KANG".to_string(),
            token: None,
        }
    }

    /// 버전 7을 적은 입장 요청입니다. 버전 뒤에 방 코드가 없어서, 지금의 `Join`으로는 해석되지 않습니다.
    const PROTOCOL_7_JOIN: [u8; 2] = [0, 7];

    #[test]
    fn a_join_starts_with_its_variant_number_and_its_version() {
        let bytes = encode_client(&join(300));
        let (head, _) = postcard::take_from_bytes::<JoinHead>(&bytes).unwrap();
        assert_eq!((head.variant, head.version), (JOIN_VARIANT, 300));
        assert_eq!(bytes[0], JOIN_VARIANT as u8);
    }

    #[test]
    fn a_join_of_another_version_is_told_apart_without_reading_its_body() {
        assert!(postcard::from_bytes::<ClientMsg>(&PROTOCOL_7_JOIN).is_err());

        let error = decode_client(&PROTOCOL_7_JOIN).unwrap_err();

        assert_eq!(error.foreign_join_version(), Some(7));
    }

    #[test]
    fn a_join_of_another_version_whose_body_would_read_is_refused_the_same_way() {
        let error = decode_client(&encode_client(&join(PROTOCOL_VERSION + 1))).unwrap_err();
        assert_eq!(error.foreign_join_version(), Some(PROTOCOL_VERSION + 1));
    }

    #[test]
    fn a_join_of_this_version_is_read_whole_and_a_broken_body_is_an_ordinary_error() {
        let bytes = encode_client(&join(PROTOCOL_VERSION));
        assert_eq!(decode_client(&bytes), Ok(join(PROTOCOL_VERSION)));

        let cut = &bytes[..bytes.len() - 2];
        assert_eq!(decode_client(cut).unwrap_err().foreign_join_version(), None);
    }

    #[test]
    fn only_a_join_can_be_of_another_version() {
        let input = encode_client(&ClientMsg::Input(InputMsg {
            seq: PROTOCOL_VERSION + 1,
            ..InputMsg::default()
        }));
        assert!(decode_client(&input).is_ok());
        assert_eq!(decode_client(&[]).unwrap_err().foreign_join_version(), None);
        assert_eq!(
            decode_client(&[0xFF, 0xFF])
                .unwrap_err()
                .foreign_join_version(),
            None
        );
    }
}
