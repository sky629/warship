//! 클라이언트와 서버가 합의해야 하는 모든 것입니다.
//!
//! - `messages`, `snapshot`, `welcome`: 주고받는 메시지의 정의. 게임 상태를 전송 형식으로 바꾸는 함수는 `Snapshot::from_sim` 하나이고,
//!   진영마다 그 진영에 보이는 것만 싣습니다. `Welcome::from_sim`은 접속 중에 바뀌지 않는 설정을 입장할 때 한 번 바꿉니다.
//! - `room_code`, `player_name`: 플레이어가 입력하는 방 코드와 이름의 검사. 서버와 클라이언트가 같은 함수를 씁니다.
//! - `seat_token`: 끊긴 접속이 자기 자리로 돌아올 때 내는 표식.
//! - `codec`: 메시지를 바이트로 바꾸는 함수와 되돌리는 함수. 전송 형식을 아는 파일은 이것 하나입니다.
//! - `transport`: 접속 방식 인터페이스 두 개. 서버와 클라이언트는 이 트레이트만 압니다.
//! - `loopback`: 같은 프로세스 안에서 메시지를 전달하는 접속 방식. 서버 테스트가 사용합니다.
//! - `limits`: 서버가 접속 하나와 메시지 하나에 허용하는 한계.
//!
//! 읽는 순서: `messages`, `snapshot`, `welcome`, `transport`, `codec`, `loopback`.

mod codec;
mod limits;
mod loopback;
mod messages;
mod player_name;
mod room_code;
mod seat_token;
mod snapshot;
mod transport;
mod welcome;

pub use codec::{DecodeError, decode_client, decode_server, encode_client, encode_server};
pub use limits::ServerLimits;
pub use loopback::{LoopbackClient, LoopbackConnector, LoopbackServer, loopback};
pub use messages::{
    ClientMsg, CommandMsg, InputMsg, PROTOCOL_VERSION, RejectReason, ServerMsg, WireCommand,
    WireMember, WireTeam, WireVec2,
};
pub use player_name::{NameError, PlayerName};
pub use room_code::{RoomCode, RoomCodeError};
pub use seat_token::SeatToken;
pub use snapshot::{
    Snapshot, WireCreep, WireItem, WireMount, WireOutcome, WireOwn, WireRefusal, WireShip,
    WireSkill, WireStructure, WireStructureKind, WireTarget,
};
pub use transport::{ClientEvent, ClientTransport, LinkId, PeerKey, ServerEvent, ServerTransport};
pub use welcome::{
    Welcome, WireCreepSpec, WireGear, WireHire, WireHull, WireIsland, WireRecipe, WireShot,
    WireSkillSpec, WireStructureSpec, WireSupply, WireTrade, WireWeapon,
};
