//! 클라이언트와 서버가 주고받는 메시지입니다.

use serde::{Deserialize, Serialize};
use warship_rules::Team;

use crate::snapshot::Snapshot;
use crate::welcome::Welcome;

/// 메시지의 형식이 바뀔 때마다 올리는 번호입니다. 서버는 다른 번호의 입장 요청을 거절합니다.
pub const PROTOCOL_VERSION: u32 = 1;

/// 클라이언트에서 서버로 가는 메시지입니다. `Join`은 첫째 변형이어야 합니다(`codec`이 변형 번호 0으로 알아봅니다).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClientMsg {
    /// 접속한 뒤 처음 보내는 메시지입니다. 모든 값은 플레이어가 입력한 그대로이고, 서버가 검사합니다.
    Join {
        version: u32,
        /// 방 코드입니다. 서버가 `RoomCode`로 해석하고, 그 방이 없으면 만듭니다.
        room: String,
    },
    Input(InputMsg),
    /// 플레이어가 스스로 방을 나갑니다.
    Leave,
}

/// 플레이어가 요청하는 조작입니다. 클라이언트가 서버에 보내는 게임 정보는 이 필드가 전부입니다.
/// 위치나 피해량 같은 필드를 넣어서는 안 됩니다. 값은 믿을 수 없는 입력이고, 서버가 검사합니다.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputMsg {
    /// 클라이언트가 입력을 보낼 때마다 1씩 늘리는 순서 번호입니다. 서버는 이미 받은 번호보다 크지 않은 입력을 버립니다.
    pub seq: u32,
    /// 배가 향할 지점입니다. 없으면 배가 멈춥니다.
    pub destination: Option<WireVec2>,
}

/// 수면 위의 좌표입니다. 클라이언트가 보낸 값은 서버가 검사하기 전이므로 NaN이나 무한대일 수 있습니다.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WireVec2 {
    pub x: f32,
    pub y: f32,
}

impl WireVec2 {
    pub const fn new(x: f32, y: f32) -> WireVec2 {
        WireVec2 { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireTeam {
    Red,
    Blue,
}

impl From<Team> for WireTeam {
    fn from(team: Team) -> WireTeam {
        if team == Team::RED {
            WireTeam::Red
        } else {
            WireTeam::Blue
        }
    }
}

/// 서버에서 클라이언트로 가는 메시지입니다.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ServerMsg {
    /// 입장을 받아들였다는 답입니다. 접속 중에 바뀌지 않는 정보를 담고 있으므로 접속마다 한 번만 보냅니다.
    Welcome(Box<Welcome>),
    /// 입장을 거절했다는 답입니다. 서버는 접속을 닫지 않고, 이유를 읽은 클라이언트가 닫기를 기다립니다.
    JoinRejected { reason: RejectReason },
    /// Tick마다 보내는, 받는 진영에 보이는 방의 상태입니다.
    Snapshot(Box<Snapshot>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectReason {
    RoomFull { capacity: u32 },
    BadRoomCode,
    VersionMismatch { server: u32 },
}
