//! 클라이언트와 서버가 주고받는 메시지입니다.

use serde::{Deserialize, Serialize};
use warship_rules::{Command, HullId, SkillId, Team, WeaponId};

use crate::seat_token::SeatToken;
use crate::snapshot::Snapshot;
use crate::welcome::Welcome;

/// 메시지의 형식이 바뀔 때마다 올리는 번호입니다. 서버는 다른 번호의 입장 요청을 거절합니다.
pub const PROTOCOL_VERSION: u32 = 6;

/// 클라이언트에서 서버로 가는 메시지입니다. `Join`은 첫째 변형이어야 합니다(`codec`이 변형 번호 0으로 알아봅니다).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClientMsg {
    /// 접속한 뒤 처음 보내는 메시지입니다. 모든 값은 플레이어가 입력한 그대로이고, 서버가 검사합니다.
    Join {
        version: u32,
        /// 방 코드입니다. 서버가 `RoomCode`로 해석하고, 그 방이 없으면 만듭니다. 없으면 서버가 새 방 코드를 만듭니다.
        room: Option<String>,
        /// 플레이어의 이름입니다. 서버가 `PlayerName`으로 해석합니다.
        name: String,
        /// 앞선 입장에서 받은 자리의 표식입니다. 그 방에 이 표식의 자리가 남아 있으면 그 자리의 배를 다시 조종합니다.
        token: Option<SeatToken>,
    },
    Input(InputMsg),
    Command(CommandMsg),
    /// 접속이 살아 있다는 표시입니다. 서버는 입장한 접속이 `ServerLimits::idle_timeout` 동안 아무 메시지도 보내지 않으면 닫습니다.
    Ping,
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

/// 한 번만 실행해야 하는 조작(구매, 판매, 선체 교체, 스킬)입니다. 모든 조건은 서버가 검사합니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandMsg {
    /// 클라이언트가 명령을 보낼 때마다 1씩 늘리는 순서 번호입니다. 서버는 이미 처리한 번호보다 크지 않은 명령을 버리고,
    /// 처리한 번호와 거절한 이유를 스냅샷의 `WireOwn`에 싣습니다.
    pub seq: u32,
    pub command: WireCommand,
}

/// 번호는 `Welcome`의 무기, 선체, 스킬 목록의 자리입니다. 목록에 없는 번호는 서버가 거절합니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireCommand {
    Buy { weapon: u16 },
    Sell { slot: u8 },
    ChangeHull { hull: u16 },
    Learn { skill: u16 },
    Cast { skill: u16 },
}

impl From<WireCommand> for Command {
    fn from(command: WireCommand) -> Command {
        match command {
            WireCommand::Buy { weapon } => Command::Buy(WeaponId(weapon)),
            WireCommand::Sell { slot } => Command::Sell { slot },
            WireCommand::ChangeHull { hull } => Command::ChangeHull(HullId(hull)),
            WireCommand::Learn { skill } => Command::Learn(SkillId(skill)),
            WireCommand::Cast { skill } => Command::Cast(SkillId(skill)),
        }
    }
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
    /// 방에 있는 사람의 명단입니다. 입장 안내 바로 다음에 보내고, 명단이 바뀔 때마다 방의 모든 접속에 다시 보냅니다.
    Roster(Vec<WireMember>),
    /// 같은 표식의 새 접속이 이 접속의 자리를 이어받았습니다. 서버는 접속을 닫지 않고, 클라이언트가 닫기를 기다립니다.
    SeatTakenOver,
}

/// 방에 있는 사람 한 명입니다. 빈자리를 채우는 배는 명단에 없습니다.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireMember {
    /// 이 사람이 조종하는 배의 번호입니다.
    pub ship: u32,
    pub name: String,
    /// 접속이 열려 있는지입니다. 접속이 끊겨서 자리만 유지 중이면 거짓입니다.
    pub connected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectReason {
    RoomFull {
        capacity: u32,
    },
    BadRoomCode,
    BadName,
    /// 서버가 방을 더 만들 수 없습니다.
    ServerFull,
    VersionMismatch {
        server: u32,
    },
}
