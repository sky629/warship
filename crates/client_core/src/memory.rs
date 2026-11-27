//! 기기가 기억하는 것입니다. 마지막으로 쓴 이름과 방 코드, 그리고 마지막으로 앉은 자리의 표식입니다.
//!
//! 클라이언트를 다시 실행하거나 웹 페이지를 새로 고친 뒤에도 같은 방에 들어가면 같은 배로 돌아가게 합니다.
//! 표식은 그 표식을 준 서버에만 냅니다. 표식을 아는 쪽은 그 자리를 가져갈 수 있고, 서버 주소는 링크의 쿼리로 바뀔 수 있기 때문입니다.
//! 이 모듈은 글자와 값 사이의 변환만 합니다. 글자를 어디에 저장하는지는 클라이언트(`storage`)가 정합니다.
//!
//! 저장하는 글자는 `열쇠=값`의 줄입니다. 저장된 글자는 사람이 고쳤거나 깨졌을 수 있으므로, 읽을 수 없는 줄은 버립니다.

use warship_protocol::{PlayerName, RoomCode, SeatToken};

/// 마지막으로 앉은 자리입니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RememberedSeat {
    /// 이 표식을 준 서버의 주소입니다.
    pub server: String,
    pub room: RoomCode,
    pub token: SeatToken,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Memory {
    /// 마지막으로 쓴 이름입니다.
    pub name: Option<PlayerName>,
    /// 마지막으로 들어간 방의 코드입니다.
    pub room: Option<RoomCode>,
    pub seat: Option<RememberedSeat>,
}

impl Memory {
    /// 저장된 글자를 읽습니다. 읽을 수 없는 줄과 모르는 열쇠는 버립니다.
    pub fn parse(text: &str) -> Memory {
        let mut memory = Memory::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "name" => memory.name = PlayerName::parse(value).ok(),
                "room" => memory.room = RoomCode::parse(value).ok(),
                "seat" => memory.seat = parse_seat(value),
                _ => {}
            }
        }
        memory
    }

    /// `parse`가 되돌릴 수 있는 글자입니다.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        if let Some(name) = &self.name {
            text += &format!("name={name}\n");
        }
        if let Some(room) = &self.room {
            text += &format!("room={room}\n");
        }
        if let Some(seat) = &self.seat {
            let token = seat.token.to_hex();
            text += &format!("seat={} {token} {}\n", seat.room, seat.server);
        }
        text
    }

    /// `server`의 `room`에 들어갈 때 낼 표식입니다. 마지막으로 앉은 자리가 그 서버의 그 방의 자리일 때만 있습니다.
    pub fn token_for(&self, server: &str, room: Option<&str>) -> Option<SeatToken> {
        let room = RoomCode::parse(room?).ok()?;
        let seat = self.seat.as_ref()?;
        (seat.server == server && seat.room == room).then_some(seat.token)
    }
}

fn parse_seat(value: &str) -> Option<RememberedSeat> {
    let mut parts = value.splitn(3, ' ');
    let (room, token, server) = (parts.next()?, parts.next()?, parts.next()?);
    Some(RememberedSeat {
        server: server.to_string(),
        room: RoomCode::parse(room).ok()?,
        token: SeatToken::from_hex(token)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "wss://home.test";

    fn remembered() -> Memory {
        Memory {
            name: PlayerName::parse("Kang Lee").ok(),
            room: RoomCode::parse("duel").ok(),
            seat: Some(RememberedSeat {
                server: HOME.to_string(),
                room: RoomCode::parse("duel").expect("a valid code"),
                token: SeatToken([0xab; 16]),
            }),
        }
    }

    #[test]
    fn a_memory_survives_its_text() {
        let memory = remembered();
        assert_eq!(Memory::parse(&memory.to_text()), memory);
        assert_eq!(
            Memory::parse(&Memory::default().to_text()),
            Memory::default()
        );
    }

    #[test]
    fn lines_that_cannot_be_read_are_dropped_and_the_rest_is_kept() {
        let text =
            "name=K@NG\nroom=duel\nseat=duel nothex wss://home.test\nvolume=3\nbroken line\n";
        let memory = Memory::parse(text);
        assert_eq!(memory.name, None);
        assert_eq!(memory.room, RoomCode::parse("duel").ok());
        assert_eq!(memory.seat, None);

        // 서버 주소가 없는 자리는 어느 서버의 자리인지 알 수 없으므로 버립니다.
        let token = SeatToken([0xab; 16]).to_hex();
        assert_eq!(Memory::parse(&format!("seat=duel {token}\n")).seat, None);
    }

    #[test]
    fn the_token_is_given_only_for_the_room_of_the_seat_on_its_server() {
        let memory = remembered();
        let token = Some(SeatToken([0xab; 16]));
        assert_eq!(memory.token_for(HOME, Some("duel")), token);
        assert_eq!(memory.token_for(HOME, Some("DUEL")), token);
        assert_eq!(memory.token_for(HOME, Some("other")), None);
        assert_eq!(
            memory.token_for(HOME, None),
            None,
            "a new room has no seat yet"
        );
    }

    #[test]
    fn the_token_is_never_given_to_another_server() {
        let memory = remembered();
        assert_eq!(memory.token_for("wss://other.test", Some("duel")), None);
        assert_eq!(memory.token_for("ws://home.test", Some("duel")), None);
    }
}
